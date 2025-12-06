#!/bin/bash

echo "=== PRUEBA SIMPLE DE FIRMAS ==="
BASE_URL="http://127.0.0.1:8080/api/v1"

echo "1. Crear wallet..."
W1=$(curl -s -X POST $BASE_URL/wallets/create | python3 -c "import sys, json; w=json.load(sys.stdin)['data']; print(w['address'])")
echo "Wallet creado: $W1"
echo ""

echo "2. Ver balance inicial..."
curl -s $BASE_URL/wallets/$W1 | python3 -m json.tool
echo ""

echo "3. Crear bloque coinbase (1000 unidades)..."
curl -s -X POST $BASE_URL/blocks -H "Content-Type: application/json" -d "{\"transactions\":[{\"from\":\"0\",\"to\":\"$W1\",\"amount\":1000}]}" | python3 -m json.tool
echo ""

echo "4. Ver balance después de coinbase..."
sleep 1
curl -s $BASE_URL/wallets/$W1 | python3 -m json.tool
echo ""

echo "5. Ver todos los bloques..."
curl -s $BASE_URL/blocks | python3 -c "import sys, json; blocks=json.load(sys.stdin)['data']; print(f'Total bloques: {len(blocks)}'); [print(f'  Bloque {b[\"index\"]}: {len(b[\"transactions\"])} transacciones') for b in blocks]"

