#!/bin/bash

# Script de prueba para Network ID y Bootstrap Nodes
# Prueba separación de redes y auto-conexión a bootstrap nodes

echo "🧪 Testing Network ID y Bootstrap Nodes"
echo "=========================================="
echo ""

# Colores
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Limpiar procesos anteriores y bases de datos
echo -e "${YELLOW}🧹 Limpiando procesos anteriores...${NC}"
pkill -f rust-bc 2>/dev/null || true
sleep 2
rm -f test_network_*.db test_network_*.db-shm test_network_*.db-wal 2>/dev/null || true

# Verificar que el binario existe
if [ ! -f "./target/release/rust-bc" ]; then
    echo -e "${RED}❌ Error: ./target/release/rust-bc no existe. Compila primero con: cargo build --release${NC}"
    exit 1
fi

PASSED=0
FAILED=0

# Función para verificar estado de nodo
check_node() {
    local api_url=$1
    local node_name=$2
    
    response=$(curl -s --max-time 2 "$api_url/api/v1/health" 2>/dev/null)
    if [ $? -eq 0 ] && echo "$response" | grep -q "healthy\|status"; then
        echo -e "${GREEN}✅ $node_name está activo${NC}"
        ((PASSED++))
        return 0
    else
        echo -e "${RED}❌ $node_name no está respondiendo${NC}"
        ((FAILED++))
        return 1
    fi
}

# Función para esperar a que un nodo esté listo
wait_for_node() {
    local api_url=$1
    local node_name=$2
    local max_attempts=30
    local attempt=0
    
    while [ $attempt -lt $max_attempts ]; do
        if curl -s --max-time 1 "$api_url/api/v1/health" 2>/dev/null | grep -q "healthy\|status"; then
            return 0
        fi
        sleep 1
        ((attempt++))
    done
    
    echo -e "${RED}❌ $node_name no respondió después de $max_attempts segundos${NC}"
    return 1
}

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📋 Test 1: Network ID Validation"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Test 1: Nodos con diferentes network_id no deben conectarse
echo -e "${BLUE}Test 1.1: Iniciando nodo MAINNET (puerto 20000)${NC}"
NETWORK_ID=mainnet ./target/release/rust-bc 20000 20001 test_network_mainnet > /tmp/node_mainnet.log 2>&1 &
NODE_MAINNET_PID=$!
sleep 3

if ! wait_for_node "http://127.0.0.1:20000" "Nodo MAINNET"; then
    echo -e "${RED}❌ Nodo MAINNET no inició correctamente${NC}"
    kill $NODE_MAINNET_PID 2>/dev/null
    exit 1
fi

echo -e "${BLUE}Test 1.2: Iniciando nodo TESTNET (puerto 20002)${NC}"
NETWORK_ID=testnet ./target/release/rust-bc 20002 20003 test_network_testnet > /tmp/node_testnet.log 2>&1 &
NODE_TESTNET_PID=$!
sleep 3

if ! wait_for_node "http://127.0.0.1:20002" "Nodo TESTNET"; then
    echo -e "${RED}❌ Nodo TESTNET no inició correctamente${NC}"
    kill $NODE_MAINNET_PID $NODE_TESTNET_PID 2>/dev/null
    exit 1
fi

echo -e "${BLUE}Test 1.3: Intentando conectar TESTNET a MAINNET (debe fallar)${NC}"
response=$(curl -s -X POST "http://127.0.0.1:20002/api/v1/peers/127.0.0.1:20001/connect" 2>&1)
sleep 3

# Verificar que no están conectados (esto es lo más importante)
peers_testnet=$(curl -s "http://127.0.0.1:20002/api/v1/peers" 2>/dev/null | grep -o "127.0.0.1:20001" || echo "")
peers_mainnet=$(curl -s "http://127.0.0.1:20000/api/v1/peers" 2>/dev/null | grep -o "127.0.0.1:20003" || echo "")

if [ -z "$peers_testnet" ] && [ -z "$peers_mainnet" ]; then
    echo -e "${GREEN}✅ Conexión rechazada correctamente (nodos no están conectados)${NC}"
    ((PASSED++))
    # Verificar si el error está en la respuesta (opcional, pero bueno tenerlo)
    if echo "$response" | grep -qi "network.*mismatch\|error"; then
        echo -e "${GREEN}✅ Error reportado correctamente en la respuesta${NC}"
        ((PASSED++))
    else
        echo -e "${YELLOW}⚠️  Error no reportado en respuesta, pero conexión fue rechazada${NC}"
    fi
