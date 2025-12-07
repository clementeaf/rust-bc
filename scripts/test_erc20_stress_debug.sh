#!/bin/bash

# Test de Stress con Debug Detallado

API_PORT=20000
BASE_URL="http://localhost:${API_PORT}/api/v1"
TOTAL_REQUESTS=50

echo "🔍 STRESS TEST CON DEBUG"
echo "======================="

# Limpiar
pkill -9 -f "rust-bc.*${API_PORT}" 2>/dev/null || true
sleep 2
rm -f test_debug.db*

# Iniciar nodo
echo "🚀 Iniciando nodo..."
cargo run --release -- ${API_PORT} 20001 test_debug > /tmp/debug_node.log 2>&1 &
NODE_PID=$!

# Esperar servidor
for i in {1..30}; do
    if curl -s "${BASE_URL}/health" > /dev/null 2>&1; then
        echo "✅ Servidor listo"
        break
    fi
    sleep 1
done
sleep 2

# Crear wallets
WALLET1=$(curl -s -X POST "${BASE_URL}/wallets/create" | jq -r '.data.address')
WALLET2=$(curl -s -X POST "${BASE_URL}/wallets/create" | jq -r '.data.address')
echo "Wallet 1: ${WALLET1:0:20}..."
echo "Wallet 2: ${WALLET2:0:20}..."

# Minar bloque
curl -s -X POST "${BASE_URL}/mine" -H "Content-Type: application/json" -d "{\"miner_address\": \"${WALLET1}\"}" > /dev/null

# Desplegar contrato
CONTRACT=$(curl -s -X POST "${BASE_URL}/contracts" \
    -H "Content-Type: application/json" \
    -d "{
        \"owner\": \"${WALLET1}\",
        \"contract_type\": \"token\",
        \"name\": \"DebugToken\",
        \"symbol\": \"DBG\",
        \"total_supply\": 1000000,
        \"decimals\": 18
    }" | jq -r '.data')

echo "Contrato: ${CONTRACT:0:30}..."

# Mint inicial
curl -s -X POST "${BASE_URL}/contracts/${CONTRACT}/execute" \
    -H "Content-Type: application/json" \
    -d "{
        \"function\": \"mint\",
        \"params\": {
            \"to\": \"${WALLET1}\",
            \"amount\": 1000000
        }
    }" > /dev/null

# Test con debug detallado
SUCCESS=0
FAILED=0
RATE_LIMIT=0
INSUFFICIENT=0
OTHER=0

echo ""
echo "🔥 Ejecutando test con debug..."
for i in $(seq 1 ${TOTAL_REQUESTS}); do
    if [ $((i % 2)) -eq 0 ]; then
        FROM=${WALLET1}
        TO=${WALLET2}
    else
        FROM=${WALLET2}
        TO=${WALLET1}
    fi
    
    # Ejecutar y capturar respuesta completa
    RESPONSE=$(curl -s -w "\n%{http_code}" -X POST "${BASE_URL}/contracts/${CONTRACT}/execute" \
        -H "Content-Type: application/json" \
        -d "{
            \"function\": \"transfer\",
            \"params\": {
                \"caller\": \"${FROM}\",
                \"to\": \"${TO}\",
                \"amount\": 1
            }
        }")
    
    HTTP_CODE=$(echo "$RESPONSE" | tail -1)
    BODY=$(echo "$RESPONSE" | head -n -1)
    SUCCESS_FLAG=$(echo "$BODY" | jq -r '.success' 2>/dev/null || echo "false")
    ERROR_MSG=$(echo "$BODY" | jq -r '.error // .message // ""' 2>/dev/null || echo "")
    
    if [ "$SUCCESS_FLAG" = "true" ]; then
        SUCCESS=$((SUCCESS + 1))
    else
        FAILED=$((FAILED + 1))
        echo "❌ Fallo #${FAILED} (Request ${i}):"
        echo "   HTTP: ${HTTP_CODE}"
        echo "   Error: ${ERROR_MSG}"
        echo "   From: ${FROM:0:20}..."
        echo "   To: ${TO:0:20}..."
        
        # Categorizar error
        if echo "$ERROR_MSG" | grep -qi "rate limit"; then
            RATE_LIMIT=$((RATE_LIMIT + 1))
            echo "   Tipo: Rate Limiting"
        elif echo "$ERROR_MSG" | grep -qi "insufficient"; then
            INSUFFICIENT=$((INSUFFICIENT + 1))
            echo "   Tipo: Balance Insuficiente"
        else
            OTHER=$((OTHER + 1))
            echo "   Tipo: Otro"
        fi
        echo ""
    fi
    
    sleep 0.01
done

# Verificar balances
BALANCE1=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/balance/${WALLET1}" | jq -r '.data')
BALANCE2=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/balance/${WALLET2}" | jq -r '.data')
TOTAL=$((BALANCE1 + BALANCE2))

echo "=========================================="
echo "📊 RESULTADOS DEBUG"
echo "=========================================="
echo "Éxitos: ${SUCCESS}"
echo "Fallos: ${FAILED}"
echo "  - Rate Limiting: ${RATE_LIMIT}"
echo "  - Balance Insuficiente: ${INSUFFICIENT}"
echo "  - Otros: ${OTHER}"
echo "Balance total: ${TOTAL}"
echo ""

# Limpiar
pkill -9 -P $NODE_PID 2>/dev/null || true

