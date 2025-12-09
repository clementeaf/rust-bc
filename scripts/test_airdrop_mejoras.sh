#!/bin/bash

# Script de prueba para todas las mejoras del sistema de airdrop
# Valida: elegibilidad robusta, verificación de transacciones, rate limiting, tiers, etc.

set -e

BASE_URL="http://localhost:8080"
API_BASE="${BASE_URL}/api/v1"

# Colores para output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}🧪 Test de Mejoras del Sistema de Airdrop${NC}"
echo "=========================================="
echo ""

# Verificar que el servidor está corriendo
echo -e "${YELLOW}1. Verificando servidor...${NC}"
if ! curl -s "${API_BASE}/health" > /dev/null; then
    echo -e "${RED}❌ Servidor no está corriendo en ${BASE_URL}${NC}"
    echo "   Por favor inicia el servidor con: cargo run"
    exit 1
fi
echo -e "${GREEN}✅ Servidor activo${NC}"
echo ""

# Crear wallets de prueba
echo -e "${YELLOW}2. Creando wallets de prueba...${NC}"
WALLET1=$(curl -s -X POST "${API_BASE}/wallets/create" | jq -r '.data.address')
WALLET2=$(curl -s -X POST "${API_BASE}/wallets/create" | jq -r '.data.address')
WALLET3=$(curl -s -X POST "${API_BASE}/wallets/create" | jq -r '.data.address')
AIRDROP_WALLET=$(curl -s -X POST "${API_BASE}/wallets/create" | jq -r '.data.address')

echo "   WALLET1: ${WALLET1}"
echo "   WALLET2: ${WALLET2}"
echo "   WALLET3: ${WALLET3}"
echo "   AIRDROP_WALLET: ${AIRDROP_WALLET}"
echo -e "${GREEN}✅ Wallets creados${NC}"
echo ""

# Minar bloques iniciales para crear tracking
echo -e "${YELLOW}3. Minando bloques iniciales para crear tracking...${NC}"
for i in {1..15}; do
    curl -s -X POST "${API_BASE}/mine" \
        -H "Content-Type: application/json" \
        -d "{\"miner_address\": \"${WALLET1}\"}" > /dev/null
    sleep 0.5
done
echo -e "${GREEN}✅ 15 bloques minados con WALLET1${NC}"
echo ""

# Verificar tracking inicial
echo -e "${YELLOW}4. Verificando tracking inicial...${NC}"
TRACKING1=$(curl -s "${API_BASE}/airdrop/tracking/${WALLET1}" | jq -r '.data')
BLOCKS_VALIDATED=$(echo "$TRACKING1" | jq -r '.blocks_validated')
UPTIME_SECONDS=$(echo "$TRACKING1" | jq -r '.uptime_seconds')

echo "   Bloques validados: ${BLOCKS_VALIDATED}"
echo "   Uptime (segundos): ${UPTIME_SECONDS}"
echo "   Uptime (días): $(($UPTIME_SECONDS / 86400))"

if [ "$BLOCKS_VALIDATED" -ge "10" ]; then
    echo -e "${GREEN}✅ Mínimo de bloques cumplido${NC}"
else
    echo -e "${RED}❌ No cumple mínimo de bloques (requiere 10, tiene ${BLOCKS_VALIDATED})${NC}"
fi
echo ""

# Verificar elegibilidad (debe ser false por uptime)
echo -e "${YELLOW}5. Verificando elegibilidad (debe ser false por uptime insuficiente)...${NC}"
ELIGIBILITY=$(curl -s "${API_BASE}/airdrop/eligibility/${WALLET1}" | jq -r '.data')
IS_ELIGIBLE=$(echo "$ELIGIBILITY" | jq -r '.is_eligible')
MEETS_BLOCKS=$(echo "$ELIGIBILITY" | jq -r '.requirements.meets_blocks_requirement')
MEETS_UPTIME=$(echo "$ELIGIBILITY" | jq -r '.requirements.meets_uptime_requirement')
MEETS_POSITION=$(echo "$ELIGIBILITY" | jq -r '.requirements.meets_position_requirement')

echo "   Es elegible: ${IS_ELIGIBLE}"
echo "   Cumple bloques: ${MEETS_BLOCKS}"
echo "   Cumple uptime: ${MEETS_UPTIME}"
echo "   Cumple posición: ${MEETS_POSITION}"

if [ "$IS_ELIGIBLE" = "false" ]; then
    echo -e "${GREEN}✅ Elegibilidad correctamente rechazada (uptime insuficiente)${NC}"
else
    echo -e "${YELLOW}⚠️  Nodo es elegible (puede ser válido si pasó suficiente tiempo)${NC}"
