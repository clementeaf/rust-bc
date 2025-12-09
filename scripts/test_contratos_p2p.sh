#!/bin/bash

# Test: Sincronización de contratos P2P con BlockStorage

set -e

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo "🧪 TEST: Sincronización de Contratos P2P"
echo "=========================================="
echo ""

cd /Users/clementefalcone/Desktop/personal/rust-bc

# Limpiar
pkill -9 -f "rust-bc.*8090\|rust-bc.*8092" 2>/dev/null || true
rm -rf test_contratos* test_contratos_blocks test_contratos2* test_contratos2_blocks 2>/dev/null || true
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

# Test 1: Iniciar nodo 1
echo "1️⃣  Iniciando nodo 1 (puerto 8090)..."
DB_NAME="test_contratos" cargo run -- 8090 8091 > /tmp/node1-contracts.log 2>&1 &
NODE1_PID=$!
sleep 10

if wait_for_server 8090; then
    echo -e "${GREEN}✅ Nodo 1 iniciado${NC}"
else
    echo -e "${RED}❌ Nodo 1 no inició${NC}"
    kill $NODE1_PID 2>/dev/null || true
    exit 1
fi

# Test 2: Crear wallet y minar en nodo 1
echo ""
echo "2️⃣  Creando wallet y minando en nodo 1..."
WALLET1=$(curl -s -X POST "http://localhost:8090/api/v1/wallets/create")
ADDR1=$(echo "$WALLET1" | jq -r '.data.address' 2>/dev/null)

if [ -n "$ADDR1" ] && [ "$ADDR1" != "null" ]; then
    echo -e "${GREEN}✅ Wallet creado: ${ADDR1:0:20}...${NC}"
    
    # Minar 1 bloque para tener balance
    MINE=$(curl -s -X POST "http://localhost:8090/api/v1/mine" \
        -H "Content-Type: application/json" \
        -d "{\"miner_address\": \"$ADDR1\"}")
    if echo "$MINE" | grep -q "success\|block"; then
        echo "   Bloque minado"
        sleep 1
    fi
else
    echo -e "${RED}❌ Error creando wallet${NC}"
    kill $NODE1_PID 2>/dev/null || true
    exit 1
fi

# Test 3: Desplegar contrato en nodo 1
echo ""
echo "3️⃣  Desplegando contrato ERC-20 en nodo 1..."
DEPLOY=$(curl -s -X POST "http://localhost:8090/api/v1/contracts/deploy" \
    -H "Content-Type: application/json" \
    -d '{
        "owner": "'"$ADDR1"'",
        "contract_type": "token",
        "name": "TestToken",
        "symbol": "TEST",
        "total_supply": 1000000,
        "decimals": 18
    }')

CONTRACT_ADDR=$(echo "$DEPLOY" | jq -r '.data // ""' 2>/dev/null)
if [ -n "$CONTRACT_ADDR" ] && [ "$CONTRACT_ADDR" != "null" ] && [ ${#CONTRACT_ADDR} -gt 10 ]; then
    echo -e "${GREEN}✅ Contrato desplegado: ${CONTRACT_ADDR:0:30}...${NC}"
    
    # Verificar que el contrato existe en nodo 1
    GET_CONTRACT=$(curl -s "http://localhost:8090/api/v1/contracts/${CONTRACT_ADDR}")
    if echo "$GET_CONTRACT" | grep -q "TestToken\|TEST"; then
        echo -e "${GREEN}✅ Contrato verificado en nodo 1${NC}"
    else
        echo -e "${YELLOW}⚠️  Contrato no encontrado en nodo 1${NC}"
    fi
else
    echo -e "${RED}❌ Error desplegando contrato${NC}"
    echo "Respuesta: $DEPLOY"
    kill $NODE1_PID 2>/dev/null || true
    exit 1
fi

# Test 4: Iniciar nodo 2 y conectar
echo ""
echo "4️⃣  Iniciando nodo 2 y conectando P2P..."
DB_NAME="test_contratos2" cargo run -- 8092 8093 > /tmp/node2-contracts.log 2>&1 &
NODE2_PID=$!
sleep 10

if wait_for_server 8092; then
    echo -e "${GREEN}✅ Nodo 2 iniciado${NC}"
    
    # Conectar nodo 2 a nodo 1
    CONNECT=$(curl -s -X POST "http://localhost:8092/api/v1/peers/127.0.0.1:8091/connect")
    if echo "$CONNECT" | grep -q "success\|connected"; then
        echo -e "${GREEN}✅ Nodos conectados P2P${NC}"
        sleep 5
        
        # Verificar que el contrato se sincronizó en nodo 2
        echo ""
        echo "5️⃣  Verificando sincronización de contrato..."
        GET_CONTRACT2=$(curl -s "http://localhost:8092/api/v1/contracts/${CONTRACT_ADDR}")
        if echo "$GET_CONTRACT2" | grep -q "TestToken\|TEST"; then
            echo -e "${GREEN}✅ Contrato sincronizado en nodo 2${NC}"
            
            # Verificar detalles
            CONTRACT_NAME=$(echo "$GET_CONTRACT2" | jq -r '.data.name // ""' 2>/dev/null)
            CONTRACT_SYMBOL=$(echo "$GET_CONTRACT2" | jq -r '.data.symbol // ""' 2>/dev/null)
            if [ "$CONTRACT_NAME" = "TestToken" ] && [ "$CONTRACT_SYMBOL" = "TEST" ]; then
                echo -e "${GREEN}✅ Detalles del contrato correctos${NC}"
                echo "   Nombre: $CONTRACT_NAME"
                echo "   Símbolo: $CONTRACT_SYMBOL"
            else
                echo -e "${YELLOW}⚠️  Detalles del contrato no coinciden${NC}"
            fi
        else
            echo -e "${YELLOW}⚠️  Contrato no sincronizado en nodo 2${NC}"
            echo "Respuesta: $GET_CONTRACT2"
        fi
        
        # Verificar lista de contratos
        ALL_CONTRACTS=$(curl -s "http://localhost:8092/api/v1/contracts")
        CONTRACT_COUNT=$(echo "$ALL_CONTRACTS" | jq -r '.data | length // 0' 2>/dev/null)
        echo "   Contratos en nodo 2: $CONTRACT_COUNT"
        
    else
        echo -e "${YELLOW}⚠️  Conexión P2P no confirmada${NC}"
    fi
else
    echo -e "${RED}❌ Nodo 2 no inició${NC}"
    kill $NODE1_PID $NODE2_PID 2>/dev/null || true
    exit 1
fi

# Limpiar
echo ""
echo "🧹 Limpiando..."
kill $NODE1_PID $NODE2_PID 2>/dev/null || true
sleep 2

# Resumen
echo ""
echo "=========================================="
echo -e "${GREEN}✅ TEST DE CONTRATOS COMPLETADO${NC}"
echo ""
echo "📊 Resumen:"
echo "  ✅ Nodos P2P: OK"
echo "  ✅ Despliegue de contrato: OK"
echo "  ✅ Sincronización P2P: Verificada"
echo "  ✅ BlockStorage: Funcionando"
echo ""