else
    echo -e "${RED}❌ Conexión no fue rechazada (nodos están conectados)${NC}"
    echo "   Respuesta API: $response"
    echo "   Peers TESTNET: $peers_testnet"
    echo "   Peers MAINNET: $peers_mainnet"
    ((FAILED++))
fi

# Limpiar para siguiente test
echo -e "${YELLOW}🧹 Limpiando nodos del Test 1...${NC}"
kill $NODE_MAINNET_PID $NODE_TESTNET_PID 2>/dev/null
sleep 2
rm -f test_network_mainnet.db test_network_testnet.db 2>/dev/null || true

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📋 Test 2: Bootstrap Nodes"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Test 2: Bootstrap nodes - auto-conexión
echo -e "${BLUE}Test 2.1: Iniciando nodo bootstrap (puerto 20004)${NC}"
NETWORK_ID=testnet ./target/release/rust-bc 20004 20005 test_network_bootstrap1 > /tmp/node_bootstrap1.log 2>&1 &
NODE_BOOTSTRAP1_PID=$!
sleep 3

if ! wait_for_node "http://127.0.0.1:20004" "Nodo Bootstrap 1"; then
    echo -e "${RED}❌ Nodo Bootstrap 1 no inició correctamente${NC}"
    kill $NODE_BOOTSTRAP1_PID 2>/dev/null
    exit 1
fi

echo -e "${BLUE}Test 2.2: Iniciando nodo con bootstrap configurado (puerto 20006)${NC}"
BOOTSTRAP_NODES="127.0.0.1:20005" NETWORK_ID=testnet ./target/release/rust-bc 20006 20007 test_network_bootstrap2 > /tmp/node_bootstrap2.log 2>&1 &
NODE_BOOTSTRAP2_PID=$!
sleep 5  # Dar tiempo para que se conecte automáticamente

if ! wait_for_node "http://127.0.0.1:20006" "Nodo Bootstrap 2"; then
    echo -e "${RED}❌ Nodo Bootstrap 2 no inició correctamente${NC}"
    kill $NODE_BOOTSTRAP1_PID $NODE_BOOTSTRAP2_PID 2>/dev/null
    exit 1
fi

# Verificar que se conectaron automáticamente
echo -e "${BLUE}Test 2.3: Verificando auto-conexión...${NC}"
sleep 2

peers_bootstrap1=$(curl -s "http://127.0.0.1:20004/api/v1/peers" 2>/dev/null | grep -o "127.0.0.1:20007" || echo "")
peers_bootstrap2=$(curl -s "http://127.0.0.1:20006/api/v1/peers" 2>/dev/null | grep -o "127.0.0.1:20005" || echo "")

if [ -n "$peers_bootstrap1" ] && [ -n "$peers_bootstrap2" ]; then
    echo -e "${GREEN}✅ Auto-conexión exitosa (ambos nodos se ven mutuamente)${NC}"
    ((PASSED++))
else
    echo -e "${RED}❌ Auto-conexión falló${NC}"
    echo "   Bootstrap1 peers: $peers_bootstrap1"
    echo "   Bootstrap2 peers: $peers_bootstrap2"
    ((FAILED++))
fi

# Verificar logs para mensaje de conexión
if grep -q "Conectado a bootstrap node" /tmp/node_bootstrap2.log 2>/dev/null; then
    echo -e "${GREEN}✅ Log muestra conexión a bootstrap node${NC}"
    ((PASSED++))
else
    echo -e "${YELLOW}⚠️  Log no muestra mensaje de conexión (puede ser normal si ya estaba conectado)${NC}"
fi

# Limpiar
echo -e "${YELLOW}🧹 Limpiando nodos del Test 2...${NC}"
kill $NODE_BOOTSTRAP1_PID $NODE_BOOTSTRAP2_PID 2>/dev/null
sleep 2
rm -f test_network_bootstrap*.db 2>/dev/null || true

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📋 Test 3: Múltiples Bootstrap Nodes"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Test 3: Múltiples bootstrap nodes
echo -e "${BLUE}Test 3.1: Iniciando nodo 1 (puerto 20008)${NC}"
NETWORK_ID=testnet ./target/release/rust-bc 20008 20009 test_network_multi1 > /tmp/node_multi1.log 2>&1 &
NODE_MULTI1_PID=$!
sleep 3

