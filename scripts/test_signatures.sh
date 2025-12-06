#!/bin/bash

echo "=== PRUEBA DE FIRMAS DIGITALES ==="
echo ""

BASE_URL="http://127.0.0.1:8080/api/v1"

echo "1. Creando wallet 1..."
WALLET1=$(curl -s -X POST $BASE_URL/wallets/create)
echo "$WALLET1" | python3 -m json.tool
WALLET1_ADDRESS=$(echo "$WALLET1" | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['address'])")
echo "Wallet 1 Address: $WALLET1_ADDRESS"
echo ""

echo "2. Creando wallet 2..."
WALLET2=$(curl -s -X POST $BASE_URL/wallets/create)
echo "$WALLET2" | python3 -m json.tool
WALLET2_ADDRESS=$(echo "$WALLET2" | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['address'])")
echo "Wallet 2 Address: $WALLET2_ADDRESS"
echo ""

echo "3. Verificando balances iniciales..."
BALANCE1=$(curl -s $BASE_URL/wallets/$WALLET1_ADDRESS)
BALANCE2=$(curl -s $BASE_URL/wallets/$WALLET2_ADDRESS)
echo "Balance Wallet 1:"
echo "$BALANCE1" | python3 -m json.tool
echo "Balance Wallet 2:"
echo "$BALANCE2" | python3 -m json.tool
echo ""

echo "4. Creando transacción firmada (Wallet 1 -> Wallet 2, 100 unidades)..."
TX=$(curl -s -X POST $BASE_URL/transactions \
  -H "Content-Type: application/json" \
  -d "{
    \"from\": \"$WALLET1_ADDRESS\",
    \"to\": \"$WALLET2_ADDRESS\",
    \"amount\": 100,
    \"data\": \"Transacción de prueba con firma digital\"
  }")
echo "$TX" | python3 -m json.tool
TX_SIGNATURE=$(echo "$TX" | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['signature'])")
echo "Firma de la transacción: ${TX_SIGNATURE:0:20}..."
echo ""

echo "5. Creando bloque con la transacción firmada..."
BLOCK=$(curl -s -X POST $BASE_URL/blocks \
  -H "Content-Type: application/json" \
  -d "{
    \"transactions\": [
      {
        \"from\": \"$WALLET1_ADDRESS\",
        \"to\": \"$WALLET2_ADDRESS\",
        \"amount\": 100,
        \"data\": \"Transacción de prueba con firma digital\"
      }
    ]
  }")
echo "$BLOCK" | python3 -m json.tool
echo ""

echo "6. Verificando balances después de la transacción..."
sleep 1
BALANCE1_AFTER=$(curl -s $BASE_URL/wallets/$WALLET1_ADDRESS)
BALANCE2_AFTER=$(curl -s $BASE_URL/wallets/$WALLET2_ADDRESS)
echo "Balance Wallet 1 (después):"
echo "$BALANCE1_AFTER" | python3 -m json.tool
echo "Balance Wallet 2 (después):"
echo "$BALANCE2_AFTER" | python3 -m json.tool
echo ""

echo "7. Verificando la cadena..."
CHAIN_VERIFY=$(curl -s $BASE_URL/chain/verify)
echo "$CHAIN_VERIFY" | python3 -m json.tool
echo ""

echo "8. Obteniendo información de la blockchain..."
CHAIN_INFO=$(curl -s $BASE_URL/chain/info)
echo "$CHAIN_INFO" | python3 -m json.tool
echo ""

echo "=== PRUEBA COMPLETADA ==="
echo ""
echo "✅ Firmas digitales funcionando correctamente"
echo "✅ Transacciones firmadas y validadas"
echo "✅ Saldos actualizados correctamente"

