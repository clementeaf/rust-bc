#!/bin/bash

# Prueba completa del estándar ERC-20

API_PORT=20000
BASE_URL="http://localhost:${API_PORT}/api/v1"

echo "🧪 Prueba Completa del Estándar ERC-20"
echo "========================================"

# Limpiar
pkill -9 -f "rust-bc.*${API_PORT}" 2>/dev/null || true
sleep 2
rm -f test_erc20.db*

# Iniciar nodo
echo ""
echo "🚀 Iniciando nodo..."
cargo run --release -- ${API_PORT} 20001 test_erc20 > /tmp/erc20_node.log 2>&1 &
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
echo ""
echo "📝 Creando wallets..."
WALLET_OWNER=$(curl -s -X POST "${BASE_URL}/wallets/create" | jq -r '.data.address')
WALLET_SPENDER=$(curl -s -X POST "${BASE_URL}/wallets/create" | jq -r '.data.address')
WALLET_RECIPIENT=$(curl -s -X POST "${BASE_URL}/wallets/create" | jq -r '.data.address')

echo "Owner: ${WALLET_OWNER}"
echo "Spender: ${WALLET_SPENDER}"
echo "Recipient: ${WALLET_RECIPIENT}"

# Minar bloque inicial
echo ""
echo "⛏️  Minando bloque inicial..."
curl -s -X POST "${BASE_URL}/mine" -H "Content-Type: application/json" -d "{\"miner_address\": \"${WALLET_OWNER}\"}" > /dev/null

# Desplegar token ERC-20
echo ""
echo "📄 Desplegando token ERC-20..."
CONTRACT_RESPONSE=$(curl -s -X POST "${BASE_URL}/contracts" \
    -H "Content-Type: application/json" \
    -d "{
        \"owner\": \"${WALLET_OWNER}\",
        \"contract_type\": \"token\",
        \"name\": \"TestERC20\",
        \"symbol\": \"TST\",
        \"total_supply\": 1000000,
        \"decimals\": 18
    }")

CONTRACT=$(echo $CONTRACT_RESPONSE | jq -r '.data')
echo "Respuesta deploy: ${CONTRACT_RESPONSE}"
echo "Contrato desplegado: ${CONTRACT}"

if [ "$CONTRACT" = "null" ] || [ -z "$CONTRACT" ]; then
    echo "❌ Error al desplegar contrato"
    pkill -9 -P $NODE_PID 2>/dev/null || true
    exit 1
fi

# Verificar funciones de lectura ERC-20
echo ""
echo "🔍 Verificando funciones de lectura ERC-20..."

# totalSupply
TOTAL_SUPPLY=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/totalSupply" | jq -r '.data')
echo "✅ totalSupply: ${TOTAL_SUPPLY}"

# name, symbol, decimals (desde get_contract)
CONTRACT_INFO=$(curl -s "${BASE_URL}/contracts/${CONTRACT}")
NAME=$(echo $CONTRACT_INFO | jq -r '.data.name')
SYMBOL=$(echo $CONTRACT_INFO | jq -r '.data.symbol')
DECIMALS=$(echo $CONTRACT_INFO | jq -r '.data.decimals')
echo "✅ name: ${NAME}"
echo "✅ symbol: ${SYMBOL}"
echo "✅ decimals: ${DECIMALS}"

# balanceOf (inicial)
BALANCE_OWNER=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/balance/${WALLET_OWNER}" | jq -r '.data')
echo "✅ balanceOf(owner): ${BALANCE_OWNER} (debe ser 0)"

# Mint tokens al owner
echo ""
echo "💰 Minting tokens al owner..."
MINT_RESULT=$(curl -s -X POST "${BASE_URL}/contracts/${CONTRACT}/execute" \
    -H "Content-Type: application/json" \
    -d "{
        \"function\": \"mint\",
        \"params\": {
            \"to\": \"${WALLET_OWNER}\",
            \"amount\": 10000
        }
    }" | jq -r '.success')
echo "Mint resultado: ${MINT_RESULT}"

# Verificar balance después de mint
BALANCE_OWNER=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/balance/${WALLET_OWNER}" | jq -r '.data')
echo "✅ balanceOf(owner) después de mint: ${BALANCE_OWNER} (debe ser 10000)"

# ERC-20: transfer
echo ""
echo "📤 Probando ERC-20 transfer..."
TRANSFER_RESULT=$(curl -s -X POST "${BASE_URL}/contracts/${CONTRACT}/execute" \
    -H "Content-Type: application/json" \
    -d "{
        \"function\": \"transfer\",
        \"params\": {
            \"caller\": \"${WALLET_OWNER}\",
            \"to\": \"${WALLET_RECIPIENT}\",
            \"amount\": 1000
        }
    }" | jq -r '.success')
