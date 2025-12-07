#!/bin/bash

# Stress Test Simplificado para ERC-20

API_PORT=20000
BASE_URL="http://localhost:${API_PORT}/api/v1"
TOTAL_REQUESTS=100

echo "🔥 STRESS TEST ERC-20 (Simplificado)"
echo "===================================="
echo "Total requests: ${TOTAL_REQUESTS}"
echo ""

# Limpiar procesos anteriores
pkill -9 -f "rust-bc.*${API_PORT}" 2>/dev/null || true
sleep 2

# Limpiar DBs
rm -f test_stress.db test_stress.db-shm test_stress.db-wal 2>/dev/null || true

# Iniciar nodo en background
echo "🚀 Iniciando nodo..."
cargo run --release -- ${API_PORT} 20001 test_stress > /tmp/stress_node.log 2>&1 &
NODE_PID=$!
echo "Nodo PID: ${NODE_PID}"

# Esperar servidor
echo "⏳ Esperando servidor..."
for i in {1..30}; do
    if curl -s "${BASE_URL}/health" > /dev/null 2>&1; then
        echo "✅ Servidor listo"
        break
    fi
    if [ $i -eq 30 ]; then
        echo "❌ Servidor no responde después de 30 segundos"
        pkill -9 -P $NODE_PID 2>/dev/null || true
        exit 1
    fi
    sleep 1
done
sleep 2

# Crear wallets
echo ""
echo "📝 Creando wallets..."
WALLET1=$(curl -s -X POST "${BASE_URL}/wallets/create" | jq -r '.data.address')
WALLET2=$(curl -s -X POST "${BASE_URL}/wallets/create" | jq -r '.data.address')

if [ -z "$WALLET1" ] || [ "$WALLET1" = "null" ]; then
    echo "❌ Error al crear wallet 1"
    pkill -9 -P $NODE_PID 2>/dev/null || true
    exit 1
fi

echo "Wallet 1: ${WALLET1:0:20}..."
echo "Wallet 2: ${WALLET2:0:20}..."

# Minar bloque
echo ""
echo "⛏️  Minando bloque inicial..."
curl -s -X POST "${BASE_URL}/mine" -H "Content-Type: application/json" -d "{\"miner_address\": \"${WALLET1}\"}" > /dev/null

# Desplegar contrato
echo ""
echo "📄 Desplegando contrato..."
CONTRACT_RESPONSE=$(curl -s -X POST "${BASE_URL}/contracts" \
    -H "Content-Type: application/json" \
    -d "{
        \"owner\": \"${WALLET1}\",
        \"contract_type\": \"token\",
        \"name\": \"StressToken\",
        \"symbol\": \"STR\",
        \"total_supply\": 1000000000,
        \"decimals\": 18
    }")

CONTRACT=$(echo $CONTRACT_RESPONSE | jq -r '.data')

if [ -z "$CONTRACT" ] || [ "$CONTRACT" = "null" ]; then
    echo "❌ Error al desplegar contrato"
    echo "Respuesta: ${CONTRACT_RESPONSE}"
    pkill -9 -P $NODE_PID 2>/dev/null || true
    exit 1
fi

echo "Contrato: ${CONTRACT:0:30}..."

# Mint inicial a ambos wallets para que ambos puedan transferir
echo ""
echo "💰 Minting tokens iniciales..."
MINT_RESULT1=$(curl -s -X POST "${BASE_URL}/contracts/${CONTRACT}/execute" \
    -H "Content-Type: application/json" \
    -d "{
        \"function\": \"mint\",
        \"params\": {
            \"to\": \"${WALLET1}\",
            \"amount\": 500000
        }
    }" | jq -r '.success')

MINT_RESULT2=$(curl -s -X POST "${BASE_URL}/contracts/${CONTRACT}/execute" \
    -H "Content-Type: application/json" \
    -d "{
        \"function\": \"mint\",
        \"params\": {
            \"to\": \"${WALLET2}\",
            \"amount\": 500000
        }
    }" | jq -r '.success')

if [ "$MINT_RESULT1" != "true" ] || [ "$MINT_RESULT2" != "true" ]; then
    echo "❌ Error en mint inicial"
    pkill -9 -P $NODE_PID 2>/dev/null || true
    exit 1
fi

echo "✅ Mint exitoso a ambos wallets"

