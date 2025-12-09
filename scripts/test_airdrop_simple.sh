#!/bin/bash

# Script simplificado de testing para el sistema de airdrop
# Versión rápida para verificación básica

set -e

API_URL="http://127.0.0.1:8080/api/v1"

echo "🧪 Test Simplificado del Sistema de Airdrop"
echo "=========================================="
echo ""

# Verificar servidor
if ! curl -s "${API_URL}/health" > /dev/null 2>&1; then
    echo "❌ Error: El servidor no está corriendo"
    echo "   Inicia el servidor con: cargo run"
    exit 1
fi
echo "✅ Servidor está corriendo"
echo ""

# Crear wallet
echo "1. Creando wallet..."
WALLET=$(curl -s -X POST "${API_URL}/wallets/create" | jq -r '.data.address')
echo "   Wallet: ${WALLET}"
echo ""

# Minar bloques para crear tracking
echo "2. Minando bloques para crear tracking..."
for i in {1..3}; do
    curl -s -X POST "${API_URL}/mine" \
        -H "Content-Type: application/json" \
        -d "{\"miner_address\": \"${WALLET}\", \"max_transactions\": 10}" > /dev/null
    echo "   Bloque $i minado"
done
echo ""

# Verificar tracking
echo "3. Verificando tracking..."
TRACKING=$(curl -s "${API_URL}/airdrop/tracking/${WALLET}")
IS_ELIGIBLE=$(echo "$TRACKING" | jq -r '.data.is_eligible // false')
echo "   Es elegible: ${IS_ELIGIBLE}"
echo ""

# Ver estadísticas
echo "4. Estadísticas del airdrop:"
STATS=$(curl -s "${API_URL}/airdrop/statistics")
echo "$STATS" | jq '.data'
echo ""

# Si es elegible, intentar reclamar
if [ "$IS_ELIGIBLE" = "true" ]; then
    echo "5. Reclamando airdrop..."
    CLAIM=$(curl -s -X POST "${API_URL}/airdrop/claim" \
        -H "Content-Type: application/json" \
        -d "{\"node_address\": \"${WALLET}\"}")
    
    if echo "$CLAIM" | jq -e '.success == true' > /dev/null 2>&1; then
        echo "   ✅ Airdrop reclamado exitosamente"
        echo "$CLAIM" | jq '.data'
    else
        echo "   ❌ Error al reclamar:"
        echo "$CLAIM" | jq '.message'
    fi
else
    echo "5. ⚠️  Nodo no es elegible (necesita estar entre los primeros 500 nodos)"
fi
echo ""

echo "✅ Test completado"

