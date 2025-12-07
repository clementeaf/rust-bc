#!/bin/bash

# Script para probar la sincronización P2P de contratos entre nodos
# Requiere dos instancias del servidor corriendo en puertos diferentes

set -e

API_PORT_1=8080
API_PORT_2=8081
P2P_PORT_1=5000
P2P_PORT_2=5001

BASE_URL_1="http://localhost:${API_PORT_1}/api/v1"
BASE_URL_2="http://localhost:${API_PORT_2}/api/v1"

echo "🧪 Test de Sincronización P2P de Contratos"
echo "=========================================="
echo ""
echo "Este script prueba la sincronización de contratos entre dos nodos."
echo "Asegúrate de tener dos instancias del servidor corriendo:"
echo "  - Nodo 1: API en puerto ${API_PORT_1}, P2P en puerto ${P2P_PORT_1}"
echo "  - Nodo 2: API en puerto ${API_PORT_2}, P2P en puerto ${P2P_PORT_2}"
echo ""
echo "Iniciando prueba en 2 segundos..."
sleep 2

# Función para esperar a que el servidor esté listo
wait_for_server() {
    local url=$1
    local max_attempts=30
    local attempt=0
    
    echo "⏳ Esperando servidor en ${url}..."
    while [ $attempt -lt $max_attempts ]; do
        if curl -s "${url}/health" > /dev/null 2>&1; then
            echo "✅ Servidor ${url} está listo"
            return 0
        fi
        attempt=$((attempt + 1))
        sleep 1
    done
    
    echo "❌ Servidor ${url} no está disponible después de ${max_attempts} intentos"
    return 1
}

# Verificar que ambos servidores estén corriendo
echo ""
echo "📡 Verificando servidores..."
wait_for_server "${BASE_URL_1}" || exit 1
wait_for_server "${BASE_URL_2}" || exit 1

# Crear wallet en nodo 1
echo ""
echo "📝 Creando wallet en Nodo 1..."
WALLET_1=$(curl -s -X POST "${BASE_URL_1}/wallets/create" | jq -r '.data.address')
if [ -z "$WALLET_1" ] || [ "$WALLET_1" == "null" ]; then
    echo "❌ Error al crear wallet en Nodo 1"
    exit 1
fi
echo "✅ Wallet creado en Nodo 1: ${WALLET_1}"

# Crear wallet en nodo 2
echo ""
echo "📝 Creando wallet en Nodo 2..."
WALLET_2=$(curl -s -X POST "${BASE_URL_2}/wallets/create" | jq -r '.data.address')
if [ -z "$WALLET_2" ] || [ "$WALLET_2" == "null" ]; then
    echo "❌ Error al crear wallet en Nodo 2"
    exit 1
fi
echo "✅ Wallet creado en Nodo 2: ${WALLET_2}"

# Minar un bloque en nodo 1 para tener fondos
echo ""
echo "⛏️  Minando bloque en Nodo 1 para tener fondos..."
MINE_RESULT=$(curl -s -X POST "${BASE_URL_1}/mine" \
    -H "Content-Type: application/json" \
    -d "{\"miner_address\": \"${WALLET_1}\"}")
echo "✅ Bloque minado"

# Desplegar contrato en Nodo 1
echo ""
echo "📋 Desplegando contrato en Nodo 1..."
CONTRACT_DATA=$(cat <<EOF
{
  "owner": "${WALLET_1}",
  "contract_type": "token",
  "name": "TestToken",
  "symbol": "TEST",
  "total_supply": 1000000,
  "decimals": 18
}
EOF
)

DEPLOY_RESPONSE=$(curl -s -X POST "${BASE_URL_1}/contracts" \
    -H "Content-Type: application/json" \
    -d "${CONTRACT_DATA}")

CONTRACT_ADDRESS=$(echo "$DEPLOY_RESPONSE" | jq -r '.data')
if [ -z "$CONTRACT_ADDRESS" ] || [ "$CONTRACT_ADDRESS" == "null" ]; then
    echo "❌ Error al desplegar contrato en Nodo 1"
    echo "Respuesta: ${DEPLOY_RESPONSE}"
    exit 1
fi
echo "✅ Contrato desplegado en Nodo 1: ${CONTRACT_ADDRESS}"

# Verificar que el contrato existe en Nodo 1
echo ""
echo "🔍 Verificando contrato en Nodo 1..."
CONTRACT_1=$(curl -s "${BASE_URL_1}/contracts/${CONTRACT_ADDRESS}" | jq -r '.data.address')
if [ "$CONTRACT_1" != "$CONTRACT_ADDRESS" ]; then
    echo "❌ Contrato no encontrado en Nodo 1"
    exit 1
fi
echo "✅ Contrato verificado en Nodo 1"

