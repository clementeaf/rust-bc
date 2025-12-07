#!/bin/bash

# Prueba detallada de sincronización de contratos

API_PORT_1=20000
API_PORT_2=20001
P2P_PORT_1=20002
P2P_PORT_2=20003

BASE_URL_1="http://localhost:${API_PORT_1}/api/v1"
BASE_URL_2="http://localhost:${API_PORT_2}/api/v1"

echo "🧪 Prueba Detallada de Sincronización P2P de Contratos"
echo "======================================================"

# Limpiar
pkill -9 -f "rust-bc.*20000" 2>/dev/null || true
pkill -9 -f "rust-bc.*20001" 2>/dev/null || true
sleep 2
rm -f test_node*.db*

# Iniciar nodos
echo ""
echo "🚀 Iniciando nodos..."
cargo run --release -- ${API_PORT_1} ${P2P_PORT_1} test_node1 > /tmp/node1_detailed.log 2>&1 &
NODE1_PID=$!
cargo run --release -- ${API_PORT_2} ${P2P_PORT_2} test_node2 > /tmp/node2_detailed.log 2>&1 &
NODE2_PID=$!

# Esperar
echo "⏳ Esperando servidores..."
for i in {1..30}; do
    if curl -s "${BASE_URL_1}/health" > /dev/null 2>&1 && \
       curl -s "${BASE_URL_2}/health" > /dev/null 2>&1; then
        echo "✅ Servidores listos"
        break
    fi
    sleep 1
done
sleep 2

# Crear wallets
WALLET_1=$(curl -s -X POST "${BASE_URL_1}/wallets/create" | jq -r '.data.address')
WALLET_2=$(curl -s -X POST "${BASE_URL_2}/wallets/create" | jq -r '.data.address')
echo "Wallet 1: ${WALLET_1}"
echo "Wallet 2: ${WALLET_2}"

# Minar
curl -s -X POST "${BASE_URL_1}/mine" -H "Content-Type: application/json" -d "{\"miner_address\": \"${WALLET_1}\"}" > /dev/null

# Desplegar contrato
CONTRACT=$(curl -s -X POST "${BASE_URL_1}/contracts" \
    -H "Content-Type: application/json" \
    -d "{\"owner\": \"${WALLET_1}\", \"contract_type\": \"token\", \"name\": \"TestToken\", \"symbol\": \"TEST\", \"total_supply\": 1000000, \"decimals\": 18}" \
    | jq -r '.data')
echo "Contrato: ${CONTRACT}"

# Verificar que no existe en Nodo 2
echo ""
echo "🔍 Verificando que NO existe en Nodo 2..."
curl -s "${BASE_URL_2}/contracts/${CONTRACT}" | jq -r '.message // "existe"'

# Conectar
echo ""
echo "🔗 Conectando..."
curl -s -X POST "${BASE_URL_2}/peers/127.0.0.1:${P2P_PORT_1}/connect" > /dev/null
sleep 5

# Verificar sincronización
echo ""
echo "🔍 Verificando sincronización..."
CONTRACT_2=$(curl -s "${BASE_URL_2}/contracts/${CONTRACT}" | jq -r '.data.address // "no encontrado"')
echo "Contrato en Nodo 2: ${CONTRACT_2}"

# Verificar peers conectados
echo ""
echo "🔍 Verificando peers conectados..."
PEERS_1=$(curl -s "${BASE_URL_1}/peers" | jq -r '.data | length')
PEERS_2=$(curl -s "${BASE_URL_2}/peers" | jq -r '.data | length')
echo "Peers en Nodo 1: ${PEERS_1}"
echo "Peers en Nodo 2: ${PEERS_2}"

# Ejecutar mint
echo ""
echo "💰 Ejecutando mint..."
MINT_RESULT=$(curl -s -X POST "${BASE_URL_1}/contracts/${CONTRACT}/execute" \
    -H "Content-Type: application/json" \
    -d "{\"function\": \"mint\", \"params\": {\"to\": \"${WALLET_1}\", \"amount\": 1000}}")
echo "Mint resultado: $(echo $MINT_RESULT | jq -r '.success')"

# Esperar y verificar balance
echo ""
echo "⏳ Esperando sincronización de actualización..."
sleep 10

echo ""
echo "🔍 Verificando balance en Nodo 1..."
BALANCE_1=$(curl -s "${BASE_URL_1}/contracts/${CONTRACT}/balance/${WALLET_1}" | jq -r '.data')
echo "Balance Nodo 1: ${BALANCE_1}"

echo ""
echo "🔍 Verificando balance en Nodo 2..."
BALANCE_2=$(curl -s "${BASE_URL_2}/contracts/${CONTRACT}/balance/${WALLET_1}" | jq -r '.data')
echo "Balance Nodo 2: ${BALANCE_2}"

# Verificar update_sequence
echo ""
echo "🔍 Verificando update_sequence..."
SEQ_1=$(curl -s "${BASE_URL_1}/contracts/${CONTRACT}" | jq -r '.data.update_sequence')
SEQ_2=$(curl -s "${BASE_URL_2}/contracts/${CONTRACT}" | jq -r '.data.update_sequence')
echo "Update sequence Nodo 1: ${SEQ_1}"
echo "Update sequence Nodo 2: ${SEQ_2}"

# Logs
echo ""
echo "📋 Últimos logs Nodo 1 (contratos):"
tail -20 /tmp/node1_detailed.log | grep -E "(contrato|broadcast|UpdateContract|mint)" || tail -5 /tmp/node1_detailed.log

echo ""
echo "📋 Últimos logs Nodo 2 (contratos):"
tail -20 /tmp/node2_detailed.log | grep -E "(contrato|UpdateContract|actualizado|mint)" || tail -5 /tmp/node2_detailed.log

# Limpiar
pkill -9 -P $NODE1_PID 2>/dev/null || true
pkill -9 -P $NODE2_PID 2>/dev/null || true
pkill -9 -f "rust-bc.*20000" 2>/dev/null || true
pkill -9 -f "rust-bc.*20001" 2>/dev/null || true

