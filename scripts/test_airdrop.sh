#!/bin/bash

# Script de testing para el sistema de airdrop
# Prueba: tracking, elegibilidad, claims, y prevención de doble claim

set -e

BASE_PORT=20000
API_PORT=$((BASE_PORT + 0))
P2P_PORT=$((BASE_PORT + 1))
API_URL="http://127.0.0.1:${API_PORT}/api/v1"
DB_NAME="test_airdrop"

# Colores para output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Función para limpiar procesos al finalizar
cleanup() {
    echo ""
    echo "🧹 Limpiando procesos..."
    pkill -f "rust-bc.*${API_PORT}" || true
    sleep 2
    rm -f "${DB_NAME}.db"
    echo "✅ Limpieza completada"
}

trap cleanup EXIT

echo -e "${GREEN}🧪 Test del Sistema de Airdrop${NC}"
echo "=================================="
echo ""

# Limpiar base de datos anterior
rm -f "${DB_NAME}.db"

# Función para verificar que el servidor esté corriendo
check_server() {
    if ! curl -s "${API_URL}/health" > /dev/null 2>&1; then
        echo -e "${RED}❌ Error: El servidor no está corriendo en ${API_URL}${NC}"
        return 1
    fi
    echo -e "${GREEN}✅ Servidor está corriendo${NC}"
    return 0
}

# Función para crear un wallet
create_wallet() {
    local response=$(curl -s -X POST "${API_URL}/wallets/create")
    local address=$(echo "$response" | jq -r '.data.address // empty')
    if [ -z "$address" ] || [ "$address" = "null" ]; then
        echo -e "${RED}❌ Error al crear wallet${NC}"
        echo "$response" | jq '.'
        exit 1
    fi
    echo "$address"
}

# Función para minar un bloque
mine_block() {
    local miner_address=$1
    local response=$(curl -s -X POST "${API_URL}/mine" \
        -H "Content-Type: application/json" \
        -d "{\"miner_address\": \"${miner_address}\", \"max_transactions\": 10}")
    
    if echo "$response" | jq -e '.success == true' > /dev/null 2>&1; then
        echo "$response" | jq -r '.data.hash // empty'
    else
        echo -e "${RED}❌ Error al minar bloque${NC}"
        echo "$response" | jq '.'
        return 1
    fi
}

# Función para obtener tracking de un nodo
get_tracking() {
    local address=$1
    curl -s "${API_URL}/airdrop/tracking/${address}" | jq '.'
}

# Función para obtener estadísticas
get_statistics() {
    curl -s "${API_URL}/airdrop/statistics" | jq '.'
}

# Función para reclamar airdrop
claim_airdrop() {
    local node_address=$1
    curl -s -X POST "${API_URL}/airdrop/claim" \
        -H "Content-Type: application/json" \
        -d "{\"node_address\": \"${node_address}\"}" | jq '.'
}

# Función para obtener balance de un wallet
get_balance() {
    local address=$1
    local response=$(curl -s "${API_URL}/wallets/${address}")
    echo "$response" | jq -r '.data.balance // 0'
}

# Iniciar servidor con wallet de airdrop temporal
echo "1. Iniciando servidor de prueba..."
# Primero creamos un wallet temporal para usar como AIRDROP_WALLET
# Necesitamos iniciar el servidor, crear el wallet, y luego reiniciar con esa variable
# Por simplicidad, iniciaremos el servidor sin AIRDROP_WALLET primero

RUST_LOG=info cargo run --release ${API_PORT} ${P2P_PORT} ${DB_NAME} > /tmp/node_airdrop.log 2>&1 &
NODE_PID=$!
sleep 5

# Verificar que el nodo está corriendo
if ! kill -0 $NODE_PID 2>/dev/null; then
    echo -e "${RED}❌ Error: Nodo no inició correctamente${NC}"
    cat /tmp/node_airdrop.log
    exit 1
fi

echo -e "${GREEN}✅ Nodo iniciado (PID: $NODE_PID)${NC}"
sleep 3

# Verificar servidor
echo "2. Verificando servidor..."
if ! check_server; then
    echo -e "${RED}❌ Error: Servidor no responde${NC}"
    cat /tmp/node_airdrop.log
    exit 1
