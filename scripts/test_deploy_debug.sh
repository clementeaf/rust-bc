#!/bin/bash

cd /Users/clementefalcone/Desktop/personal/rust-bc

# Limpiar
killall rust-bc 2>/dev/null
sleep 1
rm -f test_nft_security.db* 2>/dev/null

# Iniciar servidor en background
./target/release/rust-bc 20000 21000 test_nft_security > /tmp/rust-bc-debug.log 2>&1 &
SERVER_PID=$!
echo "Servidor iniciado, PID: $SERVER_PID"

# Esperar a que inicie
sleep 6

# Verificar que está vivo
if ! curl -s http://localhost:20000/api/v1/health > /dev/null; then
    echo "❌ Servidor no responde"
    tail -20 /tmp/rust-bc-debug.log
    kill $SERVER_PID 2>/dev/null
    exit 1
fi

echo "✅ Servidor respondiendo"

# Crear wallet
WALLET1=$(curl -s -X POST http://localhost:20000/api/v1/wallets/create | jq -r '.data.address')
echo "Wallet: $WALLET1"

# Deploy
NFT_JSON=$(jq -n --arg owner "$WALLET1" '{"owner": $owner, "contract_type": "nft", "name": "TestNFT", "symbol": "TEST"}')
echo ""
echo "=== Deploy NFT ==="
echo "JSON: $NFT_JSON"

# Hacer request y capturar logs en tiempo real
curl -s --max-time 10 -X POST http://localhost:20000/api/v1/contracts/deploy \
    -H "Content-Type: application/json" \
    -d "$NFT_JSON" &
CURL_PID=$!

# Monitorear logs mientras se ejecuta
for i in {1..5}; do
    sleep 1
    echo ""
    echo "--- Logs en segundo $i ---"
    tail -20 /tmp/rust-bc-debug.log | grep -E "\[DEPLOY\]|\[HASH\]" | tail -10
done

wait $CURL_PID 2>/dev/null
RESPONSE=$?

echo ""
echo "=== Respuesta completa ==="
curl -s -X POST http://localhost:20000/api/v1/contracts/deploy \
    -H "Content-Type: application/json" \
    -d "$NFT_JSON" | jq .

echo ""
echo "=== Todos los logs [DEPLOY] y [HASH] ==="
grep -E "\[DEPLOY\]|\[HASH\]" /tmp/rust-bc-debug.log

echo ""
echo "=== Errores ==="
grep -i "error\|panic" /tmp/rust-bc-debug.log | tail -10

# Limpiar
kill $SERVER_PID 2>/dev/null
killall rust-bc 2>/dev/null

