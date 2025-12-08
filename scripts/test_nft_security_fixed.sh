#!/bin/bash

# Test de Seguridad - NFTs (Versión Corregida)
set -e

PORT=20000
BASE_URL="http://localhost:${PORT}"

echo "🔒 Test de Seguridad - NFTs"
echo "============================"
echo ""

# Limpiar
killall rust-bc 2>/dev/null || true
pkill -f rust-bc 2>/dev/null || true
rm -f test_nft_security.db* 2>/dev/null || true
sleep 1

# Compilar
echo "📦 Compilando..."
cargo build --release > /dev/null 2>&1 || { echo "❌ Error al compilar"; exit 1; }

# Iniciar servidor
echo "📡 Iniciando servidor en puerto ${PORT}..."
./target/release/rust-bc ${PORT} $((PORT + 1000)) test_nft_security > /tmp/rust-bc-test.log 2>&1 &
SERVER_PID=$!

# Esperar servidor
echo "  Esperando servidor..."
for i in {1..15}; do
    sleep 1
    if curl -s --max-time 2 "${BASE_URL}/api/v1/health" > /dev/null 2>&1; then
        echo "✅ Servidor iniciado (intento $i)"
        break
    fi
    if [ $i -eq 15 ]; then
        echo "❌ Servidor no responde"
        tail -20 /tmp/rust-bc-test.log
        kill $SERVER_PID 2>/dev/null || true
        exit 1
    fi
done

# Limpieza al salir
trap "kill $SERVER_PID 2>/dev/null; killall rust-bc 2>/dev/null; rm -f test_nft_security.db* 2>/dev/null" EXIT

# Helper
api_call() {
    curl -s --max-time 5 -X "$1" "${BASE_URL}$2" \
        ${3:+-H "Content-Type: application/json" -d "$3"}
}

# Crear wallet
echo ""
echo "📝 Creando wallet..."
WALLET_RESP=$(api_call POST "/api/v1/wallet/create")
echo "  Debug respuesta: $WALLET_RESP" | head -c 200
echo ""
WALLET1=$(echo "$WALLET_RESP" | jq -r '.data.address // .data.wallet.address // .address // .wallet.address // empty' 2>/dev/null || echo "")
if [ -z "$WALLET1" ] || [ "$WALLET1" = "null" ] || [ "$WALLET1" = "" ]; then
    echo "❌ Error creando wallet"
    echo "Respuesta completa: $WALLET_RESP"
    echo "Verificando servidor..."
    curl -s "${BASE_URL}/api/v1/health" | jq . || echo "Servidor no responde"
    exit 1
fi
echo "  ✅ Wallet: $WALLET1"

# Deploy NFT
echo ""
echo "🔧 Deployando contrato NFT..."
NFT_DATA=$(jq -n --arg o "$WALLET1" '{
    owner: $o,
    contract_type: "nft",
    name: "TestNFT",
    symbol: "TEST"
}')
NFT_RESP=$(api_call POST "/api/v1/contracts/deploy" "$NFT_DATA")
NFT_CONTRACT=$(echo "$NFT_RESP" | jq -r '.data.contract.address // .data.address // empty' 2>/dev/null)
if [ -z "$NFT_CONTRACT" ] || [ "$NFT_CONTRACT" = "null" ]; then
    echo "❌ Error deployando NFT"
    echo "Respuesta: $NFT_RESP"
    exit 1
fi
echo "  ✅ NFT Contract: $NFT_CONTRACT"

# Deploy ERC-20
echo ""
echo "🔧 Deployando contrato ERC-20..."
ERC20_DATA=$(jq -n --arg o "$WALLET1" '{
    owner: $o,
    contract_type: "token",
    name: "TestToken",
    symbol: "TEST",
    total_supply: 1000000,
    decimals: 18
}')
ERC20_RESP=$(api_call POST "/api/v1/contracts/deploy" "$ERC20_DATA")
ERC20_CONTRACT=$(echo "$ERC20_RESP" | jq -r '.data.contract.address // .data.address // empty' 2>/dev/null)
if [ -z "$ERC20_CONTRACT" ] || [ "$ERC20_CONTRACT" = "null" ]; then
    echo "❌ Error deployando ERC-20"
    exit 1
