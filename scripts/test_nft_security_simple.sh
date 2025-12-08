#!/bin/bash

# Test de Seguridad - NFTs (Versión Simplificada)
# Valida las mejoras de seguridad más críticas

set -e

PORT=20000
BASE_URL="http://localhost:${PORT}"

echo "🔒 Test de Seguridad - NFTs (Simplificado)"
echo "==========================================="
echo ""

# Limpiar procesos anteriores
killall rust-bc 2>/dev/null || true
pkill -f rust-bc 2>/dev/null || true
rm -f test_nft_security.db test_nft_security.db-shm test_nft_security.db-wal 2>/dev/null || true
sleep 1

# Iniciar servidor
echo "📡 Iniciando servidor..."
cargo build --release > /dev/null 2>&1

# Los argumentos son posicionales: api_port p2p_port db_name
./target/release/rust-bc ${PORT} $((PORT + 1000)) test_nft_security > /tmp/rust-bc-test.log 2>&1 &
SERVER_PID=$!

# Esperar a que el servidor inicie (con retry)
echo "  Esperando servidor..."
for i in {1..10}; do
    sleep 1
    if curl -s "${BASE_URL}/api/v1/health" > /dev/null 2>&1; then
        echo "✅ Servidor iniciado correctamente (intento $i)"
        break
    fi
    if [ $i -eq 10 ]; then
        echo "❌ ERROR: Servidor no responde después de 10 intentos"
        echo "Logs:"
        tail -30 /tmp/rust-bc-test.log
        kill $SERVER_PID 2>/dev/null || true
        exit 1
    fi
done

echo "✅ Servidor iniciado correctamente"
echo ""

# Función de limpieza
cleanup() {
    echo ""
    echo "🧹 Limpiando..."
    kill $SERVER_PID 2>/dev/null || true
    killall rust-bc 2>/dev/null || true
    pkill -f rust-bc 2>/dev/null || true
    rm -f test_nft_security.db test_nft_security.db-shm test_nft_security.db-wal 2>/dev/null || true
}

trap cleanup EXIT

# Función helper
api_request() {
    local method=$1
    local endpoint=$2
    local data=$3
    
    if [ -z "$data" ]; then
        curl -s -X $method "${BASE_URL}${endpoint}" --max-time 5
    else
        curl -s -X $method "${BASE_URL}${endpoint}" \
            -H "Content-Type: application/json" \
            -d "$data" --max-time 5
    fi
}

# Crear wallet
echo "📝 Creando wallet..."
WALLET_RESPONSE=$(api_request POST "/api/v1/wallet/create")
WALLET1=$(echo "$WALLET_RESPONSE" | jq -r '.data.address // .data.wallet.address // .address // .wallet.address // empty' 2>/dev/null)
if [ -z "$WALLET1" ]; then
    echo "❌ ERROR: No se pudo crear wallet"
    echo "Respuesta: $WALLET_RESPONSE"
    exit 1
fi
echo "  Wallet: $WALLET1"
echo ""

# Deployar contrato NFT
echo "🔧 Deployando contrato NFT..."
NFT_DATA=$(jq -n \
    --arg owner "$WALLET1" \
    '{
        owner: $owner,
        contract_type: "nft",
        name: "TestNFT",
        symbol: "TEST"
    }')

NFT_CONTRACT=$(api_request POST "/api/v1/contracts/deploy" "$NFT_DATA" | jq -r '.contract.address // .address // empty')
if [ -z "$NFT_CONTRACT" ]; then
    echo "❌ ERROR: No se pudo deployar contrato NFT"
    exit 1
fi
echo "  NFT Contract: $NFT_CONTRACT"
echo ""

# Deployar contrato ERC-20
echo "🔧 Deployando contrato ERC-20..."
ERC20_DATA=$(jq -n \
    --arg owner "$WALLET1" \
    '{
        owner: $owner,
        contract_type: "token",
        name: "TestToken",
        symbol: "TEST",
        total_supply: 1000000,
        decimals: 18
    }')

ERC20_CONTRACT=$(api_request POST "/api/v1/contracts/deploy" "$ERC20_DATA" | jq -r '.contract.address // .address // empty')
if [ -z "$ERC20_CONTRACT" ]; then
    echo "❌ ERROR: No se pudo deployar contrato ERC-20"
    exit 1
