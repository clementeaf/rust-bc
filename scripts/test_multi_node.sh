#!/bin/bash

# Script de prueba con múltiples nodos P2P
# Prueba sincronización, broadcast y consenso entre nodos

echo "🌐 Testing con Múltiples Nodos P2P"
echo "==================================="
echo ""

# Colores
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

NODE1_API="http://127.0.0.1:8080"
NODE1_P2P="127.0.0.1:8081"
NODE2_API="http://127.0.0.1:8082"
NODE2_P2P="127.0.0.1:8083"
NODE3_API="http://127.0.0.1:8084"
NODE3_P2P="127.0.0.1:8085"

echo -e "${YELLOW}⚠️  IMPORTANTE: Este script requiere 3 nodos corriendo${NC}"
echo ""
echo "Para ejecutar los nodos, abre 3 terminales y ejecuta:"
echo ""
echo -e "${BLUE}Terminal 1 (Nodo 1):${NC}"
echo "  cargo run 8080 8081 blockchain1"
echo ""
echo -e "${BLUE}Terminal 2 (Nodo 2):${NC}"
echo "  cargo run 8082 8083 blockchain2"
echo ""
echo -e "${BLUE}Terminal 3 (Nodo 3):${NC}"
echo "  cargo run 8084 8085 blockchain3"
echo ""
read -p "¿Los 3 nodos están corriendo? (y/n): " -n 1 -r
echo ""
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo -e "${RED}Por favor inicia los 3 nodos primero${NC}"
    exit 1
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📋 Pruebas de Red P2P"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

PASSED=0
FAILED=0

# Función para verificar estado de nodo
check_node() {
    local api_url=$1
    local node_name=$2
    
    response=$(curl -s "$api_url/api/v1/chain/info" 2>/dev/null)
    if [ $? -eq 0 ] && echo "$response" | grep -q "block_count"; then
        echo -e "${GREEN}✅ $node_name está activo${NC}"
        ((PASSED++))
        return 0
    else
        echo -e "${RED}❌ $node_name no está respondiendo${NC}"
        ((FAILED++))
        return 1
    fi
}

# 1. Verificar que todos los nodos estén activos
echo "🔍 Verificando nodos activos..."
check_node "$NODE1_API" "Nodo 1"
check_node "$NODE2_API" "Nodo 2"
check_node "$NODE3_API" "Nodo 3"
echo ""

# 2. Conectar nodos
echo "🔗 Conectando nodos..."
echo "Conectando Nodo 2 -> Nodo 1..."
curl -s -X POST "$NODE2_API/api/v1/peers/$NODE1_P2P/connect" > /dev/null
sleep 1

echo "Conectando Nodo 3 -> Nodo 1..."
curl -s -X POST "$NODE3_API/api/v1/peers/$NODE1_P2P/connect" > /dev/null
sleep 1

echo "Conectando Nodo 3 -> Nodo 2..."
curl -s -X POST "$NODE3_API/api/v1/peers/$NODE2_P2P/connect" > /dev/null
sleep 2
echo ""

# 3. Verificar peers conectados
echo "👥 Verificando peers conectados..."
echo "Peers del Nodo 1:"
curl -s "$NODE1_API/api/v1/peers" | jq '.' 2>/dev/null || curl -s "$NODE1_API/api/v1/peers"
echo ""
echo "Peers del Nodo 2:"
curl -s "$NODE2_API/api/v1/peers" | jq '.' 2>/dev/null || curl -s "$NODE2_API/api/v1/peers"
echo ""
echo "Peers del Nodo 3:"
curl -s "$NODE3_API/api/v1/peers" | jq '.' 2>/dev/null || curl -s "$NODE3_API/api/v1/peers"
echo ""

# 4. Crear wallet en Nodo 1
echo "📝 Creando wallet en Nodo 1..."
wallet_response=$(curl -s -X POST "$NODE1_API/api/v1/wallets/create" 2>/dev/null)
wallet_address=$(echo "$wallet_response" | grep -o '"address":"[^"]*' | cut -d'"' -f4)
if [ -z "$wallet_address" ]; then
    echo -e "${RED}❌ Error creando wallet${NC}"
    exit 1
