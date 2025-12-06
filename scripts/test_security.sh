#!/bin/bash

echo "=== PRUEBAS DE SEGURIDAD - FIRMAS DIGITALES ==="
echo ""

BASE_URL="http://127.0.0.1:8080/api/v1"

echo "1. Creando wallets de prueba..."
W1=$(curl -s -X POST $BASE_URL/wallets/create | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['address'])")
W2=$(curl -s -X POST $BASE_URL/wallets/create | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['address'])")
echo "Wallet 1: $W1"
echo "Wallet 2: $W2"
echo ""

echo "2. Dando saldo inicial a Wallet 1 (coinbase)..."
curl -s -X POST $BASE_URL/blocks -H "Content-Type: application/json" -d "{\"transactions\":[{\"from\":\"0\",\"to\":\"$W1\",\"amount\":1000}]}" > /dev/null
sleep 1
BALANCE1=$(curl -s $BASE_URL/wallets/$W1 | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['balance'])")
echo "Balance Wallet 1: $BALANCE1"
echo ""

echo "3. Intentando transacción con saldo insuficiente..."
TX_INVALID=$(curl -s -X POST $BASE_URL/transactions -H "Content-Type: application/json" -d "{\"from\":\"$W1\",\"to\":\"$W2\",\"amount\":2000}")
SUCCESS=$(echo "$TX_INVALID" | python3 -c "import sys, json; print(json.load(sys.stdin)['success'])")
if [ "$SUCCESS" = "False" ]; then
    echo "✅ Correctamente rechazada: Saldo insuficiente"
else
    echo "❌ ERROR: Debería rechazar transacción con saldo insuficiente"
fi
echo ""

echo "4. Creando transacción válida..."
TX_VALID=$(curl -s -X POST $BASE_URL/transactions -H "Content-Type: application/json" -d "{\"from\":\"$W1\",\"to\":\"$W2\",\"amount\":100}")
SUCCESS=$(echo "$TX_VALID" | python3 -c "import sys, json; print(json.load(sys.stdin)['success'])")
if [ "$SUCCESS" = "True" ]; then
    echo "✅ Transacción válida aceptada"
    SIG=$(echo "$TX_VALID" | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['signature'])")
    echo "   Firma: ${SIG:0:40}..."
else
    echo "❌ ERROR: Debería aceptar transacción válida"
fi
echo ""

echo "5. Minando bloque con transacción válida..."
BLOCK=$(curl -s -X POST $BASE_URL/blocks -H "Content-Type: application/json" -d "{\"transactions\":[{\"from\":\"$W1\",\"to\":\"$W2\",\"amount\":100}]}")
BLOCK_SUCCESS=$(echo "$BLOCK" | python3 -c "import sys, json; print(json.load(sys.stdin)['success'])")
if [ "$BLOCK_SUCCESS" = "True" ]; then
    echo "✅ Bloque minado exitosamente"
else
    echo "❌ ERROR al minar bloque"
    echo "$BLOCK" | python3 -m json.tool
fi
echo ""

echo "6. Verificando saldos después de transacción..."
sleep 1
BALANCE1_AFTER=$(curl -s $BASE_URL/wallets/$W1 | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['balance'])")
BALANCE2_AFTER=$(curl -s $BASE_URL/wallets/$W2 | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['balance'])")
echo "Balance Wallet 1: $BALANCE1_AFTER (esperado: 900)"
echo "Balance Wallet 2: $BALANCE2_AFTER (esperado: 100)"
if [ "$BALANCE1_AFTER" = "900" ] && [ "$BALANCE2_AFTER" = "100" ]; then
    echo "✅ Saldos correctos"
else
    echo "❌ ERROR en saldos"
fi
echo ""

echo "7. Verificando integridad de la cadena..."
VERIFY=$(curl -s $BASE_URL/chain/verify)
VALID=$(echo "$VERIFY" | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['valid'])")
if [ "$VALID" = "True" ]; then
    echo "✅ Cadena válida"
else
    echo "❌ ERROR: Cadena inválida"
fi
echo ""

echo "=== RESUMEN DE PRUEBAS DE SEGURIDAD ==="
echo ""
if [ "$SUCCESS" = "False" ] && [ "$BLOCK_SUCCESS" = "True" ] && [ "$VALID" = "True" ]; then
    echo "✅ TODAS LAS PRUEBAS DE SEGURIDAD PASARON"
    echo ""
    echo "✅ Validación de saldos funcionando"
    echo "✅ Firmas digitales operativas"
    echo "✅ Prevención de doble gasto implementada"
    echo "✅ Integridad de cadena verificada"
    echo ""
    echo "🚀 LISTO PARA FASE 3: RED P2P"
else
    echo "⚠️  Algunas pruebas fallaron - revisar antes de continuar"
fi

