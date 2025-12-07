#!/bin/bash

# Test Completo para NFTs Básicos

API_PORT=20000
BASE_URL="http://localhost:${API_PORT}/api/v1"

echo "🎨 TEST COMPLETO: NFTs Básicos"
echo "==============================="

# Limpiar
pkill -9 -f "rust-bc.*${API_PORT}" 2>/dev/null || true
sleep 2
rm -f test_nft.db*

# Iniciar nodo
echo "🚀 Iniciando nodo..."
cargo run --release -- ${API_PORT} 20001 test_nft > /tmp/nft_node.log 2>&1 &
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
WALLET3=$(curl -s -X POST "${BASE_URL}/wallets/create" | jq -r '.data.address')
echo "Wallet 1: ${WALLET1:0:20}..."
echo "Wallet 2: ${WALLET2:0:20}..."
echo "Wallet 3: ${WALLET3:0:20}..."

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
        \"name\": \"TestNFT\",
        \"symbol\": \"TNFT\",
        \"total_supply\": null,
        \"decimals\": null
    }" | jq -r '.data')

echo "Contrato NFT: ${CONTRACT:0:30}..."

# Test 1: Mint NFT
echo ""
echo "🧪 Test 1: Mint NFT"
MINT_RESULT=$(curl -s -X POST "${BASE_URL}/contracts/${CONTRACT}/execute" \
    -H "Content-Type: application/json" \
    -d "{
        \"function\": \"mintNFT\",
        \"params\": {
            \"to\": \"${WALLET1}\",
            \"token_id\": 1,
            \"token_uri\": \"https://example.com/nft/1\"
        }
    }" | jq -r '.success')

if [ "$MINT_RESULT" = "true" ]; then
    echo "✅ Mint NFT exitoso"
else
    echo "❌ Error en mint NFT"
    pkill -9 -P $NODE_PID 2>/dev/null || true
    exit 1
fi

# Test 2: Verificar owner
echo ""
echo "🧪 Test 2: Verificar owner del NFT"
OWNER=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/nft/1/owner" | jq -r '.data')
if [ "$OWNER" = "$WALLET1" ]; then
    echo "✅ Owner correcto: ${OWNER:0:20}..."
else
    echo "❌ Owner incorrecto. Esperado: ${WALLET1:0:20}..., Obtenido: ${OWNER:0:20}..."
fi

# Test 3: Verificar URI
echo ""
echo "🧪 Test 3: Verificar URI del NFT"
URI=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/nft/1/uri" | jq -r '.data')
if [ "$URI" = "https://example.com/nft/1" ]; then
    echo "✅ URI correcta: ${URI}"
else
    echo "❌ URI incorrecta. Esperado: https://example.com/nft/1, Obtenido: ${URI}"
fi

# Test 4: Verificar balance
echo ""
echo "🧪 Test 4: Verificar balance de NFTs"
BALANCE=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/nft/balance/${WALLET1}" | jq -r '.data')
if [ "$BALANCE" = "1" ]; then
    echo "✅ Balance correcto: ${BALANCE}"
else
    echo "❌ Balance incorrecto. Esperado: 1, Obtenido: ${BALANCE}"
fi

# Test 5: Mint más NFTs
echo ""
echo "🧪 Test 5: Mint más NFTs"
for i in {2..5}; do
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
        echo "  ✅ Mint NFT ${i} exitoso"
    else
        echo "  ❌ Error en mint NFT ${i}"
    fi
done

# Verificar balance después de mint
BALANCE=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/nft/balance/${WALLET1}" | jq -r '.data')
echo "Balance Wallet 1 después de mint: ${BALANCE}"

# Test 6: Transfer NFT
echo ""
echo "🧪 Test 6: Transfer NFT"
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
    echo "✅ Transfer NFT exitoso"
    
    # Verificar nuevo owner
    OWNER=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/nft/1/owner" | jq -r '.data')
    if [ "$OWNER" = "$WALLET2" ]; then
        echo "✅ Nuevo owner correcto: ${OWNER:0:20}..."
    else
        echo "❌ Nuevo owner incorrecto"
    fi
    
    # Verificar balances
    BALANCE1=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/nft/balance/${WALLET1}" | jq -r '.data')
    BALANCE2=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/nft/balance/${WALLET2}" | jq -r '.data')
    echo "Balance Wallet 1: ${BALANCE1}"
    echo "Balance Wallet 2: ${BALANCE2}"
else
    echo "❌ Error en transfer NFT"
fi

