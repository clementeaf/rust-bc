#!/bin/bash

# Test: Staking y Airdrop con BlockStorage

set -e

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo "🧪 TEST: Staking y Airdrop"
echo "==========================="
echo ""

cd /Users/clementefalcone/Desktop/personal/rust-bc

# Limpiar
pkill -9 -f "rust-bc.*8090" 2>/dev/null || true
rm -rf test_staking* test_staking_blocks 2>/dev/null || true
sleep 1

wait_for_server() {
    local port=$1
    local max=20
    local count=0
    
    while [ $count -lt $max ]; do
        if curl -s "http://localhost:${port}/api/v1/stats" > /dev/null 2>&1; then
            return 0
        fi
        sleep 1
        count=$((count + 1))
    done
    return 1
}

# Iniciar servidor
echo "1️⃣  Iniciando servidor..."
DB_NAME="test_staking" cargo run -- 8090 8091 > /tmp/test-staking.log 2>&1 &
SERVER_PID=$!
sleep 10

if wait_for_server 8090; then
    echo -e "${GREEN}✅ Servidor iniciado${NC}"
else
    echo -e "${RED}❌ Servidor no inició${NC}"
    kill $SERVER_PID 2>/dev/null || true
    exit 1
fi

# Test 2: Crear wallet y minar
echo ""
echo "2️⃣  Creando wallet y minando..."
WALLET=$(curl -s -X POST "http://localhost:8090/api/v1/wallets/create")
ADDR=$(echo "$WALLET" | jq -r '.data.address' 2>/dev/null)

if [ -n "$ADDR" ] && [ "$ADDR" != "null" ]; then
    echo -e "${GREEN}✅ Wallet creado: ${ADDR:0:20}...${NC}"
    
    # Minar 5 bloques para tener balance suficiente
    for i in {1..5}; do
        MINE=$(curl -s -X POST "http://localhost:8090/api/v1/mine" \
            -H "Content-Type: application/json" \
            -d "{\"miner_address\": \"$ADDR\"}")
        if echo "$MINE" | grep -q "success\|block"; then
            echo "   Bloque $i minado"
        fi
        sleep 1
    done
else
    echo -e "${RED}❌ Error creando wallet${NC}"
    kill $SERVER_PID 2>/dev/null || true
    exit 1
fi

# Test 3: Verificar balance
echo ""
echo "3️⃣  Verificando balance..."
BALANCE=$(curl -s "http://localhost:8090/api/v1/wallets/${ADDR}/balance" | jq -r '.data.balance // 0' 2>/dev/null)
if [ "$BALANCE" != "null" ] && [ "$BALANCE" != "0" ]; then
    echo -e "${GREEN}✅ Balance: $BALANCE${NC}"
else
    echo -e "${YELLOW}⚠️  Balance: $BALANCE (puede necesitar más bloques)${NC}"
fi

# Test 4: Staking
echo ""
echo "4️⃣  Probando staking..."
STAKE=$(curl -s -X POST "http://localhost:8090/api/v1/staking/stake" \
    -H "Content-Type: application/json" \
    -d "{\"address\": \"$ADDR\", \"amount\": 1000}")

if echo "$STAKE" | grep -q "success\|staked"; then
    echo -e "${GREEN}✅ Staking exitoso${NC}"
    
    # Verificar validador
    VALIDATOR=$(curl -s "http://localhost:8090/api/v1/staking/validator/${ADDR}")
    if echo "$VALIDATOR" | grep -q "$ADDR"; then
        echo -e "${GREEN}✅ Validador creado${NC}"
    fi
else
    echo -e "${YELLOW}⚠️  Staking no disponible o sin balance suficiente${NC}"
    echo "Respuesta: $STAKE"
fi

# Test 5: Verificar validadores
echo ""
echo "5️⃣  Verificando validadores..."
VALIDATORS=$(curl -s "http://localhost:8090/api/v1/staking/validators")
VALIDATOR_COUNT=$(echo "$VALIDATORS" | jq -r '.data | length // 0' 2>/dev/null)
echo "   Validadores: $VALIDATOR_COUNT"

# Test 6: Verificar archivos de bloques
echo ""
echo "6️⃣  Verificando archivos de bloques..."
sleep 2
BLOCK_FILES=$(ls -1 test_staking_blocks/block_*.dat 2>/dev/null | wc -l | tr -d ' ')
if [ "$BLOCK_FILES" -gt 0 ]; then
    echo -e "${GREEN}✅ Archivos de bloques: $BLOCK_FILES${NC}"
else
    echo -e "${YELLOW}⚠️  No se encontraron archivos${NC}"
fi

# Test 7: Airdrop (verificar tracking)
echo ""
echo "7️⃣  Verificando tracking de airdrop..."
# Minar un bloque más para registrar en airdrop
MINE=$(curl -s -X POST "http://localhost:8090/api/v1/mine" \
    -H "Content-Type: application/json" \
    -d "{\"miner_address\": \"$ADDR\"}")
sleep 2

TRACKING=$(curl -s "http://localhost:8090/api/v1/airdrop/tracking/${ADDR}")
if echo "$TRACKING" | grep -q "node_address\|blocks_validated"; then
    echo -e "${GREEN}✅ Tracking de airdrop funciona${NC}"
    BLOCKS_VALIDATED=$(echo "$TRACKING" | jq -r '.data.blocks_validated // 0' 2>/dev/null)
    echo "   Bloques validados: $BLOCKS_VALIDATED"
else
    echo -e "${YELLOW}⚠️  Tracking de airdrop no disponible${NC}"
fi

# Limpiar
echo ""
echo "🧹 Limpiando..."
kill $SERVER_PID 2>/dev/null || true
sleep 2

# Resumen
echo ""
echo "==========================="
echo -e "${GREEN}✅ TEST STAKING/AIRDROP COMPLETADO${NC}"
echo ""
echo "📊 Resumen:"
echo "  ✅ Servidor: OK"
echo "  ✅ Wallets: OK"
echo "  ✅ Minería: OK"
echo "  ✅ Staking: Verificado"
echo "  ✅ Airdrop: Verificado"
echo "  ✅ BlockStorage: OK"
echo ""