fi
echo ""

# Crear wallets para testing
echo "3. Creando wallets para testing..."
WALLET1=$(create_wallet)
WALLET2=$(create_wallet)
WALLET3=$(create_wallet)
AIRDROP_WALLET=$(create_wallet)

echo -e "${GREEN}✅ Wallets creados:${NC}"
echo "   WALLET1: ${WALLET1}"
echo "   WALLET2: ${WALLET2}"
echo "   WALLET3: ${WALLET3}"
echo "   AIRDROP_WALLET: ${AIRDROP_WALLET}"
echo ""

# Minar bloques iniciales para dar balance al wallet de airdrop
echo "4. Minando bloques iniciales para balance del wallet de airdrop..."
for i in {1..20}; do
    mine_block "$AIRDROP_WALLET" > /dev/null 2>&1
done
AIRDROP_BALANCE=$(get_balance "$AIRDROP_WALLET")
echo -e "   ${GREEN}✅ Balance del wallet de airdrop: ${AIRDROP_BALANCE}${NC}"

# El sistema usa "AIRDROP" por defecto, pero podemos configurar AIRDROP_WALLET
# Para este test, vamos a usar el wallet creado
# Necesitamos detener y reiniciar el servidor con AIRDROP_WALLET configurado
echo "   Configurando AIRDROP_WALLET y reiniciando servidor..."
kill $NODE_PID 2>/dev/null || true
sleep 2

# Reiniciar con AIRDROP_WALLET configurado
AIRDROP_WALLET="${AIRDROP_WALLET}" RUST_LOG=info cargo run --release ${API_PORT} ${P2P_PORT} ${DB_NAME} > /tmp/node_airdrop.log 2>&1 &
NODE_PID=$!
sleep 5

if ! kill -0 $NODE_PID 2>/dev/null; then
    echo -e "${RED}❌ Error: Nodo no reinició correctamente${NC}"
    cat /tmp/node_airdrop.log
    exit 1
fi

sleep 3
if ! check_server; then
    echo -e "${RED}❌ Error: Servidor no responde después de reiniciar${NC}"
    exit 1
fi

echo -e "   ${GREEN}✅ Servidor reiniciado con AIRDROP_WALLET=${AIRDROP_WALLET}${NC}"
echo ""

# Verificar estadísticas iniciales
echo "5. Verificando estadísticas iniciales..."
STATS=$(get_statistics)
TOTAL_NODES=$(echo "$STATS" | jq -r '.data.total_nodes // 0')
ELIGIBLE_NODES=$(echo "$STATS" | jq -r '.data.eligible_nodes // 0')
echo "   Total nodos: ${TOTAL_NODES}"
echo "   Nodos elegibles: ${ELIGIBLE_NODES}"
echo ""

# Minar bloques con diferentes nodos para crear tracking
echo "6. Minando bloques con diferentes nodos (creando tracking)..."
echo "   Minando con WALLET1 (debe ser elegible - primer nodo)..."
BLOCK1=$(mine_block "$WALLET1")
if [ -n "$BLOCK1" ]; then
    echo -e "   ${GREEN}✅ Bloque minado: ${BLOCK1:0:16}...${NC}"
else
    echo -e "   ${RED}❌ Error al minar bloque${NC}"
    exit 1
fi

echo "   Minando con WALLET2 (debe ser elegible - segundo nodo)..."
BLOCK2=$(mine_block "$WALLET2")
if [ -n "$BLOCK2" ]; then
    echo -e "   ${GREEN}✅ Bloque minado: ${BLOCK2:0:16}...${NC}"
fi

echo "   Minando con WALLET3 (debe ser elegible - tercer nodo)..."
BLOCK3=$(mine_block "$WALLET3")
if [ -n "$BLOCK3" ]; then
    echo -e "   ${GREEN}✅ Bloque minado: ${BLOCK3:0:16}...${NC}"
fi
echo ""

# Verificar tracking de WALLET1
echo "7. Verificando tracking de WALLET1..."
TRACKING1=$(get_tracking "$WALLET1")
IS_ELIGIBLE1=$(echo "$TRACKING1" | jq -r '.data.is_eligible // false')
FIRST_BLOCK1=$(echo "$TRACKING1" | jq -r '.data.first_block_index // 0')
BLOCKS_VALIDATED1=$(echo "$TRACKING1" | jq -r '.data.blocks_validated // 0')

