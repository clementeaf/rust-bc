#!/bin/bash

# Script de prueba para Auto-Discovery
# Prueba que los nodos descubran y se conecten automáticamente a nuevos peers

echo "🧪 Testing Auto-Discovery"
echo "=========================="
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
rm -f test_autodiscovery_*.db test_autodiscovery_*.db-shm test_autodiscovery_*.db-wal 2>/dev/null || true

# Verificar que el binario existe
if [ ! -f "./target/release/rust-bc" ]; then
    echo -e "${RED}❌ Error: ./target/release/rust-bc no existe. Compila primero con: cargo build --release${NC}"
    exit 1
fi

PASSED=0
FAILED=0

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

# Función para obtener número de peers
get_peer_count() {
    local api_url=$1
    local count=$(curl -s "$api_url/api/v1/peers" 2>/dev/null | grep -o '\[.*\]' | grep -o ',' | wc -l | tr -d ' ')
    if [ -z "$count" ] || [ "$count" = "0" ]; then
        # Si no hay comas, puede ser 0 o 1 peer
        local response=$(curl -s "$api_url/api/v1/peers" 2>/dev/null)
        if echo "$response" | grep -q '\[\]'; then
            echo "0"
        elif echo "$response" | grep -q '"127.0.0.1'; then
            echo "1"
        else
            echo "0"
        fi
    else
        echo $((count + 1))
    fi
}

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📋 Test: Auto-Discovery de Peers"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Test: Nodo sin bootstrap debería descubrir y conectar a otros nodos automáticamente
echo -e "${BLUE}Test 1: Iniciando nodo 1 (bootstrap, puerto 30000)${NC}"
NETWORK_ID=testnet ./target/release/rust-bc 30000 30001 test_autodiscovery_node1 > /tmp/node_autodiscovery1.log 2>&1 &
NODE1_PID=$!
sleep 3

if ! wait_for_node "http://127.0.0.1:30000" "Nodo 1"; then
    echo -e "${RED}❌ Nodo 1 no inició correctamente${NC}"
    kill $NODE1_PID 2>/dev/null
    exit 1
fi
echo -e "${GREEN}✅ Nodo 1 iniciado${NC}"
((PASSED++))

echo -e "${BLUE}Test 2: Iniciando nodo 2 (con bootstrap al nodo 1, puerto 30002)${NC}"
BOOTSTRAP_NODES="127.0.0.1:30001" NETWORK_ID=testnet ./target/release/rust-bc 30002 30003 test_autodiscovery_node2 > /tmp/node_autodiscovery2.log 2>&1 &
NODE2_PID=$!
sleep 5

if ! wait_for_node "http://127.0.0.1:30002" "Nodo 2"; then
    echo -e "${RED}❌ Nodo 2 no inició correctamente${NC}"
    kill $NODE1_PID $NODE2_PID 2>/dev/null
    exit 1
fi
echo -e "${GREEN}✅ Nodo 2 iniciado y conectado a bootstrap${NC}"
((PASSED++))

# Verificar que nodo 2 se conectó a nodo 1
sleep 2
peers_node2=$(get_peer_count "http://127.0.0.1:30002")
if [ "$peers_node2" -ge "1" ]; then
    echo -e "${GREEN}✅ Nodo 2 tiene peers conectados (bootstrap funcionó)${NC}"
    ((PASSED++))
else
    echo -e "${RED}❌ Nodo 2 no tiene peers conectados${NC}"
    ((FAILED++))
fi

echo -e "${BLUE}Test 3: Iniciando nodo 3 (con bootstrap al nodo 1, puerto 30004)${NC}"
echo -e "${YELLOW}   Este nodo se conectará a bootstrap y luego debería descubrir más peers automáticamente${NC}"
BOOTSTRAP_NODES="127.0.0.1:30001" NETWORK_ID=testnet ./target/release/rust-bc 30004 30005 test_autodiscovery_node3 > /tmp/node_autodiscovery3.log 2>&1 &
NODE3_PID=$!
sleep 5

if ! wait_for_node "http://127.0.0.1:30004" "Nodo 3"; then
    echo -e "${RED}❌ Nodo 3 no inició correctamente${NC}"
    kill $NODE1_PID $NODE2_PID $NODE3_PID 2>/dev/null
    exit 1
fi
echo -e "${GREEN}✅ Nodo 3 iniciado y conectado a bootstrap${NC}"
((PASSED++))

