#!/bin/bash

# Script para probar la persistencia de Smart Contracts
# Verifica que los contratos se guarden y carguen correctamente

set -e

API_URL="http://127.0.0.1:8080/api/v1"
TIMEOUT=10

echo "🧪 TEST: Persistencia de Smart Contracts"
echo "=========================================="
echo ""

# Verificar que el servidor esté corriendo
if ! curl -s --max-time 2 "$API_URL/health" >/dev/null 2>&1; then
    echo "❌ ERROR: El servidor no está respondiendo en $API_URL"
    echo "💡 Inicia el servidor primero: DIFFICULTY=1 cargo run --release 8080 8081 blockchain"
    exit 1
fi

echo "✅ Servidor detectado y respondiendo"
echo ""

# Crear un wallet para el owner
echo "📝 Paso 1: Creando wallet para owner del contrato..."
OWNER_WALLET=$(curl -s -X POST "$API_URL/wallets/create" --max-time $TIMEOUT 2>/dev/null)
OWNER_ADDRESS=$(echo "$OWNER_WALLET" | grep -o '"address":"[^"]*"' | head -1 | cut -d'"' -f4)

if [ -z "$OWNER_ADDRESS" ]; then
    echo "❌ Error: No se pudo crear wallet"
    exit 1
fi

echo "   Wallet creado: ${OWNER_ADDRESS:0:40}..."
echo ""

# Desplegar un contrato
echo "📝 Paso 2: Desplegando contrato de token..."
CONTRACT_DEPLOY=$(curl -s -X POST "$API_URL/contracts" \
    -H "Content-Type: application/json" \
    -d "{
        \"owner\": \"$OWNER_ADDRESS\",
        \"contract_type\": \"token\",
        \"name\": \"TestToken\",
        \"symbol\": \"TTK\",
        \"total_supply\": 1000000,
        \"decimals\": 18
    }" \
    --max-time $TIMEOUT 2>/dev/null)

CONTRACT_ADDRESS=$(echo "$CONTRACT_DEPLOY" | grep -o '"data":"[^"]*"' | head -1 | cut -d'"' -f4)

if [ -z "$CONTRACT_ADDRESS" ]; then
    echo "❌ Error: No se pudo desplegar contrato"
    echo "   Respuesta: $CONTRACT_DEPLOY"
    exit 1
fi

echo "   ✅ Contrato desplegado: ${CONTRACT_ADDRESS:0:50}..."
echo ""

# Verificar que el contrato existe
echo "📝 Paso 3: Verificando que el contrato se guardó..."
CONTRACT_INFO=$(curl -s -X GET "$API_URL/contracts/$CONTRACT_ADDRESS" --max-time $TIMEOUT 2>/dev/null)
CONTRACT_NAME=$(echo "$CONTRACT_INFO" | grep -o '"name":"[^"]*"' | head -1 | cut -d'"' -f4)

if [ "$CONTRACT_NAME" = "TestToken" ]; then
    echo "   ✅ Contrato encontrado: $CONTRACT_NAME"
else
    echo "   ❌ Error: Contrato no encontrado o nombre incorrecto"
    echo "   Respuesta: $CONTRACT_INFO"
    exit 1
fi
echo ""

# Ejecutar una función (mint)
echo "📝 Paso 4: Ejecutando función mint para crear tokens..."
MINT_RESULT=$(curl -s -X POST "$API_URL/contracts/$CONTRACT_ADDRESS/execute" \
    -H "Content-Type: application/json" \
    -d "{
        \"function\": \"mint\",
        \"params\": {
            \"to\": \"$OWNER_ADDRESS\",
            \"amount\": 1000
        }
    }" \
    --max-time $TIMEOUT 2>/dev/null)

if echo "$MINT_RESULT" | grep -q "success.*true"; then
    echo "   ✅ Tokens minteados exitosamente"
else
    echo "   ❌ Error al mintear tokens"
    echo "   Respuesta: $MINT_RESULT"
    exit 1
fi
echo ""

# Verificar balance
echo "📝 Paso 5: Verificando balance del owner..."
BALANCE_RESP=$(curl -s -X GET "$API_URL/contracts/$CONTRACT_ADDRESS/balance/$OWNER_ADDRESS" --max-time $TIMEOUT 2>/dev/null)
BALANCE=$(echo "$BALANCE_RESP" | grep -o '"data":[0-9]*' | cut -d':' -f2)

if [ "$BALANCE" = "1000" ]; then
    echo "   ✅ Balance correcto: $BALANCE tokens"
else
    echo "   ⚠️  Balance: $BALANCE (esperado: 1000)"
fi
echo ""

# Obtener todos los contratos
echo "📝 Paso 6: Listando todos los contratos..."
ALL_CONTRACTS=$(curl -s -X GET "$API_URL/contracts" --max-time $TIMEOUT 2>/dev/null)
CONTRACT_COUNT=$(echo "$ALL_CONTRACTS" | grep -o '"address"' | wc -l | tr -d ' ')

echo "   Total de contratos: $CONTRACT_COUNT"
echo ""

# Verificar que el contrato está en la lista
if echo "$ALL_CONTRACTS" | grep -q "$CONTRACT_ADDRESS"; then
    echo "   ✅ Contrato encontrado en la lista"
else
    echo "   ❌ Error: Contrato no encontrado en la lista"
    exit 1
fi
echo ""

echo "✅ TEST COMPLETADO: Persistencia básica funciona"
echo ""
echo "📋 Resumen:"
echo "   - Contrato desplegado: $CONTRACT_NAME"
echo "   - Dirección: ${CONTRACT_ADDRESS:0:50}..."
echo "   - Balance del owner: $BALANCE tokens"
echo "   - Total de contratos: $CONTRACT_COUNT"
echo ""
echo "💡 Para probar la persistencia completa:"
echo "   1. Detén el servidor (Ctrl+C)"
echo "   2. Reinicia el servidor"
echo "   3. Ejecuta: curl -s $API_URL/contracts/$CONTRACT_ADDRESS"
echo "   4. El contrato debería estar disponible con su estado preservado"
echo ""