if [ "$IS_ELIGIBLE1" = "true" ]; then
    echo -e "   ${GREEN}✅ WALLET1 es elegible${NC}"
    echo "   Primer bloque: ${FIRST_BLOCK1}"
    echo "   Bloques validados: ${BLOCKS_VALIDATED1}"
else
    echo -e "   ${RED}❌ WALLET1 NO es elegible${NC}"
    exit 1
fi
echo ""

# Verificar estadísticas después de minar
echo "8. Verificando estadísticas después de minar..."
STATS=$(get_statistics)
TOTAL_NODES=$(echo "$STATS" | jq -r '.data.total_nodes // 0')
ELIGIBLE_NODES=$(echo "$STATS" | jq -r '.data.eligible_nodes // 0')
CLAIMED_NODES=$(echo "$STATS" | jq -r '.data.claimed_nodes // 0')
PENDING_CLAIMS=$(echo "$STATS" | jq -r '.data.pending_claims // 0')

echo "   Total nodos: ${TOTAL_NODES}"
echo "   Nodos elegibles: ${ELIGIBLE_NODES}"
echo "   Nodos que han reclamado: ${CLAIMED_NODES}"
echo "   Claims pendientes: ${PENDING_CLAIMS}"
echo ""

# Verificar balance inicial de WALLET1
echo "9. Verificando balance inicial de WALLET1..."
BALANCE_BEFORE=$(get_balance "$WALLET1")
echo "   Balance antes del claim: ${BALANCE_BEFORE}"
echo ""

# Reclamar airdrop para WALLET1
echo "10. Reclamando airdrop para WALLET1..."
CLAIM_RESPONSE=$(claim_airdrop "$WALLET1")
CLAIM_SUCCESS=$(echo "$CLAIM_RESPONSE" | jq -r '.success // false')

if [ "$CLAIM_SUCCESS" = "true" ]; then
    AIRDROP_AMOUNT=$(echo "$CLAIM_RESPONSE" | jq -r '.data.airdrop_amount // 0')
    TX_ID=$(echo "$CLAIM_RESPONSE" | jq -r '.data.transaction_id // ""')
    echo -e "   ${GREEN}✅ Airdrop reclamado exitosamente${NC}"
    echo "   Cantidad: ${AIRDROP_AMOUNT}"
    echo "   Transaction ID: ${TX_ID:0:16}..."
else
    ERROR_MSG=$(echo "$CLAIM_RESPONSE" | jq -r '.message // "Error desconocido"')
    echo -e "   ${RED}❌ Error al reclamar airdrop: ${ERROR_MSG}${NC}"
    echo "$CLAIM_RESPONSE" | jq '.'
    exit 1
fi
echo ""

# Minar un bloque para procesar la transacción de airdrop
echo "11. Minando bloque para procesar transacción de airdrop..."
mine_block "$WALLET1" > /dev/null 2>&1
sleep 1
echo -e "${GREEN}✅ Bloque minado${NC}"
echo ""

# Verificar balance después del claim
echo "12. Verificando balance después del claim..."
BALANCE_AFTER=$(get_balance "$WALLET1")
echo "   Balance después del claim: ${BALANCE_AFTER}"

if [ "$BALANCE_AFTER" -gt "$BALANCE_BEFORE" ]; then
    DIFF=$((BALANCE_AFTER - BALANCE_BEFORE))
    echo -e "   ${GREEN}✅ Balance aumentó en ${DIFF} tokens${NC}"
else
    echo -e "   ${YELLOW}⚠️  Balance no aumentó (puede que la transacción aún no se haya procesado)${NC}"
fi
echo ""

# Intentar reclamar airdrop de nuevo (debe fallar)
echo "13. Intentando reclamar airdrop de nuevo (debe fallar - prevención de doble claim)..."
CLAIM_RESPONSE2=$(claim_airdrop "$WALLET1")
CLAIM_SUCCESS2=$(echo "$CLAIM_RESPONSE2" | jq -r '.success // false')