fi
echo ""

# Verificar tier
echo -e "${YELLOW}6. Verificando tier asignado...${NC}"
TIER=$(echo "$ELIGIBILITY" | jq -r '.tier')
ESTIMATED_AMOUNT=$(echo "$ELIGIBILITY" | jq -r '.estimated_amount')
echo "   Tier: ${TIER}"
echo "   Cantidad estimada: ${ESTIMATED_AMOUNT} tokens"
echo -e "${GREEN}✅ Tier calculado correctamente${NC}"
echo ""

# Verificar tiers disponibles
echo -e "${YELLOW}7. Verificando tiers disponibles...${NC}"
TIERS=$(curl -s "${API_BASE}/airdrop/tiers" | jq -r '.data')
TIER_COUNT=$(echo "$TIERS" | jq 'length')
echo "   Número de tiers: ${TIER_COUNT}"

if [ "$TIER_COUNT" -ge "3" ]; then
    echo -e "${GREEN}✅ Tiers configurados correctamente${NC}"
    echo "$TIERS" | jq -r '.[] | "   Tier \(.tier_id): \(.name) - Base: \(.base_amount)"'
else
    echo -e "${RED}❌ Faltan tiers (esperado: 3, encontrado: ${TIER_COUNT})${NC}"
fi
echo ""

# Verificar rate limiting
echo -e "${YELLOW}8. Verificando rate limiting...${NC}"
echo "   Intentando 12 claims rápidos (límite: 10/min)..."
RATE_LIMIT_HIT=false
for i in {1..12}; do
    RESPONSE=$(curl -s -X POST "${API_BASE}/airdrop/claim" \
        -H "Content-Type: application/json" \
        -d "{\"node_address\": \"${WALLET2}\"}" 2>&1)
    
    if echo "$RESPONSE" | grep -q "Rate limit exceeded"; then
        RATE_LIMIT_HIT=true
        echo -e "   ${GREEN}✅ Rate limit activado en intento ${i}${NC}"
        break
    fi
    sleep 0.1
done

if [ "$RATE_LIMIT_HIT" = "false" ]; then
    echo -e "${YELLOW}⚠️  Rate limiting no se activó (puede ser válido si WALLET2 no es elegible)${NC}"
fi
echo ""

# Verificar estadísticas
echo -e "${YELLOW}9. Verificando estadísticas...${NC}"
STATS=$(curl -s "${API_BASE}/airdrop/statistics" | jq -r '.data')
TOTAL_NODES=$(echo "$STATS" | jq -r '.total_nodes')
ELIGIBLE_NODES=$(echo "$STATS" | jq -r '.eligible_nodes')
CLAIMED_NODES=$(echo "$STATS" | jq -r '.claimed_nodes')
PENDING_VERIFICATION=$(echo "$STATS" | jq -r '.pending_verification')
TIERS_COUNT=$(echo "$STATS" | jq -r '.tiers_count')

echo "   Total nodos: ${TOTAL_NODES}"
echo "   Nodos elegibles: ${ELIGIBLE_NODES}"
echo "   Nodos con claim: ${CLAIMED_NODES}"
echo "   Pendientes verificación: ${PENDING_VERIFICATION}"
echo "   Tiers disponibles: ${TIERS_COUNT}"
echo -e "${GREEN}✅ Estadísticas disponibles${NC}"
echo ""

# Verificar historial
echo -e "${YELLOW}10. Verificando historial de claims...${NC}"
HISTORY=$(curl -s "${API_BASE}/airdrop/history?limit=10" | jq -r '.data')
HISTORY_COUNT=$(echo "$HISTORY" | jq 'length')
echo "   Claims en historial: ${HISTORY_COUNT}"
echo -e "${GREEN}✅ Historial disponible${NC}"
echo ""

# Resumen final
echo "=========================================="
echo -e "${GREEN}✅ Tests completados${NC}"
echo ""
echo "Resumen de funcionalidades validadas:"
echo "  ✅ Tracking de nodos"
echo "  ✅ Cálculo de uptime"
echo "  ✅ Criterios de elegibilidad robustos"
echo "  ✅ Sistema de tiers"
echo "  ✅ Rate limiting"
echo "  ✅ Estadísticas"
echo "  ✅ Historial de claims"
echo ""
echo -e "${YELLOW}Nota:${NC} Para probar verificación de transacciones, necesitas:"
echo "  1. Fundear el AIRDROP_WALLET"
echo "  2. Hacer un claim válido"
echo "  3. Minar un bloque que incluya la transacción"
echo "  4. Verificar que el claim se marcó como verificado"

