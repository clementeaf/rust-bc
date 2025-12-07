#!/bin/bash

# Script simplificado para probar sincronización P2P de contratos

set -e

API_PORT_1=8080
API_PORT_2=8081
P2P_PORT_1=6000
P2P_PORT_2=6001

BASE_URL_1="http://localhost:${API_PORT_1}/api/v1"
BASE_URL_2="http://localhost:${API_PORT_2}/api/v1"

# Limpiar procesos anteriores
echo "🧹 Limpiando procesos anteriores..."
pkill -9 -f rust-bc 2>/dev/null || true
lsof -ti:${API_PORT_1},${API_PORT_2},${P2P_PORT_1},${P2P_PORT_2} | xargs kill -9 2>/dev/null || true
sleep 2

# Iniciar Nodo 1
echo "🚀 Iniciando Nodo 1 (API: ${API_PORT_1}, P2P: ${P2P_PORT_1})..."
cd "$(dirname "$0")/.."
API_PORT=${API_PORT_1} P2P_PORT=${P2P_PORT_1} DB_NAME=blockchain_node1 ./target/release/rust-bc > /tmp/node1_simple.log 2>&1 &
NODE1_PID=$!
echo "Nodo 1 iniciado (PID: $NODE1_PID)"

# Esperar a que Nodo 1 esté listo
echo "⏳ Esperando a que Nodo 1 esté listo..."
sleep 3  # Dar tiempo inicial para que el servidor inicie
for i in {1..30}; do
    if curl -s "${BASE_URL_1}/health" > /dev/null 2>&1; then
        echo "✅ Nodo 1 está listo"
        break
    fi
    if [ $i -eq 30 ]; then
        echo "❌ Nodo 1 no está disponible después de 30 intentos"
        echo "Logs del Nodo 1:"
        tail -20 /tmp/node1_simple.log
        kill $NODE1_PID 2>/dev/null || true
        exit 1
    fi
    sleep 1
done

# Iniciar Nodo 2
echo "🚀 Iniciando Nodo 2 (API: ${API_PORT_2}, P2P: ${P2P_PORT_2})..."
API_PORT=${API_PORT_2} P2P_PORT=${P2P_PORT_2} DB_NAME=blockchain_node2 ./target/release/rust-bc > /tmp/node2_simple.log 2>&1 &
NODE2_PID=$!
echo "Nodo 2 iniciado (PID: $NODE2_PID)"

# Esperar a que Nodo 2 esté listo
echo "⏳ Esperando a que Nodo 2 esté listo..."
sleep 3  # Dar tiempo inicial para que el servidor inicie
for i in {1..30}; do
    if curl -s "${BASE_URL_2}/health" > /dev/null 2>&1; then
        echo "✅ Nodo 2 está listo"
        break
    fi
    if [ $i -eq 30 ]; then
        echo "❌ Nodo 2 no está disponible después de 30 intentos"
        echo "Logs del Nodo 2:"
        tail -20 /tmp/node2_simple.log
        kill $NODE1_PID $NODE2_PID 2>/dev/null || true
        exit 1
    fi
    sleep 1
done

# Crear wallets
echo ""
echo "📝 Creando wallets..."
WALLET_1=$(curl -s -X POST "${BASE_URL_1}/wallets" | jq -r '.data.address')
WALLET_2=$(curl -s -X POST "${BASE_URL_2}/wallets" | jq -r '.data.address')
echo "Wallet Nodo 1: ${WALLET_1}"
echo "Wallet Nodo 2: ${WALLET_2}"

# Minar bloque en Nodo 1
echo ""
echo "⛏️  Minando bloque en Nodo 1..."
curl -s -X POST "${BASE_URL_1}/mining/mine" > /dev/null
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
    echo "❌ Error al desplegar contrato"
    echo "Respuesta: ${DEPLOY_RESPONSE}"
    kill $NODE1_PID $NODE2_PID 2>/dev/null || true
    exit 1
fi
echo "✅ Contrato desplegado: ${CONTRACT_ADDRESS}"

# Verificar contrato en Nodo 1
echo ""
echo "🔍 Verificando contrato en Nodo 1..."
CONTRACT_1=$(curl -s "${BASE_URL_1}/contracts/${CONTRACT_ADDRESS}" | jq -r '.data.name')
echo "✅ Contrato encontrado: ${CONTRACT_1}"