# Verificar balances iniciales
BALANCE1_INIT=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/balance/${WALLET1}" | jq -r '.data')
BALANCE2_INIT=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/balance/${WALLET2}" | jq -r '.data')
echo "Balance inicial Wallet 1: ${BALANCE1_INIT}"
echo "Balance inicial Wallet 2: ${BALANCE2_INIT}"

# Stress test: transfers secuenciales
echo ""
echo "🔥 Iniciando stress test (${TOTAL_REQUESTS} transfers)..."
START_TIME=$(date +%s.%N)

SUCCESS=0
FAILED=0

for i in $(seq 1 ${TOTAL_REQUESTS}); do
    # Alternar entre wallet1 y wallet2
    if [ $((i % 2)) -eq 0 ]; then
        FROM=${WALLET1}
        TO=${WALLET2}
    else
        FROM=${WALLET2}
        TO=${WALLET1}
    fi
    
    # Ejecutar transfer (ambos wallets tienen balance ahora)
    RESULT=$(curl -s -X POST "${BASE_URL}/contracts/${CONTRACT}/execute" \
        -H "Content-Type: application/json" \
        -d "{
            \"function\": \"transfer\",
            \"params\": {
                \"caller\": \"${FROM}\",
                \"to\": \"${TO}\",
                \"amount\": 1
            }
        }" | jq -r '.success' 2>/dev/null || echo "false")
    
    if [ "$RESULT" = "true" ]; then
        SUCCESS=$((SUCCESS + 1))
    else
        FAILED=$((FAILED + 1))
        # Capturar mensaje de error para debugging
        ERROR_RESPONSE=$(curl -s -X POST "${BASE_URL}/contracts/${CONTRACT}/execute" \
            -H "Content-Type: application/json" \
            -d "{
                \"function\": \"transfer\",
                \"params\": {
                    \"caller\": \"${FROM}\",
                    \"to\": \"${TO}\",
                    \"amount\": 1
                }
            }")
        ERROR_MSG=$(echo "$ERROR_RESPONSE" | jq -r '.message // ""' 2>/dev/null || echo "")
        if [ -n "$ERROR_MSG" ] && [ "$ERROR_MSG" != "null" ] && [ "$ERROR_MSG" != "" ]; then
            echo "    Error en request ${i}: ${ERROR_MSG}"
        fi
    fi
    
    # Delay pequeño para no saturar (10ms entre requests)
    sleep 0.01
    
    # Mostrar progreso cada 10 requests
    if [ $((i % 10)) -eq 0 ]; then
        echo "  Progreso: ${i}/${TOTAL_REQUESTS} (Éxitos: ${SUCCESS}, Fallos: ${FAILED})"
    fi
done

END_TIME=$(date +%s.%N)
DURATION=$(echo "$END_TIME - $START_TIME" | bc)

# Verificar balances finales
echo ""
echo "📊 Verificando balances finales..."
BALANCE1_FINAL=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/balance/${WALLET1}" | jq -r '.data')
BALANCE2_FINAL=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/balance/${WALLET2}" | jq -r '.data')
TOTAL=$((BALANCE1_FINAL + BALANCE2_FINAL))

echo "Balance final Wallet 1: ${BALANCE1_FINAL}"
echo "Balance final Wallet 2: ${BALANCE2_FINAL}"
echo "Total: ${TOTAL} (debe ser 1000000 - igual que inicial)"

# Calcular throughput
THROUGHPUT=$(echo "scale=2; ${TOTAL_REQUESTS} / ${DURATION}" | bc)

# Resultados
echo ""
echo "=========================================="
echo "📈 RESULTADOS STRESS TEST"
echo "=========================================="
echo "Tiempo total: ${DURATION} segundos"
echo "Requests totales: ${TOTAL_REQUESTS}"
echo "Éxitos: ${SUCCESS}"
echo "Fallos: ${FAILED}"
echo "Throughput: ${THROUGHPUT} req/s"
echo "Balance total: ${TOTAL}"
echo ""

if [ "${TOTAL}" -eq 1000000 ]; then
    echo "✅ Integridad de balances: OK (sin pérdida de tokens)"
else
    echo "❌ Integridad de balances: FALLO (pérdida de tokens: $((1000000 - TOTAL)))"
fi

if [ "${FAILED}" -eq 0 ]; then
    echo "✅ Todas las operaciones exitosas"
else
    echo "⚠️  ${FAILED} operaciones fallaron"
fi

echo ""

# Limpiar
pkill -9 -P $NODE_PID 2>/dev/null || true
pkill -9 -f "rust-bc.*${API_PORT}" 2>/dev/null || true

echo "✅ Test completado"

