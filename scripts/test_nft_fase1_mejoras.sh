#!/bin/bash

# Test Completo para NFTs Fase 1 - Mejoras

API_PORT=20000
BASE_URL="http://localhost:${API_PORT}/api/v1"

echo "🎨 TEST: NFTs Fase 1 - Mejoras"
echo "==============================="

# Limpiar
pkill -9 -f "rust-bc.*${API_PORT}" 2>/dev/null || true
sleep 2
rm -f test_nft_fase1.db*

# Iniciar nodo
echo "🚀 Iniciando nodo..."
cargo run --release -- ${API_PORT} 20001 test_nft_fase1 > /tmp/nft_fase1_node.log 2>&1 &
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

# Desplegar contrato NFT
echo ""
echo "📄 Desplegando contrato NFT..."
CONTRACT=$(curl -s -X POST "${BASE_URL}/contracts" \
    -H "Content-Type: application/json" \
    -d "{
        \"owner\": \"${WALLET1}\",
        \"contract_type\": \"nft\",
        \"name\": \"TestNFTFase1\",
        \"symbol\": \"TNF1\",
        \"total_supply\": null,
        \"decimals\": null
    }" | jq -r '.data')

echo "Contrato NFT: ${CONTRACT:0:30}..."

# Test 1: Mint múltiples NFTs
echo ""
echo "🧪 Test 1: Mint múltiples NFTs (10 tokens)..."
MINTED_COUNT=0
for i in {1..10}; do
    MINT_RESULT=$(curl -s -X POST "${BASE_URL}/contracts/${CONTRACT}/execute" \
        -H "Content-Type: application/json" \
        -d "{
            \"function\": \"mintNFT\",
            \"params\": {
                \"to\": \"${WALLET1}\",
                \"token_id\": ${i},
                \"token_uri\": \"https://example.com/nft/${i}\"
            }
        }" | jq -r '.success')
    
    if [ "$MINT_RESULT" = "true" ]; then
        MINTED_COUNT=$((MINTED_COUNT + 1))
    fi
done

echo "✅ Minteados: ${MINTED_COUNT}/10"

# Test 2: Enumeración - tokensOfOwner
echo ""
echo "🧪 Test 2: Enumeración - tokensOfOwner"
TOKENS=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/nft/tokens/${WALLET1}" | jq -r '.data[]' 2>/dev/null)
TOKEN_COUNT=$(echo "$TOKENS" | wc -l | tr -d ' ')

if [ "$TOKEN_COUNT" = "10" ]; then
    echo "✅ tokensOfOwner correcto: ${TOKEN_COUNT} tokens"
    echo "   Tokens: $(echo "$TOKENS" | tr '\n' ' ')"
else
    echo "❌ tokensOfOwner incorrecto. Esperado: 10, Obtenido: ${TOKEN_COUNT}"
fi

# Test 3: Enumeración - tokenByIndex
echo ""
echo "🧪 Test 3: Enumeración - tokenByIndex"
TOKEN_0=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/nft/index/0" | jq -r '.data')
TOKEN_5=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/nft/index/5" | jq -r '.data')
TOKEN_9=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/nft/index/9" | jq -r '.data')

if [ -n "$TOKEN_0" ] && [ -n "$TOKEN_5" ] && [ -n "$TOKEN_9" ]; then
    echo "✅ tokenByIndex correcto:"
    echo "   Index 0: ${TOKEN_0}"
    echo "   Index 5: ${TOKEN_5}"
    echo "   Index 9: ${TOKEN_9}"
else
    echo "❌ tokenByIndex incorrecto"
fi

# Test 4: Metadata On-Chain - Set
echo ""
echo "🧪 Test 4: Metadata On-Chain - Set"
METADATA_SET=$(curl -s -X POST "${BASE_URL}/contracts/${CONTRACT}/nft/1/metadata" \
    -H "Content-Type: application/json" \
    -d "{
        \"metadata\": {
            \"name\": \"Test NFT #1\",
            \"description\": \"This is a test NFT with on-chain metadata\",
            \"image\": \"https://example.com/images/nft1.png\",
            \"external_url\": \"https://example.com/nft/1\",
            \"attributes\": [
                {\"trait_type\": \"Color\", \"value\": \"Blue\"},
                {\"trait_type\": \"Rarity\", \"value\": \"Common\"}
            ]
        }
    }" | jq -r '.success')