# Test 7: Approve NFT
echo ""
echo "🧪 Test 7: Approve NFT"
APPROVE_RESULT=$(curl -s -X POST "${BASE_URL}/contracts/${CONTRACT}/execute" \
    -H "Content-Type: application/json" \
    -d "{
        \"function\": \"approveNFT\",
        \"params\": {
            \"caller\": \"${WALLET2}\",
            \"to\": \"${WALLET3}\",
            \"token_id\": 1
        }
    }" | jq -r '.success')

if [ "$APPROVE_RESULT" = "true" ]; then
    echo "✅ Approve NFT exitoso"
    
    # Verificar approved
    APPROVED=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/nft/1/approved" | jq -r '.data')
    if [ "$APPROVED" = "$WALLET3" ]; then
        echo "✅ Approved correcto: ${APPROVED:0:20}..."
    else
        echo "❌ Approved incorrecto"
    fi
else
    echo "❌ Error en approve NFT"
fi

# Test 8: TransferFrom NFT
echo ""
echo "🧪 Test 8: TransferFrom NFT (usando approval)"
TRANSFERFROM_RESULT=$(curl -s -X POST "${BASE_URL}/contracts/${CONTRACT}/execute" \
    -H "Content-Type: application/json" \
    -d "{
        \"function\": \"transferFromNFT\",
        \"params\": {
            \"caller\": \"${WALLET3}\",
            \"from\": \"${WALLET2}\",
            \"to\": \"${WALLET3}\",
            \"token_id\": 1
        }
    }" | jq -r '.success')

if [ "$TRANSFERFROM_RESULT" = "true" ]; then
    echo "✅ TransferFrom NFT exitoso"
    
    # Verificar nuevo owner
    OWNER=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/nft/1/owner" | jq -r '.data')
    if [ "$OWNER" = "$WALLET3" ]; then
        echo "✅ Nuevo owner correcto: ${OWNER:0:20}..."
    else
        echo "❌ Nuevo owner incorrecto"
    fi
    
    # Verificar que approval fue limpiado
    APPROVED=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/nft/1/approved" | jq -r '.data')
    if [ -z "$APPROVED" ] || [ "$APPROVED" = "" ]; then
        echo "✅ Approval limpiado correctamente"
    else
        echo "⚠️  Approval no fue limpiado: ${APPROVED}"
    fi
else
    echo "❌ Error en transferFrom NFT"
fi

# Test 9: Total Supply
echo ""
echo "🧪 Test 9: Total Supply de NFTs"
TOTAL_SUPPLY=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/nft/totalSupply" | jq -r '.data')
echo "Total Supply: ${TOTAL_SUPPLY}"
if [ "$TOTAL_SUPPLY" = "5" ]; then
    echo "✅ Total Supply correcto"
else
    echo "❌ Total Supply incorrecto. Esperado: 5, Obtenido: ${TOTAL_SUPPLY}"
fi

# Test 10: Intentar mint token duplicado (debe fallar)
echo ""
echo "🧪 Test 10: Intentar mint token duplicado (debe fallar)"
DUPLICATE_RESULT=$(curl -s -X POST "${BASE_URL}/contracts/${CONTRACT}/execute" \
    -H "Content-Type: application/json" \
    -d "{
        \"function\": \"mintNFT\",
        \"params\": {
            \"to\": \"${WALLET1}\",
            \"token_id\": 1,
            \"token_uri\": \"https://example.com/nft/1\"
        }
    }" | jq -r '.success')

if [ "$DUPLICATE_RESULT" = "false" ]; then
    echo "✅ Mint duplicado rechazado correctamente"
else
    echo "❌ Mint duplicado no fue rechazado"
fi

# Resumen final
echo ""
echo "=========================================="
echo "📊 RESUMEN"
echo "=========================================="
BALANCE1=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/nft/balance/${WALLET1}" | jq -r '.data')
BALANCE2=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/nft/balance/${WALLET2}" | jq -r '.data')
BALANCE3=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/nft/balance/${WALLET3}" | jq -r '.data')
TOTAL_SUPPLY=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/nft/totalSupply" | jq -r '.data')
TOTAL_BALANCES=$((BALANCE1 + BALANCE2 + BALANCE3))

echo "Balance Wallet 1: ${BALANCE1}"
echo "Balance Wallet 2: ${BALANCE2}"
echo "Balance Wallet 3: ${BALANCE3}"
echo "Total balances: ${TOTAL_BALANCES}"
echo "Total Supply: ${TOTAL_SUPPLY}"

if [ "$TOTAL_BALANCES" = "$TOTAL_SUPPLY" ]; then
    echo "✅ Integridad de NFTs: OK"
else
    echo "❌ Integridad de NFTs: ERROR"
fi

# Limpiar
pkill -9 -P $NODE_PID 2>/dev/null || true

echo ""
echo "✅ Test completado"

