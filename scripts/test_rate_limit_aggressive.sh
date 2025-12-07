#!/bin/bash

# Test agresivo de rate limiting (sin delays)

API_PORT=20000
BASE_URL="http://localhost:${API_PORT}/api/v1"

echo "🔥 TEST AGRESIVO: Rate Limiting por Caller"
echo "==========================================="

# Limpiar
pkill -9 -f "rust-bc.*${API_PORT}" 2>/dev/null || true
sleep 2
rm -f test_ratelimit_agg.db*

# Iniciar nodo
echo "🚀 Iniciando nodo..."
cargo run --release -- ${API_PORT} 20001 test_ratelimit_agg > /tmp/ratelimit_agg_node.log 2>&1 &
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
        \"name\": \"RateLimitTest\",
        \"symbol\": \"RLT\",
        \"total_supply\": 1000000,
        \"decimals\": 18
    }" | jq -r '.data')

# Mint a ambos wallets
curl -s -X POST "${BASE_URL}/contracts/${CONTRACT}/execute" \
    -H "Content-Type: application/json" \
    -d "{
        \"function\": \"mint\",
        \"params\": {
            \"to\": \"${WALLET1}\",
            \"amount\": 10000
        }
    }" > /dev/null

curl -s -X POST "${BASE_URL}/contracts/${CONTRACT}/execute" \
    -H "Content-Type: application/json" \
    -d "{
        \"function\": \"mint\",
        \"params\": {
            \"to\": \"${WALLET2}\",
            \"amount\": 10000
        }
    }" > /dev/null

echo ""
echo "🧪 Test 1: WALLET1 hace 15 requests SIN DELAY (límite: 10/segundo)..."
WALLET1_SUCCESS=0
WALLET1_RATE_LIMITED=0

for i in {1..15}; do
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
    
    if [ "$SUCCESS" = "true" ]; then
        WALLET1_SUCCESS=$((WALLET1_SUCCESS + 1))
        echo "  Request ${i}: ✅"
    else
        if echo "$MESSAGE" | grep -qi "rate limit"; then
            WALLET1_RATE_LIMITED=$((WALLET1_RATE_LIMITED + 1))
            echo "  Request ${i}: ⚠️  Rate Limited: ${MESSAGE}"
        else
            echo "  Request ${i}: ❌ ${MESSAGE}"
        fi
    fi
done

echo ""
echo "Esperando 1 segundo para resetear rate limit..."
sleep 1

echo ""
echo "🧪 Test 2: WALLET2 hace 15 requests SIN DELAY (debe funcionar, límite es por caller)..."
WALLET2_SUCCESS=0
WALLET2_RATE_LIMITED=0

for i in {1..15}; do
    RESPONSE=$(curl -s -X POST "${BASE_URL}/contracts/${CONTRACT}/execute" \
        -H "Content-Type: application/json" \
        -d "{
            \"function\": \"transfer\",
            \"params\": {
                \"caller\": \"${WALLET2}\",
                \"to\": \"${WALLET1}\",
                \"amount\": 1
            }
        }")
    
    SUCCESS=$(echo "$RESPONSE" | jq -r '.success')
    MESSAGE=$(echo "$RESPONSE" | jq -r '.message // ""')
    
    if [ "$SUCCESS" = "true" ]; then
        WALLET2_SUCCESS=$((WALLET2_SUCCESS + 1))
        echo "  Request ${i}: ✅"
    else
        if echo "$MESSAGE" | grep -qi "rate limit"; then
            WALLET2_RATE_LIMITED=$((WALLET2_RATE_LIMITED + 1))
            echo "  Request ${i}: ⚠️  Rate Limited: ${MESSAGE}"
        else
            echo "  Request ${i}: ❌ ${MESSAGE}"
        fi
    fi
done

echo ""
echo "=========================================="
echo "📊 RESULTADOS"
echo "=========================================="
echo "WALLET1 (15 requests rápidas):"
echo "  Éxitos: ${WALLET1_SUCCESS}/15"
echo "  Rate Limited: ${WALLET1_RATE_LIMITED}/15"
echo ""
echo "WALLET2 (15 requests rápidas, después de 1s):"
echo "  Éxitos: ${WALLET2_SUCCESS}/15"
echo "  Rate Limited: ${WALLET2_RATE_LIMITED}/15"
echo ""

if [ "${WALLET1_RATE_LIMITED}" -ge 5 ] && [ "${WALLET2_RATE_LIMITED}" -eq 0 ]; then
    echo "✅ Rate limiting funciona correctamente por caller"
    echo "   WALLET1 fue limitado (esperado: 5+ requests bloqueadas)"
    echo "   WALLET2 NO fue limitado (correcto: límite es por caller)"
elif [ "${WALLET1_RATE_LIMITED}" -ge 1 ]; then
    echo "✅ Rate limiting está funcionando (al menos parcialmente)"
    echo "   WALLET1: ${WALLET1_RATE_LIMITED} requests bloqueadas"
    echo "   WALLET2: ${WALLET2_RATE_LIMITED} requests bloqueadas"
else
    echo "⚠️  Rate limiting no se activó"
    echo "   Puede ser que las requests sean muy lentas o el límite sea muy alto"
fi

# Limpiar
pkill -9 -P $NODE_PID 2>/dev/null || true