if [ "$METADATA_SET" = "true" ]; then
    echo "✅ Metadata set exitoso"
else
    echo "❌ Error al setear metadata"
fi

# Test 5: Metadata On-Chain - Get
echo ""
echo "🧪 Test 5: Metadata On-Chain - Get"
METADATA_NAME=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/nft/1/metadata" | jq -r '.data.name')
METADATA_DESC=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/nft/1/metadata" | jq -r '.data.description')
METADATA_ATTR_COUNT=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/nft/1/metadata" | jq -r '.data.attributes | length')

if [ "$METADATA_NAME" = "Test NFT #1" ]; then
    echo "✅ Metadata get exitoso:"
    echo "   Name: ${METADATA_NAME}"
    echo "   Description: ${METADATA_DESC:0:50}..."
    echo "   Attributes: ${METADATA_ATTR_COUNT}"
else
    echo "❌ Metadata incorrecta"
fi

# Test 6: Transfer y verificar enumeración actualizada
echo ""
echo "🧪 Test 6: Transfer y verificar enumeración actualizada"
TRANSFER_RESULT=$(curl -s -X POST "${BASE_URL}/contracts/${CONTRACT}/execute" \
    -H "Content-Type: application/json" \
    -d "{
        \"function\": \"transferNFT\",
        \"params\": {
            \"caller\": \"${WALLET1}\",
            \"from\": \"${WALLET1}\",
            \"to\": \"${WALLET2}\",
            \"token_id\": 1
        }
    }" | jq -r '.success')

if [ "$TRANSFER_RESULT" = "true" ]; then
    echo "✅ Transfer exitoso"
    
    # Verificar tokens de WALLET1 (debe tener 9 ahora)
    TOKENS_W1=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/nft/tokens/${WALLET1}" | jq -r '.data | length')
    TOKENS_W2=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/nft/tokens/${WALLET2}" | jq -r '.data | length')
    
    if [ "$TOKENS_W1" = "9" ] && [ "$TOKENS_W2" = "1" ]; then
        echo "✅ Enumeración actualizada correctamente:"
        echo "   Wallet 1: ${TOKENS_W1} tokens"
        echo "   Wallet 2: ${TOKENS_W2} tokens"
    else
        echo "❌ Enumeración no actualizada. W1: ${TOKENS_W1}, W2: ${TOKENS_W2}"
    fi
else
    echo "❌ Error en transfer"
fi

# Test 7: Burn NFT
echo ""
echo "🧪 Test 7: Burn NFT"
BURN_RESULT=$(curl -s -X POST "${BASE_URL}/contracts/${CONTRACT}/execute" \
    -H "Content-Type: application/json" \
    -d "{
        \"function\": \"burnNFT\",
        \"params\": {
            \"caller\": \"${WALLET2}\",
            \"owner\": \"${WALLET2}\",
            \"token_id\": 1
        }
    }" | jq -r '.success')

if [ "$BURN_RESULT" = "true" ]; then
    echo "✅ Burn exitoso"
    
    # Verificar que el token ya no existe
    OWNER_AFTER_BURN=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/nft/1/owner" | jq -r '.data' 2>/dev/null)
    if [ -z "$OWNER_AFTER_BURN" ] || [ "$OWNER_AFTER_BURN" = "null" ]; then
        echo "✅ Token eliminado correctamente"
    else
        echo "⚠️  Token aún existe: ${OWNER_AFTER_BURN}"
    fi
    
    # Verificar total supply
    TOTAL_SUPPLY=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/nft/totalSupply" | jq -r '.data')
    if [ "$TOTAL_SUPPLY" = "9" ]; then
        echo "✅ Total supply actualizado: ${TOTAL_SUPPLY}"
    else
        echo "❌ Total supply incorrecto. Esperado: 9, Obtenido: ${TOTAL_SUPPLY}"
    fi
    
    # Verificar enumeración después de burn
    TOKENS_W2_AFTER=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/nft/tokens/${WALLET2}" | jq -r '.data | length')
    if [ "$TOKENS_W2_AFTER" = "0" ]; then
        echo "✅ Enumeración actualizada después de burn: Wallet 2 tiene 0 tokens"
    else
        echo "❌ Enumeración no actualizada. Wallet 2 tiene: ${TOKENS_W2_AFTER}"
    fi