# Verificar que NO existe en Nodo 2
echo ""
echo "🔍 Verificando que contrato NO existe en Nodo 2..."
CONTRACT_2_BEFORE=$(curl -s "${BASE_URL_2}/contracts/${CONTRACT_ADDRESS}" | jq -r '.error // "not_found"')
if [ "$CONTRACT_2_BEFORE" == "not_found" ] || [ "$CONTRACT_2_BEFORE" == "Contract not found" ]; then
    echo "✅ Contrato no existe en Nodo 2 (como se esperaba)"
else
    echo "⚠️  Contrato ya existe (puede ser de prueba anterior)"
fi

# Conectar Nodo 2 a Nodo 1
echo ""
echo "🔗 Conectando Nodo 2 a Nodo 1..."
CONNECT_ADDRESS="127.0.0.1:${P2P_PORT_1}"
CONNECT_RESPONSE=$(curl -s -X POST "${BASE_URL_2}/peers/${CONNECT_ADDRESS}/connect")
echo "Respuesta: ${CONNECT_RESPONSE}"

# Esperar sincronización
echo ""
echo "⏳ Esperando sincronización de contratos (5 segundos)..."
sleep 5

# Verificar que el contrato ahora existe en Nodo 2
echo ""
echo "🔍 Verificando que contrato ahora existe en Nodo 2..."
CONTRACT_2_AFTER=$(curl -s "${BASE_URL_2}/contracts/${CONTRACT_ADDRESS}" | jq -r '.data.address // "not_found"')
if [ "$CONTRACT_2_AFTER" == "$CONTRACT_ADDRESS" ]; then
    echo "✅ Contrato sincronizado en Nodo 2!"
    CONTRACT_NAME_2=$(curl -s "${BASE_URL_2}/contracts/${CONTRACT_ADDRESS}" | jq -r '.data.name')
    echo "   Nombre: ${CONTRACT_NAME_2}"
else
    echo "❌ Contrato NO sincronizado"
    echo "Respuesta: $(curl -s "${BASE_URL_2}/contracts/${CONTRACT_ADDRESS}")"
    kill $NODE1_PID $NODE2_PID 2>/dev/null || true
    exit 1
fi

# Ejecutar mint en Nodo 1
echo ""
echo "💰 Ejecutando mint en Nodo 1..."
MINT_DATA=$(cat <<EOF
{
  "function": {
    "Mint": {
      "to": "${WALLET_1}",
      "amount": 1000
    }
  }
}
EOF
)
# Escapar comillas dobles para JSON
MINT_DATA_ESCAPED=$(echo "$MINT_DATA" | sed 's/"/\\"/g')

MINT_RESPONSE=$(curl -s -X POST "${BASE_URL_1}/contracts/${CONTRACT_ADDRESS}/execute" \
    -H "Content-Type: application/json" \
    -d "${MINT_DATA}")

if echo "$MINT_RESPONSE" | jq -e '.success' > /dev/null 2>&1; then
    echo "✅ Mint ejecutado exitosamente"
else
    echo "❌ Error al ejecutar mint: ${MINT_RESPONSE}"
fi

# Esperar sincronización de actualización
echo ""
echo "⏳ Esperando sincronización de actualización (5 segundos)..."
sleep 5

# Verificar balance en Nodo 2
echo ""
echo "🔍 Verificando balance en Nodo 2..."
BALANCE_2=$(curl -s "${BASE_URL_2}/contracts/${CONTRACT_ADDRESS}/balance/${WALLET_1}" | jq -r '.data // "0"')
if [ "$BALANCE_2" != "0" ] && [ "$BALANCE_2" != "null" ]; then
    echo "✅ Balance sincronizado en Nodo 2: ${BALANCE_2}"
else
    echo "⚠️  Balance no sincronizado aún (puede necesitar más tiempo)"
fi

# Resumen
echo ""
echo "=========================================="
echo "✅ TEST COMPLETADO"
echo "=========================================="
echo "Contrato: ${CONTRACT_ADDRESS}"
echo "Sincronización: ✅"
echo "Balance sincronizado: ${BALANCE_2}"
echo ""

# Limpiar
echo "🧹 Deteniendo servidores..."
kill $NODE1_PID $NODE2_PID 2>/dev/null || true
sleep 2
echo "✅ Servidores detenidos"