fi
echo "  ✅ ERC-20 Contract: $ERC20_CONTRACT"

# Tests
echo ""
echo "🧪 Ejecutando tests..."
echo ""

PASSED=0
FAILED=0

test_fail() {
    local name=$1
    local data=$2
    echo -n "  Test: $name ... "
    local resp=$(api_call POST "/api/v1/contracts/${NFT_CONTRACT}/execute" "$data")
    local err=$(echo "$resp" | jq -r '.error // .message // ""' 2>/dev/null)
    if [ -n "$err" ] || echo "$resp" | grep -qi "error"; then
        echo "✅ PASS"
        ((PASSED++))
    else
        echo "❌ FAIL"
        ((FAILED++))
    fi
}

test_pass() {
    local name=$1
    local data=$2
    echo -n "  Test: $name ... "
    local resp=$(api_call POST "/api/v1/contracts/${NFT_CONTRACT}/execute" "$data")
    local err=$(echo "$resp" | jq -r '.error // .message // ""' 2>/dev/null)
    if [ -z "$err" ] && ! echo "$resp" | grep -qi "error"; then
        echo "✅ PASS"
        ((PASSED++))
    else
        echo "❌ FAIL"
        ((FAILED++))
    fi
}

# Test 1: Token ID 0
echo "1️⃣  Token ID 0 debe fallar"
test_fail "Token ID 0" "$(jq -n --arg to "$WALLET1" '{
    function: { MintNFT: { to: $to, token_id: 0, token_uri: "ipfs://test" } }
}')"

# Test 2: Token ID > 1 billón
echo ""
echo "2️⃣  Token ID > 1 billón debe fallar"
test_fail "Token ID > 1 billón" "$(jq -n --arg to "$WALLET1" '{
    function: { MintNFT: { to: $to, token_id: 1000000001, token_uri: "ipfs://test" } }
}')"

# Test 3: Zero address
echo ""
echo "3️⃣  Zero address debe fallar"
test_fail "Zero address" "$(jq -n '{
    function: { MintNFT: { to: "0", token_id: 1, token_uri: "ipfs://test" } }
}')"

# Test 4: NFT en ERC-20
echo ""
echo "4️⃣  MintNFT en ERC-20 debe fallar"
echo -n "  Test: MintNFT en ERC-20 ... "
RESP=$(api_call POST "/api/v1/contracts/${ERC20_CONTRACT}/execute" "$(jq -n --arg to "$WALLET1" '{
    function: { MintNFT: { to: $to, token_id: 1, token_uri: "ipfs://test" } }
}')")
ERR=$(echo "$RESP" | jq -r '.error // .message // ""' 2>/dev/null)
if [ -n "$ERR" ] || echo "$RESP" | grep -qi "error"; then
    echo "✅ PASS"
    ((PASSED++))
else
    echo "❌ FAIL"
    ((FAILED++))
fi

# Test 5: Operación válida
echo ""
echo "5️⃣  Operación válida debe pasar"
test_pass "Mint NFT válido" "$(jq -n --arg to "$WALLET1" '{
    function: { MintNFT: { to: $to, token_id: 100, token_uri: "ipfs://test100" } }
}')"

# Resumen
echo ""
echo "============================"
echo "📊 Resumen"
echo "============================"
echo "✅ Tests pasados: $PASSED"
echo "❌ Tests fallidos: $FAILED"
echo ""

if [ $FAILED -eq 0 ]; then
    echo "🎉 ¡TODOS LOS TESTS PASARON!"
    exit 0
else
    echo "⚠️  Algunos tests fallaron"
    exit 1
fi