else
    echo "❌ Error en burn"
fi

# Test 8: Verificar integridad de índices
echo ""
echo "🧪 Test 8: Verificar integridad de índices"
TOTAL_SUPPLY=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/nft/totalSupply" | jq -r '.data')
TOKENS_W1=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/nft/tokens/${WALLET1}" | jq -r '.data | length')
TOKENS_W2=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/nft/tokens/${WALLET2}" | jq -r '.data | length')
TOTAL_BY_ENUM=$((TOKENS_W1 + TOKENS_W2))

if [ "$TOTAL_SUPPLY" = "$TOTAL_BY_ENUM" ]; then
    echo "✅ Integridad de índices: OK"
    echo "   Total Supply: ${TOTAL_SUPPLY}"
    echo "   Total por enumeración: ${TOTAL_BY_ENUM}"
else
    echo "❌ Integridad de índices: ERROR"
    echo "   Total Supply: ${TOTAL_SUPPLY}"
    echo "   Total por enumeración: ${TOTAL_BY_ENUM}"
fi

# Test 9: Performance - tokensOfOwner con muchos tokens
echo ""
echo "🧪 Test 9: Performance - tokensOfOwner (debe ser rápido O(1))"
START_TIME=$(date +%s%N)
TOKENS_W1=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/nft/tokens/${WALLET1}" | jq -r '.data | length')
END_TIME=$(date +%s%N)
DURATION=$(( (END_TIME - START_TIME) / 1000000 ))

echo "✅ tokensOfOwner ejecutado en ${DURATION}ms"
if [ "$DURATION" -lt 100 ]; then
    echo "   ✅ Performance excelente (< 100ms)"
elif [ "$DURATION" -lt 500 ]; then
    echo "   ✅ Performance buena (< 500ms)"
else
    echo "   ⚠️  Performance puede mejorar"
fi

# Resumen final
echo ""
echo "=========================================="
echo "📊 RESUMEN FINAL"
echo "=========================================="
TOTAL_SUPPLY=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/nft/totalSupply" | jq -r '.data')
TOKENS_W1=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/nft/tokens/${WALLET1}" | jq -r '.data | length')
TOKENS_W2=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/nft/tokens/${WALLET2}" | jq -r '.data | length')
BALANCE_W1=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/nft/balance/${WALLET1}" | jq -r '.data')
BALANCE_W2=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/nft/balance/${WALLET2}" | jq -r '.data')

echo "Total Supply: ${TOTAL_SUPPLY}"
echo "Wallet 1 - Tokens (enumeración): ${TOKENS_W1}"
echo "Wallet 1 - Balance: ${BALANCE_W1}"
echo "Wallet 2 - Tokens (enumeración): ${TOKENS_W2}"
echo "Wallet 2 - Balance: ${BALANCE_W2}"

TOTAL_BY_ENUM=$((TOKENS_W1 + TOKENS_W2))
if [ "$TOTAL_SUPPLY" = "$TOTAL_BY_ENUM" ] && [ "$TOTAL_SUPPLY" = "$((BALANCE_W1 + BALANCE_W2))" ]; then
    echo ""
    echo "✅ Integridad completa: OK"
    echo "   - Total Supply = Enumeración = Balances"
else
    echo ""
    echo "❌ Integridad: ERROR"
fi

# Limpiar
pkill -9 -P $NODE_PID 2>/dev/null || true

echo ""
echo "✅ Test completado"