# Verificar que el contrato NO existe en Nodo 2 (antes de conectar)
echo ""
echo "🔍 Verificando que el contrato NO existe en Nodo 2 (antes de conectar)..."
CONTRACT_2_BEFORE=$(curl -s "${BASE_URL_2}/contracts/${CONTRACT_ADDRESS}" | jq -r '.error // "not_found"')
if [ "$CONTRACT_2_BEFORE" != "not_found" ] && [ "$CONTRACT_2_BEFORE" != "Contract not found" ]; then
    echo "⚠️  Contrato ya existe en Nodo 2 antes de conectar (puede ser de una prueba anterior)"
else
    echo "✅ Contrato no existe en Nodo 2 (como se esperaba)"
fi

# Conectar Nodo 2 a Nodo 1
echo ""
echo "🔗 Conectando Nodo 2 a Nodo 1..."
CONNECT_RESPONSE=$(curl -s -X POST "${BASE_URL_2}/peers/127.0.0.1:${P2P_PORT_1}/connect")

if echo "$CONNECT_RESPONSE" | jq -e '.success' > /dev/null 2>&1; then
    echo "✅ Nodo 2 conectado a Nodo 1"
else
    echo "⚠️  Respuesta de conexión: ${CONNECT_RESPONSE}"
    echo "⚠️  Continuando con la prueba (puede que ya estén conectados)..."
fi

# Esperar un momento para que la sincronización ocurra
echo ""
echo "⏳ Esperando sincronización de contratos..."
sleep 3

# Verificar que el contrato ahora existe en Nodo 2
echo ""
echo "🔍 Verificando que el contrato ahora existe en Nodo 2..."
CONTRACT_2_AFTER=$(curl -s "${BASE_URL_2}/contracts/${CONTRACT_ADDRESS}" | jq -r '.data.address // "not_found"')
if [ "$CONTRACT_2_AFTER" != "$CONTRACT_ADDRESS" ]; then
    echo "❌ Contrato no sincronizado en Nodo 2"
    echo "Respuesta: $(curl -s "${BASE_URL_2}/contracts/${CONTRACT_ADDRESS}")"
    exit 1
fi
echo "✅ Contrato sincronizado en Nodo 2: ${CONTRACT_2_AFTER}"

# Verificar detalles del contrato en ambos nodos
echo ""
echo "🔍 Comparando detalles del contrato en ambos nodos..."
CONTRACT_1_DETAILS=$(curl -s "${BASE_URL_1}/contracts/${CONTRACT_ADDRESS}")
CONTRACT_2_DETAILS=$(curl -s "${BASE_URL_2}/contracts/${CONTRACT_ADDRESS}")

NAME_1=$(echo "$CONTRACT_1_DETAILS" | jq -r '.data.name')
NAME_2=$(echo "$CONTRACT_2_DETAILS" | jq -r '.data.name')

if [ "$NAME_1" != "$NAME_2" ]; then
    echo "❌ Los nombres del contrato no coinciden"
    echo "Nodo 1: ${NAME_1}"
    echo "Nodo 2: ${NAME_2}"
    exit 1
fi
echo "✅ Detalles del contrato coinciden: ${NAME_1}"

# Ejecutar función en Nodo 1 (mint)
echo ""
echo "💰 Ejecutando mint en Nodo 1..."
MINT_DATA=$(cat <<EOF
{
  "function": "mint",
  "params": {
    "to": "${WALLET_1}",
    "amount": 1000
  }
}
EOF
)

MINT_RESPONSE=$(curl -s -X POST "${BASE_URL_1}/contracts/${CONTRACT_ADDRESS}/execute" \
    -H "Content-Type: application/json" \
    -d "${MINT_DATA}")

if echo "$MINT_RESPONSE" | jq -e '.success' > /dev/null 2>&1; then
    echo "✅ Mint ejecutado en Nodo 1"
else
    echo "❌ Error al ejecutar mint: ${MINT_RESPONSE}"
    exit 1
fi

# Esperar sincronización de actualización
echo ""
echo "⏳ Esperando sincronización de actualización del contrato..."
sleep 3

# Verificar balance en Nodo 2
echo ""
echo "🔍 Verificando balance en Nodo 2..."
BALANCE_2=$(curl -s "${BASE_URL_2}/contracts/${CONTRACT_ADDRESS}/balance/${WALLET_1}" | jq -r '.data // "0"')
if [ "$BALANCE_2" == "0" ] || [ "$BALANCE_2" == "null" ]; then
    echo "❌ Balance no sincronizado en Nodo 2"
    exit 1
fi
echo "✅ Balance sincronizado en Nodo 2: ${BALANCE_2}"

# Resumen
echo ""
echo "=========================================="
echo "✅ TEST COMPLETADO EXITOSAMENTE"
echo "=========================================="
echo ""
echo "Resumen:"
echo "  - Contrato desplegado en Nodo 1: ${CONTRACT_ADDRESS}"
echo "  - Contrato sincronizado en Nodo 2: ✅"
echo "  - Mint ejecutado en Nodo 1: ✅"
echo "  - Balance sincronizado en Nodo 2: ${BALANCE_2}"
echo ""
echo "🎉 La sincronización P2P de contratos funciona correctamente!"