fi
echo "  ERC-20 Contract: $ERC20_CONTRACT"
echo ""

# Contador
TESTS_PASSED=0
TESTS_FAILED=0

# Función de test
test_case() {
    local name=$1
    local should_fail=$2
    shift 2
    local command="$@"
    
    echo -n "  Test: $name ... "
    
    local output=$(eval "$command" 2>&1)
    local error=$(echo "$output" | jq -r '.error // .message // ""' 2>/dev/null || echo "")
    
    if [ "$should_fail" = "true" ]; then
        if [ -n "$error" ] || echo "$output" | grep -qi "error\|failed"; then
            echo "✅ PASS"
            ((TESTS_PASSED++))
        else
            echo "❌ FAIL (debería haber fallado)"
            ((TESTS_FAILED++))
        fi
    else
        if [ -z "$error" ] && ! echo "$output" | grep -qi "error\|failed"; then
            echo "✅ PASS"
            ((TESTS_PASSED++))
        else
            echo "❌ FAIL"
            echo "    Error: $error"
            ((TESTS_FAILED++))
        fi
    fi
}

echo "🧪 Ejecutando tests..."
echo ""

# TEST 1: Token ID 0
echo "1️⃣  Token ID 0 debe ser rechazado"
MINT_DATA=$(jq -n \
    --arg to "$WALLET1" \
    '{
        function: {
            MintNFT: {
                to: $to,
                token_id: 0,
                token_uri: "ipfs://test"
            }
        }
    }')
test_case "Token ID 0" true \
    api_request POST "/api/v1/contracts/${NFT_CONTRACT}/execute" "$MINT_DATA"

# TEST 2: Token ID > 1 billón
echo ""
echo "2️⃣  Token ID > 1 billón debe ser rechazado"
MINT_DATA=$(jq -n \
    --arg to "$WALLET1" \
    '{
        function: {
            MintNFT: {
                to: $to,
                token_id: 1000000001,
                token_uri: "ipfs://test"
            }
        }
    }')
test_case "Token ID > 1 billón" true \
    api_request POST "/api/v1/contracts/${NFT_CONTRACT}/execute" "$MINT_DATA"

# TEST 3: Zero address
echo ""
echo "3️⃣  Zero address debe ser rechazado"
MINT_DATA=$(jq -n \
    '{
        function: {
            MintNFT: {
                to: "0",
                token_id: 1,
                token_uri: "ipfs://test"
            }
        }
    }')
test_case "Zero address" true \
    api_request POST "/api/v1/contracts/${NFT_CONTRACT}/execute" "$MINT_DATA"

# TEST 4: Funciones NFT en contrato ERC-20
echo ""
echo "4️⃣  Funciones NFT en contrato ERC-20 deben fallar"
MINT_DATA=$(jq -n \
    --arg to "$WALLET1" \
    '{
        function: {
            MintNFT: {
                to: $to,
                token_id: 1,
                token_uri: "ipfs://test"
            }
        }
    }')
test_case "MintNFT en ERC-20" true \
    api_request POST "/api/v1/contracts/${ERC20_CONTRACT}/execute" "$MINT_DATA"

# TEST 5: Operación válida
echo ""
echo "5️⃣  Operación válida debe pasar"
MINT_DATA=$(jq -n \
    --arg to "$WALLET1" \
    '{
        function: {
            MintNFT: {
                to: $to,
                token_id: 100,
                token_uri: "ipfs://test100"
            }
        }
    }')
test_case "Mint NFT válido" false \
    api_request POST "/api/v1/contracts/${NFT_CONTRACT}/execute" "$MINT_DATA"

# RESUMEN
echo ""
echo "============================"
echo "📊 Resumen"
echo "============================"
echo "✅ Tests pasados: $TESTS_PASSED"
echo "❌ Tests fallidos: $TESTS_FAILED"
echo ""

if [ $TESTS_FAILED -eq 0 ]; then
    echo "🎉 ¡TODOS LOS TESTS PASARON!"
    exit 0
else
    echo "⚠️  Algunos tests fallaron"
    exit 1
fi