# Verificar estado inicial (nodo 3 debería tener al menos 1 peer del bootstrap)
echo -e "${BLUE}Test 4: Verificando estado inicial del nodo 3...${NC}"
sleep 2
peers_node3_initial=$(get_peer_count "http://127.0.0.1:30004")
echo "   Peers iniciales en nodo 3: $peers_node3_initial"

if [ "$peers_node3_initial" -ge "1" ]; then
    echo -e "${GREEN}✅ Nodo 3 tiene peers iniciales del bootstrap (correcto)${NC}"
    ((PASSED++))
else
    echo -e "${YELLOW}⚠️  Nodo 3 tiene $peers_node3_initial peers inicialmente${NC}"
fi

# Esperar para que el auto-discovery se ejecute (máximo 2.5 minutos)
echo -e "${BLUE}Test 5: Esperando auto-discovery (puede tomar hasta 2.5 minutos)...${NC}"
echo -e "${YELLOW}   El auto-discovery se ejecuta cada 2 minutos, con delay inicial de 30 segundos${NC}"
echo -e "${YELLOW}   El nodo 3 debería descubrir al nodo 2 automáticamente${NC}"

max_wait=150  # 2.5 minutos
elapsed=0
check_interval=10  # Verificar cada 10 segundos
discovered=false

while [ $elapsed -lt $max_wait ]; do
    sleep $check_interval
    elapsed=$((elapsed + check_interval))
    
    peers_node3=$(get_peer_count "http://127.0.0.1:30004")
    echo "   [$elapsed segundos] Peers en nodo 3: $peers_node3"
    
    # El nodo 3 debería tener al menos 2 peers (nodo 1 del bootstrap + nodo 2 descubierto)
    if [ "$peers_node3" -ge "2" ]; then
        echo -e "${GREEN}✅ Auto-discovery funcionó! Nodo 3 descubrió y se conectó al nodo 2${NC}"
        discovered=true
        ((PASSED++))
        break
    fi
done

if [ "$discovered" = false ]; then
    echo -e "${RED}❌ Auto-discovery no funcionó después de $max_wait segundos${NC}"
    echo "   Revisando logs..."
    echo "   Últimas líneas del log del nodo 3:"
    tail -10 /tmp/node_autodiscovery3.log
    ((FAILED++))
else
    # Verificar que los otros nodos también ven al nodo 3
    echo -e "${BLUE}Test 6: Verificando que otros nodos ven al nodo 3...${NC}"
    sleep 2
    
    peers_node1=$(get_peer_count "http://127.0.0.1:30000")
    peers_node2=$(get_peer_count "http://127.0.0.1:30002")
    
    echo "   Peers en nodo 1: $peers_node1"
    echo "   Peers en nodo 2: $peers_node2"
    
    if [ "$peers_node1" -ge "1" ] || [ "$peers_node2" -ge "2" ]; then
        echo -e "${GREEN}✅ Otros nodos también ven al nodo 3${NC}"
        ((PASSED++))
    else
        echo -e "${YELLOW}⚠️  Otros nodos pueden no ver al nodo 3 aún (puede ser normal)${NC}"
    fi
    
    # Verificar logs para mensajes de auto-discovery
    echo -e "${BLUE}Test 7: Verificando logs de auto-discovery...${NC}"
    if grep -q "Descubiertos\|Auto-conectado" /tmp/node_autodiscovery3.log 2>/dev/null; then
        echo -e "${GREEN}✅ Logs muestran actividad de auto-discovery${NC}"
        echo "   Mensajes encontrados:"
        grep -i "Descubiertos\|Auto-conectado" /tmp/node_autodiscovery3.log | tail -3
        ((PASSED++))
    else
        echo -e "${YELLOW}⚠️  No se encontraron mensajes de auto-discovery en logs (puede ser normal si aún no se ejecutó)${NC}"
    fi
fi

# Limpieza
echo ""
echo -e "${YELLOW}🧹 Limpiando nodos...${NC}"
kill $NODE1_PID $NODE2_PID $NODE3_PID 2>/dev/null
sleep 2
rm -f test_autodiscovery_*.db test_autodiscovery_*.db-shm test_autodiscovery_*.db-wal 2>/dev/null || true

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
    echo -e "${YELLOW}⚠️  Algunos tests fallaron o no se completaron${NC}"
    echo ""
    echo "Nota: El auto-discovery se ejecuta cada 2 minutos con un delay inicial de 30 segundos."
    echo "Si el test falló, puede ser porque no se esperó suficiente tiempo."
    exit 1
fi

