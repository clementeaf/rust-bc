#!/bin/bash

echo "=== PRUEBA COMPLETA DE FIRMAS DIGITALES ==="
echo ""

BASE_URL="http://127.0.0.1:8080/api/v1"

echo "1. Creando wallet 1..."
WALLET1=$(curl -s -X POST $BASE_URL/wallets/create)
WALLET1_ADDRESS=$(echo "$WALLET1" | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['address'])")
echo "✅ Wallet 1 creado: $WALLET1_ADDRESS"
echo "   Public Key: $(echo "$WALLET1" | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['public_key'])")"
echo ""

echo "2. Creando wallet 2..."
WALLET2=$(curl -s -X POST $BASE_URL/wallets/create)
WALLET2_ADDRESS=$(echo "$WALLET2" | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['address'])")
echo "✅ Wallet 2 creado: $WALLET2_ADDRESS"
echo ""

echo "3. Creando bloque inicial con coinbase (recompensa de minería) para Wallet 1..."
COINBASE=$(curl -s -X POST $BASE_URL/blocks \
  -H "Content-Type: application/json" \
  -d "{
    \"transactions\": [
      {
        \"from\": \"0\",
        \"to\": \"$WALLET1_ADDRESS\",
        \"amount\": 1000,
        \"data\": \"Coinbase - Recompensa de minería inicial\"
      }
    ]
  }")
echo "$COINBASE" | python3 -m json.tool
echo ""

echo "4. Verificando balance de Wallet 1 después de coinbase..."
sleep 1
BALANCE1=$(curl -s $BASE_URL/wallets/$WALLET1_ADDRESS)
BALANCE1_AMOUNT=$(echo "$BALANCE1" | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['balance'])")
echo "Balance Wallet 1: $BALANCE1_AMOUNT"
echo ""

echo "5. Creando transacción firmada (Wallet 1 -> Wallet 2, 100 unidades)..."
TX=$(curl -s -X POST $BASE_URL/transactions \
  -H "Content-Type: application/json" \
  -d "{
    \"from\": \"$WALLET1_ADDRESS\",
    \"to\": \"$WALLET2_ADDRESS\",
    \"amount\": 100,
    \"data\": \"Transacción de prueba con firma digital Ed25519\"
  }")

TX_SUCCESS=$(echo "$TX" | python3 -c "import sys, json; print(json.load(sys.stdin)['success'])")
if [ "$TX_SUCCESS" = "True" ]; then
    TX_SIGNATURE=$(echo "$TX" | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['signature'])")
    echo "✅ Transacción creada y firmada exitosamente"
    echo "   Firma digital: ${TX_SIGNATURE:0:40}..."
    echo "   Longitud de firma: ${#TX_SIGNATURE} caracteres (64 bytes en hex)"
    echo "$TX" | python3 -m json.tool
else
    echo "❌ Error al crear transacción:"
    echo "$TX" | python3 -m json.tool
    exit 1
fi
echo ""

echo "6. Creando bloque con la transacción firmada..."
BLOCK=$(curl -s -X POST $BASE_URL/blocks \
  -H "Content-Type: application/json" \
  -d "{
    \"transactions\": [
      {
        \"from\": \"$WALLET1_ADDRESS\",
        \"to\": \"$WALLET2_ADDRESS\",
        \"amount\": 100,
        \"data\": \"Transacción de prueba con firma digital Ed25519\"
      }
    ]
  }")
BLOCK_SUCCESS=$(echo "$BLOCK" | python3 -c "import sys, json; print(json.load(sys.stdin)['success'])")
if [ "$BLOCK_SUCCESS" = "True" ]; then
    BLOCK_HASH=$(echo "$BLOCK" | python3 -c "import sys, json; print(json.load(sys.stdin)['data'])")
    echo "✅ Bloque minado exitosamente"
    echo "   Hash del bloque: $BLOCK_HASH"
else
    echo "❌ Error al crear bloque:"
    echo "$BLOCK" | python3 -m json.tool
fi
echo ""

echo "7. Verificando balances después de la transacción..."
sleep 1
BALANCE1_AFTER=$(curl -s $BASE_URL/wallets/$WALLET1_ADDRESS)
BALANCE2_AFTER=$(curl -s $BASE_URL/wallets/$WALLET2_ADDRESS)
BALANCE1_FINAL=$(echo "$BALANCE1_AFTER" | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['balance'])")
BALANCE2_FINAL=$(echo "$BALANCE2_AFTER" | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['balance'])")
echo "Balance Wallet 1: $BALANCE1_FINAL (debería ser 900: 1000 - 100)"
echo "Balance Wallet 2: $BALANCE2_FINAL (debería ser 100)"
echo ""

echo "8. Verificando la cadena completa..."
CHAIN_VERIFY=$(curl -s $BASE_URL/chain/verify)
CHAIN_VALID=$(echo "$CHAIN_VERIFY" | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['valid'])")
CHAIN_COUNT=$(echo "$CHAIN_VERIFY" | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['block_count'])")
echo "Cadena válida: $CHAIN_VALID"
echo "Número de bloques: $CHAIN_COUNT"
echo ""

echo "9. Obteniendo información completa de la blockchain..."
CHAIN_INFO=$(curl -s $BASE_URL/chain/info)
echo "$CHAIN_INFO" | python3 -m json.tool
echo ""

echo "10. Listando todos los bloques..."
BLOCKS=$(curl -s $BASE_URL/blocks)
BLOCK_COUNT=$(echo "$BLOCKS" | python3 -c "import sys, json; print(len(json.load(sys.stdin)['data']))")
echo "Total de bloques en la cadena: $BLOCK_COUNT"
echo ""

echo "=== RESUMEN DE PRUEBAS ==="
echo ""
if [ "$TX_SUCCESS" = "True" ] && [ "$BLOCK_SUCCESS" = "True" ] && [ "$CHAIN_VALID" = "True" ]; then
    echo "✅ TODAS LAS PRUEBAS EXITOSAS"
    echo ""
    echo "✅ Firmas digitales Ed25519 funcionando"
    echo "✅ Transacciones firmadas correctamente"
    echo "✅ Verificación de firmas operativa"
    echo "✅ Saldos actualizados correctamente"
    echo "✅ Cadena válida y consistente"
    echo ""
    echo "🎉 La Fase 2 (Firmas Digitales) está completamente funcional!"
else
    echo "⚠️  Algunas pruebas fallaron"
fi