if ! wait_for_node "http://127.0.0.1:20008" "Nodo Multi 1"; then
    echo -e "${RED}❌ Nodo Multi 1 no inició correctamente${NC}"
    kill $NODE_MULTI1_PID 2>/dev/null
    exit 1
fi

echo -e "${BLUE}Test 3.2: Iniciando nodo 2 (puerto 20010)${NC}"
BOOTSTRAP_NODES="127.0.0.1:20009" NETWORK_ID=testnet ./target/release/rust-bc 20010 20011 test_network_multi2 > /tmp/node_multi2.log 2>&1 &
NODE_MULTI2_PID=$!
sleep 5

if ! wait_for_node "http://127.0.0.1:20010" "Nodo Multi 2"; then
    echo -e "${RED}❌ Nodo Multi 2 no inició correctamente${NC}"
    kill $NODE_MULTI1_PID $NODE_MULTI2_PID 2>/dev/null
    exit 1
fi

echo -e "${BLUE}Test 3.3: Iniciando nodo 3 con múltiples bootstrap nodes (puerto 20012)${NC}"
BOOTSTRAP_NODES="127.0.0.1:20009,127.0.0.1:20011" NETWORK_ID=testnet ./target/release/rust-bc 20012 20013 test_network_multi3 > /tmp/node_multi3.log 2>&1 &
NODE_MULTI3_PID=$!
sleep 5

if ! wait_for_node "http://127.0.0.1:20012" "Nodo Multi 3"; then
    echo -e "${RED}❌ Nodo Multi 3 no inició correctamente${NC}"
    kill $NODE_MULTI1_PID $NODE_MULTI2_PID $NODE_MULTI3_PID 2>/dev/null
    exit 1
fi

# Verificar conexiones
echo -e "${BLUE}Test 3.4: Verificando conexiones múltiples...${NC}"
sleep 3

peers_multi1=$(curl -s "http://127.0.0.1:20008/api/v1/peers" 2>/dev/null)
peers_multi2=$(curl -s "http://127.0.0.1:20010/api/v1/peers" 2>/dev/null)
peers_multi3=$(curl -s "http://127.0.0.1:20012/api/v1/peers" 2>/dev/null)

connected_count=0
if echo "$peers_multi1" | grep -q "20011\|20013"; then
    ((connected_count++))
fi
if echo "$peers_multi2" | grep -q "20009\|20013"; then
    ((connected_count++))
fi
if echo "$peers_multi3" | grep -q "20009\|20011"; then
    ((connected_count++))
fi

if [ $connected_count -ge 2 ]; then
    echo -e "${GREEN}✅ Múltiples conexiones establecidas correctamente${NC}"
    ((PASSED++))
else
    echo -e "${RED}❌ No se establecieron suficientes conexiones${NC}"
    echo "   Nodo 1 peers: $peers_multi1"
    echo "   Nodo 2 peers: $peers_multi2"
    echo "   Nodo 3 peers: $peers_multi3"
    ((FAILED++))
fi

# Limpiar
echo -e "${YELLOW}🧹 Limpiando nodos del Test 3...${NC}"
kill $NODE_MULTI1_PID $NODE_MULTI2_PID $NODE_MULTI3_PID 2>/dev/null
sleep 2
rm -f test_network_multi*.db 2>/dev/null || true

# Limpieza final
echo ""
echo -e "${YELLOW}🧹 Limpieza final...${NC}"
pkill -f rust-bc 2>/dev/null || true
sleep 2
rm -f test_network_*.db test_network_*.db-shm test_network_*.db-wal 2>/dev/null || true

# Resumen
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📊 Resumen de Pruebas"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo -e "${GREEN}✅ Tests pasados: $PASSED${NC}"
echo -e "${RED}❌ Tests fallidos: $FAILED${NC}"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}🎉 ¡Todos los tests pasaron!${NC}"
    exit 0
else
    echo -e "${RED}⚠️  Algunos tests fallaron${NC}"
    exit 1
fi