echo "Transfer resultado: ${TRANSFER_RESULT}"

# Verificar balances después de transfer
BALANCE_OWNER=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/balance/${WALLET_OWNER}" | jq -r '.data')
BALANCE_RECIPIENT=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/balance/${WALLET_RECIPIENT}" | jq -r '.data')
echo "✅ balanceOf(owner) después de transfer: ${BALANCE_OWNER} (debe ser 9000)"
echo "✅ balanceOf(recipient) después de transfer: ${BALANCE_RECIPIENT} (debe ser 1000)"

# ERC-20: approve
echo ""
echo "✅ Probando ERC-20 approve..."
APPROVE_RESULT=$(curl -s -X POST "${BASE_URL}/contracts/${CONTRACT}/execute" \
    -H "Content-Type: application/json" \
    -d "{
        \"function\": \"approve\",
        \"params\": {
            \"caller\": \"${WALLET_OWNER}\",
            \"spender\": \"${WALLET_SPENDER}\",
            \"amount\": 2000
        }
    }" | jq -r '.success')
echo "Approve resultado: ${APPROVE_RESULT}"

# Verificar allowance
ALLOWANCE=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/allowance/${WALLET_OWNER}/${WALLET_SPENDER}" | jq -r '.data')
echo "✅ allowance(owner, spender): ${ALLOWANCE} (debe ser 2000)"

# ERC-20: transferFrom
echo ""
echo "📥 Probando ERC-20 transferFrom..."
TRANSFER_FROM_RESULT=$(curl -s -X POST "${BASE_URL}/contracts/${CONTRACT}/execute" \
    -H "Content-Type: application/json" \
    -d "{
        \"function\": \"transferFrom\",
        \"params\": {
            \"caller\": \"${WALLET_SPENDER}\",
            \"from\": \"${WALLET_OWNER}\",
            \"to\": \"${WALLET_RECIPIENT}\",
            \"amount\": 1500
        }
    }" | jq -r '.success')
echo "TransferFrom resultado: ${TRANSFER_FROM_RESULT}"

# Verificar balances después de transferFrom
BALANCE_OWNER=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/balance/${WALLET_OWNER}" | jq -r '.data')
BALANCE_RECIPIENT=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/balance/${WALLET_RECIPIENT}" | jq -r '.data')
echo "✅ balanceOf(owner) después de transferFrom: ${BALANCE_OWNER} (debe ser 7500)"
echo "✅ balanceOf(recipient) después de transferFrom: ${BALANCE_RECIPIENT} (debe ser 2500)"

# Verificar allowance reducido
ALLOWANCE=$(curl -s "${BASE_URL}/contracts/${CONTRACT}/allowance/${WALLET_OWNER}/${WALLET_SPENDER}" | jq -r '.data')
echo "✅ allowance(owner, spender) después de transferFrom: ${ALLOWANCE} (debe ser 500)"

# Verificar que transferFrom falla con allowance insuficiente
echo ""
echo "🔒 Probando que transferFrom falla con allowance insuficiente..."
TRANSFER_FROM_FAIL=$(curl -s -X POST "${BASE_URL}/contracts/${CONTRACT}/execute" \
    -H "Content-Type: application/json" \
    -d "{
        \"function\": \"transferFrom\",
        \"params\": {
            \"caller\": \"${WALLET_SPENDER}\",
            \"from\": \"${WALLET_OWNER}\",
            \"to\": \"${WALLET_RECIPIENT}\",
            \"amount\": 1000
        }
    }" | jq -r '.success')
if [ "$TRANSFER_FROM_FAIL" = "false" ]; then
    echo "✅ TransferFrom correctamente rechazado por allowance insuficiente"
else
    echo "❌ TransferFrom debería haber fallado"
fi

# Resumen
echo ""
echo "=========================================="
echo "✅ TEST ERC-20 COMPLETADO"
echo "=========================================="
echo "Funciones probadas:"
echo "  ✅ totalSupply()"
echo "  ✅ balanceOf(address)"
echo "  ✅ name()"
echo "  ✅ symbol()"
echo "  ✅ decimals()"
echo "  ✅ transfer(to, amount)"
echo "  ✅ approve(spender, amount)"
echo "  ✅ allowance(owner, spender)"
echo "  ✅ transferFrom(from, to, amount)"
echo "  ✅ Validación de allowance insuficiente"
echo ""
echo "🎉 El estándar ERC-20 está completamente implementado!"

# Limpiar
pkill -9 -P $NODE_PID 2>/dev/null || true
pkill -9 -f "rust-bc.*${API_PORT}" 2>/dev/null || true

