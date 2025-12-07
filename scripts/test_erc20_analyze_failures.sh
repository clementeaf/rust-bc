#!/bin/bash

# Análisis detallado de fallos

API_PORT=20000
BASE_URL="http://localhost:${API_PORT}/api/v1"

echo "🔍 ANÁLISIS DE FALLOS ERC-20"
echo "============================="

# Limpiar
pkill -9 -f "rust-bc.*${API_PORT}" 2>/dev/null || true
sleep 2
rm -f test_analyze.db*

# Iniciar nodo
echo "🚀 Iniciando nodo..."
cargo run --release -- ${API_PORT} 20001 test_analyze > /tmp/analyze_node.log 2>&1 &
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
        \"name\": \"AnalyzeToken\",
        \"symbol\": \"ANZ\",
        \"total_supply\": 1000000,
        \"decimals\": 18
    }" | jq -r '.data')

echo "Contrato: ${CONTRACT:0:30}..."

# Mint inicial solo a WALLET1
echo ""
echo "💰 Minting tokens a Wallet 1..."
MINT_RESPONSE=$(curl -s -X POST "${BASE_URL}/contracts/${CONTRACT}/execute" \
    -H "Content-Type: application/json" \
    -d "{
        \"function\": \"mint\",
        \"params\": {
            \"to\": \"${WALLET1}\",
            \"amount\": 1000000
        }
    }")

echo "Mint response: ${MINT_RESPONSE}"

# Verificar balances iniciales
BALANCE1=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/balance/${WALLET1}" | jq -r '.data')
BALANCE2=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/balance/${WALLET2}" | jq -r '.data')
echo "Balance Wallet 1: ${BALANCE1}"
echo "Balance Wallet 2: ${BALANCE2}"

# Test: Intentar transfer desde WALLET2 (sin balance)
echo ""
echo "🧪 Test 1: Transfer desde WALLET2 sin balance..."
RESPONSE1=$(curl -s -X POST "${BASE_URL}/contracts/${CONTRACT}/execute" \
    -H "Content-Type: application/json" \
    -d "{
        \"function\": \"transfer\",
        \"params\": {
            \"caller\": \"${WALLET2}\",
            \"to\": \"${WALLET1}\",
            \"amount\": 1
        }
    }")

echo "Response: ${RESPONSE1}"
SUCCESS1=$(echo "$RESPONSE1" | jq -r '.success')
MESSAGE1=$(echo "$RESPONSE1" | jq -r '.message // ""')
echo "Success: ${SUCCESS1}"
echo "Message: ${MESSAGE1}"

# Test: Transfer desde WALLET1 (con balance)
echo ""
echo "🧪 Test 2: Transfer desde WALLET1 con balance..."
RESPONSE2=$(curl -s -X POST "${BASE_URL}/contracts/${CONTRACT}/execute" \
    -H "Content-Type: application/json" \
    -d "{
        \"function\": \"transfer\",
        \"params\": {
            \"caller\": \"${WALLET1}\",
            \"to\": \"${WALLET2}\",
            \"amount\": 1000
        }
    }")

echo "Response: ${RESPONSE2}"
SUCCESS2=$(echo "$RESPONSE2" | jq -r '.success')
MESSAGE2=$(echo "$RESPONSE2" | jq -r '.message // ""')
echo "Success: ${SUCCESS2}"
echo "Message: ${MESSAGE2}"

# Verificar balances después
BALANCE1_AFTER=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/balance/${WALLET1}" | jq -r '.data')
BALANCE2_AFTER=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/balance/${WALLET2}" | jq -r '.data')
echo "Balance Wallet 1 después: ${BALANCE1_AFTER}"
echo "Balance Wallet 2 después: ${BALANCE2_AFTER}"

# Test: Transfer desde WALLET2 ahora (con balance)
echo ""
echo "🧪 Test 3: Transfer desde WALLET2 con balance..."
RESPONSE3=$(curl -s -X POST "${BASE_URL}/contracts/${CONTRACT}/execute" \
    -H "Content-Type: application/json" \
    -d "{
        \"function\": \"transfer\",
        \"params\": {
            \"caller\": \"${WALLET2}\",
            \"to\": \"${WALLET1}\",
            \"amount\": 100
        }
    }")

echo "Response: ${RESPONSE3}"
SUCCESS3=$(echo "$RESPONSE3" | jq -r '.success')
MESSAGE3=$(echo "$RESPONSE3" | jq -r '.message // ""')
echo "Success: ${SUCCESS3}"
echo "Message: ${MESSAGE3}"

# Test de rate limiting: múltiples requests rápidas
echo ""
echo "🧪 Test 4: Rate limiting (10 requests rápidas)..."
RATE_LIMIT_COUNT=0
for i in {1..12}; do
    RESPONSE=$(curl -s -X POST "${BASE_URL}/contracts/${CONTRACT}/execute" \
        -H "Content-Type: application/json" \
        -d "{
            \"function\": \"transfer\",
            \"params\": {
                \"caller\": \"${WALLET1}\",
                \"to\": \"${WALLET2}\",
                \"amount\": 1
            }
        }")
    
    SUCCESS=$(echo "$RESPONSE" | jq -r '.success')
    MESSAGE=$(echo "$RESPONSE" | jq -r '.message // ""')
    
    if [ "$SUCCESS" != "true" ]; then
        RATE_LIMIT_COUNT=$((RATE_LIMIT_COUNT + 1))
        echo "  Request ${i}: ❌ ${MESSAGE}"
    else
        echo "  Request ${i}: ✅"
    fi
    
    sleep 0.05
done

echo ""
echo "=========================================="
echo "📊 RESUMEN DE ANÁLISIS"
echo "=========================================="
echo "Test 1 (WALLET2 sin balance):"
echo "  Success: ${SUCCESS1}"
echo "  Error: ${MESSAGE1}"
echo ""
echo "Test 2 (WALLET1 con balance):"
echo "  Success: ${SUCCESS2}"
echo "  Error: ${MESSAGE2}"
echo ""
echo "Test 3 (WALLET2 con balance):"
echo "  Success: ${SUCCESS3}"
echo "  Error: ${MESSAGE3}"
echo ""
echo "Test 4 (Rate limiting):"
echo "  Fallos por rate limit: ${RATE_LIMIT_COUNT}/12"
echo ""

# Limpiar
pkill -9 -P $NODE_PID 2>/dev/null || true

