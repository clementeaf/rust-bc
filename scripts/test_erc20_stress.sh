#!/bin/bash

# Stress Test para ERC-20

API_PORT=20000
BASE_URL="http://localhost:${API_PORT}/api/v1"
CONCURRENT_REQUESTS=50
TOTAL_REQUESTS=500

echo "🔥 STRESS TEST ERC-20"
echo "===================="
echo "Concurrent requests: ${CONCURRENT_REQUESTS}"
echo "Total requests: ${TOTAL_REQUESTS}"
echo ""

# Limpiar
pkill -9 -f "rust-bc.*${API_PORT}" 2>/dev/null || true
sleep 2
rm -f test_stress.db*

# Iniciar nodo
echo "🚀 Iniciando nodo..."
cargo run --release -- ${API_PORT} 20001 test_stress > /tmp/stress_node.log 2>&1 &
NODE_PID=$!

# Esperar servidor
echo "⏳ Esperando servidor..."
for i in {1..30}; do
    if curl -s "${BASE_URL}/health" > /dev/null 2>&1; then
        echo "✅ Servidor listo"
        break
    fi
    sleep 1
done
sleep 2

# Crear wallets
echo "📝 Creando wallets..."
WALLET1=$(curl -s -X POST "${BASE_URL}/wallets/create" | jq -r '.data.address')
WALLET2=$(curl -s -X POST "${BASE_URL}/wallets/create" | jq -r '.data.address')
echo "Wallet 1: ${WALLET1}"
echo "Wallet 2: ${WALLET2}"

# Minar bloque
curl -s -X POST "${BASE_URL}/mine" -H "Content-Type: application/json" -d "{\"miner_address\": \"${WALLET1}\"}" > /dev/null

# Desplegar contrato
echo ""
echo "📄 Desplegando contrato..."
CONTRACT=$(curl -s -X POST "${BASE_URL}/contracts" \
    -H "Content-Type: application/json" \
    -d "{
        \"owner\": \"${WALLET1}\",
        \"contract_type\": \"token\",
        \"name\": \"StressToken\",
        \"symbol\": \"STR\",
        \"total_supply\": 1000000000,
        \"decimals\": 18
    }" | jq -r '.data')

echo "Contrato: ${CONTRACT}"

# Mint inicial
echo "💰 Minting tokens..."
curl -s -X POST "${BASE_URL}/contracts/${CONTRACT}/execute" \
    -H "Content-Type: application/json" \
    -d "{
        \"function\": \"mint\",
        \"params\": {
            \"to\": \"${WALLET1}\",
            \"amount\": 1000000
        }
    }" > /dev/null

# Función para hacer transfer
do_transfer() {
    local wallet_from=$1
    local wallet_to=$2
    local amount=$3
    curl -s -X POST "${BASE_URL}/contracts/${CONTRACT}/execute" \
        -H "Content-Type: application/json" \
        -d "{
            \"function\": \"transfer\",
            \"params\": {
                \"caller\": \"${wallet_from}\",
                \"to\": \"${wallet_to}\",
                \"amount\": ${amount}
            }
        }" | jq -r '.success'
}

# Stress test: transfers concurrentes
echo ""
echo "🔥 Iniciando stress test..."
START_TIME=$(date +%s)

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
    
    # Ejecutar en background
    (do_transfer "${FROM}" "${TO}" 1) &
    
    # Limitar concurrencia
    if [ $((i % ${CONCURRENT_REQUESTS})) -eq 0 ]; then
        wait
    fi
done

wait

END_TIME=$(date +%s)
DURATION=$((END_TIME - START_TIME))

# Verificar balances finales
echo ""
echo "📊 Verificando balances finales..."
BALANCE1=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/balance/${WALLET1}" | jq -r '.data')
BALANCE2=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/balance/${WALLET2}" | jq -r '.data')
TOTAL=$((BALANCE1 + BALANCE2))

echo "Balance Wallet 1: ${BALANCE1}"
echo "Balance Wallet 2: ${BALANCE2}"
echo "Total: ${TOTAL} (debe ser 1000000)"

# Resultados
echo ""
echo "=========================================="
echo "📈 RESULTADOS STRESS TEST"
echo "=========================================="
echo "Tiempo total: ${DURATION} segundos"
echo "Requests: ${TOTAL_REQUESTS}"
echo "Throughput: ~$((TOTAL_REQUESTS / DURATION)) req/s"
echo "Balance total: ${TOTAL}"
if [ "${TOTAL}" -eq 1000000 ]; then
    echo "✅ Integridad de balances: OK"
else
    echo "❌ Integridad de balances: FALLO (pérdida de tokens)"
fi
echo ""

# Limpiar
pkill -9 -P $NODE_PID 2>/dev/null || true
pkill -9 -f "rust-bc.*${API_PORT}" 2>/dev/null || true