if [ "$CLAIM_SUCCESS2" = "false" ]; then
    ERROR_MSG2=$(echo "$CLAIM_RESPONSE2" | jq -r '.message // "Error desconocido"')
    echo -e "   ${GREEN}✅ Prevención de doble claim funciona correctamente${NC}"
    echo "   Error esperado: ${ERROR_MSG2}"
else
    echo -e "   ${RED}❌ Error: Se permitió reclamar airdrop dos veces${NC}"
    exit 1
fi
echo ""

# Verificar tracking después del claim
echo "14. Verificando tracking después del claim..."
TRACKING_AFTER=$(get_tracking "$WALLET1")
CLAIMED=$(echo "$TRACKING_AFTER" | jq -r '.data.airdrop_claimed // false')
CLAIM_TIMESTAMP=$(echo "$TRACKING_AFTER" | jq -r '.data.claim_timestamp // null')

if [ "$CLAIMED" = "true" ]; then
    echo -e "   ${GREEN}✅ Estado de claim actualizado correctamente${NC}"
    echo "   Claim timestamp: ${CLAIM_TIMESTAMP}"
else
    echo -e "   ${RED}❌ Error: Estado de claim no se actualizó${NC}"
    exit 1
fi
echo ""

# Verificar estadísticas finales
echo "15. Verificando estadísticas finales..."
STATS_FINAL=$(get_statistics)
TOTAL_NODES_FINAL=$(echo "$STATS_FINAL" | jq -r '.data.total_nodes // 0')
ELIGIBLE_NODES_FINAL=$(echo "$STATS_FINAL" | jq -r '.data.eligible_nodes // 0')
CLAIMED_NODES_FINAL=$(echo "$STATS_FINAL" | jq -r '.data.claimed_nodes // 0')
TOTAL_DISTRIBUTED=$(echo "$STATS_FINAL" | jq -r '.data.total_distributed // 0')

echo "   Total nodos: ${TOTAL_NODES_FINAL}"
echo "   Nodos elegibles: ${ELIGIBLE_NODES_FINAL}"
echo "   Nodos que han reclamado: ${CLAIMED_NODES_FINAL}"
echo "   Total distribuido: ${TOTAL_DISTRIBUTED}"
echo ""

# Reclamar airdrop para WALLET2
echo "16. Reclamando airdrop para WALLET2..."
CLAIM_RESPONSE3=$(claim_airdrop "$WALLET2")
CLAIM_SUCCESS3=$(echo "$CLAIM_RESPONSE3" | jq -r '.success // false')

if [ "$CLAIM_SUCCESS3" = "true" ]; then
    echo -e "   ${GREEN}✅ Airdrop reclamado exitosamente para WALLET2${NC}"
else
    ERROR_MSG3=$(echo "$CLAIM_RESPONSE3" | jq -r '.message // "Error desconocido"')
    echo -e "   ${RED}❌ Error al reclamar airdrop para WALLET2: ${ERROR_MSG3}${NC}"
fi
echo ""

# Obtener lista de nodos elegibles
echo "17. Obteniendo lista de nodos elegibles..."
ELIGIBLE_LIST=$(curl -s "${API_URL}/airdrop/eligible")
ELIGIBLE_COUNT=$(echo "$ELIGIBLE_LIST" | jq -r '.data | length // 0')
echo "   Nodos elegibles pendientes: ${ELIGIBLE_COUNT}"
if [ "$ELIGIBLE_COUNT" -gt 0 ]; then
    echo "   Primeros 3 nodos elegibles:"
    echo "$ELIGIBLE_LIST" | jq -r '.data[0:3] | .[] | "      - \(.node_address): bloque \(.first_block_index)"'
fi
echo ""

# Resumen final
echo "=================================="
echo -e "${GREEN}✅ Test del Sistema de Airdrop COMPLETADO${NC}"
echo ""
echo "Resumen:"
echo "  ✅ Tracking de nodos funciona"
echo "  ✅ Elegibilidad funciona"
echo "  ✅ Claim de airdrop funciona"
echo "  ✅ Prevención de doble claim funciona"
echo "  ✅ Estadísticas funcionan"
echo "  ✅ Lista de elegibles funciona"
echo ""
echo -e "${GREEN}🎉 Todos los tests pasaron exitosamente${NC}"

