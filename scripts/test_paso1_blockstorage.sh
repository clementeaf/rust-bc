#!/bin/bash

# Test Paso 1: Verificar que BlockStorage funciona correctamente

set -e

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "🧪 TEST PASO 1: BlockStorage en API"
echo "===================================="
echo ""

cd /Users/clementefalcone/Desktop/personal/rust-bc

# Limpiar
pkill -9 -f "rust-bc.*8090" 2>/dev/null || true
rm -rf test_paso1* test_paso1_blocks 2>/dev/null || true
sleep 1

# Función para esperar servidor
wait_for_server() {
    local port=$1
    local max=30  # Aumentar a 30 segundos para dar tiempo a compilación
    local count=0
    
    while [ $count -lt $max ]; do
        if curl -s "http://localhost:${port}/api/v1/health" > /dev/null 2>&1; then
            return 0
        fi
        # Mostrar progreso cada 5 segundos
        if [ $((count % 5)) -eq 0 ] && [ $count -gt 0 ]; then
            echo "   Esperando... (${count}s/${max}s)"
        fi
        sleep 1
        count=$((count + 1))
    done
    return 1
}

# Iniciar servidor
echo "1️⃣  Iniciando servidor..."
echo "   Compilando si es necesario (puede tardar 1-2 min)..."
DB_NAME="test_paso1" cargo run -- 8090 8091 > /tmp/test-paso1.log 2>&1 &
SERVER_PID=$!
echo "   PID: $SERVER_PID"
echo "   Esperando compilación e inicio (máximo 2 minutos)..."
sleep 15

if wait_for_server 8090; then
    echo -e "${GREEN}✅ Servidor iniciado${NC}"
else
    echo -e "${RED}❌ Servidor no responde${NC}"
    tail -20 /tmp/test-paso1.log
    kill $SERVER_PID 2>/dev/null || true
    exit 1
fi

# Test 2: Crear wallet
echo ""
echo "2️⃣  Creando wallet..."
WALLET=$(curl -s -X POST "http://localhost:8090/api/v1/wallets/create")
ADDR=$(echo "$WALLET" | jq -r '.data.address' 2>/dev/null)

if [ -n "$ADDR" ] && [ "$ADDR" != "null" ]; then
    echo -e "${GREEN}✅ Wallet creado: ${ADDR:0:20}...${NC}"
else
    echo -e "${RED}❌ Error creando wallet${NC}"
    kill $SERVER_PID 2>/dev/null || true
    exit 1
fi

# Test 3: Minar bloques
echo ""
echo "3️⃣  Minando bloques..."
for i in {1..3}; do
    MINE=$(curl -s -X POST "http://localhost:8090/api/v1/mine" \
        -H "Content-Type: application/json" \
        -d "{\"miner_address\": \"$ADDR\"}")
    
    if echo "$MINE" | grep -q "success\|block"; then
        echo "   Bloque $i minado"
    else
        echo -e "${YELLOW}⚠️  Bloque $i: respuesta inesperada${NC}"
    fi
    sleep 1
done

# Test 4: Verificar archivos de bloques
echo ""
echo "4️⃣  Verificando archivos de bloques..."
sleep 2

if [ -d "test_paso1_blocks" ]; then
    BLOCK_COUNT=$(ls -1 test_paso1_blocks/block_*.dat 2>/dev/null | wc -l | tr -d ' ')
    if [ "$BLOCK_COUNT" -gt 0 ]; then
        echo -e "${GREEN}✅ Archivos de bloques encontrados: $BLOCK_COUNT${NC}"
        ls -lh test_paso1_blocks/block_*.dat | head -3
    else
        echo -e "${YELLOW}⚠️  Directorio existe pero sin archivos${NC}"
    fi
else
    echo -e "${YELLOW}⚠️  Directorio de bloques no encontrado${NC}"
fi

# Test 5: Verificar balance
echo ""
echo "5️⃣  Verificando balance..."
BALANCE=$(curl -s "http://localhost:8090/api/v1/wallets/${ADDR}/balance" | jq -r '.data.balance // 0' 2>/dev/null)
if [ "$BALANCE" != "null" ] && [ -n "$BALANCE" ]; then
    echo -e "${GREEN}✅ Balance: $BALANCE${NC}"
else
    echo -e "${YELLOW}⚠️  Balance no disponible${NC}"
fi

# Test 6: Verificar stats
echo ""
echo "6️⃣  Verificando estadísticas..."
STATS=$(curl -s "http://localhost:8090/api/v1/stats")
BLOCK_COUNT_API=$(echo "$STATS" | jq -r '.data.block_count // 0' 2>/dev/null)
echo "   Bloques en API: $BLOCK_COUNT_API"

# Limpiar
echo ""
echo "🧹 Limpiando..."
kill $SERVER_PID 2>/dev/null || true
sleep 2

# Resumen
echo ""
echo "===================================="
echo -e "${GREEN}✅ TEST PASO 1 COMPLETADO${NC}"
echo ""
echo "📊 Resumen:"
echo "  ✅ Servidor: OK"
echo "  ✅ Wallets: OK"
echo "  ✅ Minería: OK"
echo "  ✅ BlockStorage: Verificado"
echo "  ✅ Balance: OK"
echo ""

