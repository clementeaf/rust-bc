#!/bin/bash

cd /Users/clementefalcone/Desktop/personal/rust-bc

# Limpiar
killall rust-bc 2>/dev/null
sleep 1
rm -f test_nft_security.db* 2>/dev/null

# Iniciar servidor
./target/release/rust-bc 20000 21000 test_nft_security > /tmp/rust-bc-investigation.log 2>&1 &
SERVER_PID=$!
echo "Servidor iniciado, PID: $SERVER_PID"

# Esperar
sleep 6

# Verificar
if ! curl -s http://localhost:20000/api/v1/health > /dev/null; then
    echo "❌ Servidor no responde"
    tail -20 /tmp/rust-bc-investigation.log
    kill $SERVER_PID 2>/dev/null
    exit 1
fi

echo "✅ Servidor respondiendo"
echo ""

# Crear wallet
WALLET1=$(curl -s -X POST http://localhost:20000/api/v1/wallets/create | jq -r '.data.address')
echo "Wallet: $WALLET1"
echo ""

# Preparar JSON
NFT_JSON=$(jq -n --arg owner "$WALLET1" '{"owner": $owner, "contract_type": "nft", "name": "TestNFT", "symbol": "TEST"}')
echo "JSON a enviar: $NFT_JSON"
echo ""

# Test 1: Endpoint DEBUG
echo "=== TEST 1: Endpoint /contracts/debug ==="
RESP1=$(curl -s -X POST http://localhost:20000/api/v1/contracts/debug \
    -H "Content-Type: application/json" \
    -d "$NFT_JSON")
echo "Respuesta: $RESP1"
echo "$RESP1" | jq . 2>/dev/null || echo "No es JSON válido"
echo ""

# Esperar un momento para logs
sleep 2

# Test 2: Endpoint NORMAL
echo "=== TEST 2: Endpoint /contracts/deploy ==="
RESP2=$(curl -s -X POST http://localhost:20000/api/v1/contracts/deploy \
    -H "Content-Type: application/json" \
    -d "$NFT_JSON")
echo "Respuesta: $RESP2"
echo "$RESP2" | jq . 2>/dev/null || echo "No es JSON válido"
echo ""

# Esperar para logs
sleep 2

# Mostrar logs
echo "=== LOGS DEL SERVIDOR ==="
echo ""
echo "--- Logs [MIDDLEWARE] ---"
grep "\[MIDDLEWARE\]" /tmp/rust-bc-investigation.log | tail -10
echo ""
echo "--- Logs [DEPLOY DEBUG] ---"
grep "\[DEPLOY DEBUG\]" /tmp/rust-bc-investigation.log | tail -20
echo ""
echo "--- Logs [DEPLOY] ---"
grep "\[DEPLOY\]" /tmp/rust-bc-investigation.log | tail -20
echo ""
echo "--- Logs [HASH] ---"
grep "\[HASH\]" /tmp/rust-bc-investigation.log | tail -10
echo ""
echo "--- Errores ---"
grep -i "error\|panic" /tmp/rust-bc-investigation.log | tail -10

# Limpiar
kill $SERVER_PID 2>/dev/null
killall rust-bc 2>/dev/null