fi
echo -e "${GREEN}✅ Wallet creado: $wallet_address${NC}"
echo ""

# 5. Minar bloque en Nodo 1
echo "⛏️  Minando bloque en Nodo 1..."
mine_response=$(curl -s -X POST "$NODE1_API/api/v1/mine" \
    -H "Content-Type: application/json" \
    -d "{\"miner_address\":\"$wallet_address\",\"max_transactions\":10}" 2>/dev/null)
echo "$mine_response" | jq '.' 2>/dev/null || echo "$mine_response"
sleep 2
echo ""

# 6. Sincronizar nodos
echo "🔄 Sincronizando nodos..."
echo "Sincronizando Nodo 2..."
curl -s -X POST "$NODE2_API/api/v1/sync" > /dev/null
sleep 2

echo "Sincronizando Nodo 3..."
curl -s -X POST "$NODE3_API/api/v1/sync" > /dev/null
sleep 2
echo ""

# 7. Verificar que todos tienen el mismo número de bloques
echo "📊 Verificando sincronización..."
node1_info=$(curl -s "$NODE1_API/api/v1/chain/info" 2>/dev/null)
node2_info=$(curl -s "$NODE2_API/api/v1/chain/info" 2>/dev/null)
node3_info=$(curl -s "$NODE3_API/api/v1/chain/info" 2>/dev/null)

node1_blocks=$(echo "$node1_info" | grep -o '"block_count":[0-9]*' | cut -d':' -f2)
node2_blocks=$(echo "$node2_info" | grep -o '"block_count":[0-9]*' | cut -d':' -f2)
node3_blocks=$(echo "$node3_info" | grep -o '"block_count":[0-9]*' | cut -d':' -f2)

echo "Nodo 1: $node1_blocks bloques"
echo "Nodo 2: $node2_blocks bloques"
echo "Nodo 3: $node3_blocks bloques"

if [ "$node1_blocks" == "$node2_blocks" ] && [ "$node2_blocks" == "$node3_blocks" ]; then
    echo -e "${GREEN}✅ Todos los nodos tienen el mismo número de bloques${NC}"
    ((PASSED++))
else
    echo -e "${RED}❌ Los nodos tienen diferentes números de bloques${NC}"
    ((FAILED++))
fi
echo ""

# 8. Minar otro bloque en Nodo 2
echo "⛏️  Minando bloque en Nodo 2..."
mine_response=$(curl -s -X POST "$NODE2_API/api/v1/mine" \
    -H "Content-Type: application/json" \
    -d "{\"miner_address\":\"$wallet_address\",\"max_transactions\":10}" 2>/dev/null)
echo "$mine_response" | jq '.' 2>/dev/null || echo "$mine_response"
sleep 3
echo ""

# 9. Verificar que el bloque se propagó
echo "📡 Verificando propagación del bloque..."
sleep 2
node1_info=$(curl -s "$NODE1_API/api/v1/chain/info" 2>/dev/null)
node3_info=$(curl -s "$NODE3_API/api/v1/chain/info" 2>/dev/null)

node1_blocks=$(echo "$node1_info" | grep -o '"block_count":[0-9]*' | cut -d':' -f2)
node3_blocks=$(echo "$node3_info" | grep -o '"block_count":[0-9]*' | cut -d':' -f2)

echo "Nodo 1: $node1_blocks bloques"
echo "Nodo 3: $node3_blocks bloques"

if [ "$node1_blocks" == "$node3_blocks" ]; then
    echo -e "${GREEN}✅ El bloque se propagó correctamente${NC}"
    ((PASSED++))
else
    echo -e "${RED}❌ El bloque no se propagó correctamente${NC}"
    ((FAILED++))
fi
echo ""

# Resumen
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📊 Resumen de Pruebas P2P"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo -e "${GREEN}✅ Pruebas pasadas: $PASSED${NC}"
echo -e "${RED}❌ Pruebas fallidas: $FAILED${NC}"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}🎉 ¡Todas las pruebas P2P pasaron!${NC}"
    exit 0
else
    echo -e "${YELLOW}⚠️  Algunas pruebas P2P fallaron${NC}"
    exit 1
fi
