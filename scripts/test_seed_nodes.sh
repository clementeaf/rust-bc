#!/bin/bash

# Script de prueba para Seed Nodes
# Prueba que los nodos con seed nodes pueden descubrir la red automáticamente

set -e

BASE_PORT=20000
API_PORT_1=$((BASE_PORT + 0))
P2P_PORT_1=$((BASE_PORT + 1))
API_PORT_2=$((BASE_PORT + 2))
P2P_PORT_2=$((BASE_PORT + 3))
API_PORT_3=$((BASE_PORT + 4))
P2P_PORT_3=$((BASE_PORT + 5))

DB1="blockchain_test_seed1"
DB2="blockchain_test_seed2"
DB3="blockchain_test_seed3"

# Limpiar bases de datos anteriores
rm -f "${DB1}.db" "${DB2}.db" "${DB3}.db"

echo "🧪 Test de Seed Nodes"
echo "===================="
echo ""
echo "Escenario:"
echo "  - Nodo 1: Sin seed nodes (primer nodo)"
echo "  - Nodo 2: Con seed node al Nodo 1"
echo "  - Nodo 3: Con seed node al Nodo 1 (debe descubrir también al Nodo 2)"
echo ""

# Función para limpiar procesos al finalizar
cleanup() {
    echo ""
    echo "🧹 Limpiando procesos..."
    pkill -f "rust-bc.*${API_PORT_1}" || true
    pkill -f "rust-bc.*${API_PORT_2}" || true
    pkill -f "rust-bc.*${API_PORT_3}" || true
    sleep 2
    rm -f "${DB1}.db" "${DB2}.db" "${DB3}.db"
    echo "✅ Limpieza completada"
}

trap cleanup EXIT

# Iniciar Nodo 1 (sin seed nodes, primer nodo)
echo "🚀 Iniciando Nodo 1 (sin seed nodes)..."
RUST_LOG=info cargo run --release ${API_PORT_1} ${P2P_PORT_1} ${DB1} > /tmp/node1_seed.log 2>&1 &
NODE1_PID=$!
sleep 5

# Verificar que Nodo 1 está corriendo
if ! kill -0 $NODE1_PID 2>/dev/null; then
    echo "❌ Error: Nodo 1 no inició correctamente"
    cat /tmp/node1_seed.log
    exit 1
fi

echo "✅ Nodo 1 iniciado (PID: $NODE1_PID)"
sleep 3

# Verificar health del Nodo 1
echo "🔍 Verificando health del Nodo 1..."
for i in {1..10}; do
    if curl -s "http://127.0.0.1:${API_PORT_1}/api/v1/health" > /dev/null 2>&1; then
        echo "✅ Nodo 1 está respondiendo"
        break
    fi
    if [ $i -eq 10 ]; then
        echo "❌ Error: Nodo 1 no responde después de 10 intentos"
        cat /tmp/node1_seed.log
        exit 1
    fi
    sleep 1
done

# Iniciar Nodo 2 (con seed node al Nodo 1)
echo ""
echo "🚀 Iniciando Nodo 2 (con seed node al Nodo 1)..."
SEED_NODES="127.0.0.1:${P2P_PORT_1}" RUST_LOG=info cargo run --release ${API_PORT_2} ${P2P_PORT_2} ${DB2} > /tmp/node2_seed.log 2>&1 &
NODE2_PID=$!
sleep 5

# Verificar que Nodo 2 está corriendo
if ! kill -0 $NODE2_PID 2>/dev/null; then
    echo "❌ Error: Nodo 2 no inició correctamente"
    cat /tmp/node2_seed.log
    exit 1
fi

echo "✅ Nodo 2 iniciado (PID: $NODE2_PID)"
sleep 5

# Verificar health del Nodo 2
echo "🔍 Verificando health del Nodo 2..."
for i in {1..10}; do
    if curl -s "http://127.0.0.1:${API_PORT_2}/api/v1/health" > /dev/null 2>&1; then
        echo "✅ Nodo 2 está respondiendo"
        break
    fi
    if [ $i -eq 10 ]; then
        echo "❌ Error: Nodo 2 no responde después de 10 intentos"
        cat /tmp/node2_seed.log
        exit 1
    fi
    sleep 1
done

# Esperar a que el auto-discovery conecte (puede tomar hasta 2 minutos con delay inicial de 30s)
echo ""
echo "⏳ Esperando auto-discovery (puede tomar hasta 90 segundos)..."
sleep 90

# Verificar que Nodo 2 se conectó a Nodo 1
echo ""
echo "🔍 Verificando conexión Nodo 2 -> Nodo 1..."
NODE2_PEERS=$(curl -s "http://127.0.0.1:${API_PORT_2}/api/v1/peers" | jq -r '.data[]' 2>/dev/null || echo "")

if echo "$NODE2_PEERS" | grep -q "127.0.0.1:${P2P_PORT_1}"; then
    echo "✅ Nodo 2 se conectó a Nodo 1 vía seed node"
else
    echo "⚠️  Nodo 2 aún no se conectó a Nodo 1"
    echo "   Peers de Nodo 2: $NODE2_PEERS"
    echo "   Logs de Nodo 2:"
    tail -20 /tmp/node2_seed.log
fi

# Verificar que Nodo 1 ve a Nodo 2
echo ""
echo "🔍 Verificando que Nodo 1 ve a Nodo 2..."
NODE1_PEERS=$(curl -s "http://127.0.0.1:${API_PORT_1}/api/v1/peers" | jq -r '.data[]' 2>/dev/null || echo "")

if echo "$NODE1_PEERS" | grep -q "127.0.0.1:${P2P_PORT_2}"; then
    echo "✅ Nodo 1 ve a Nodo 2 (conexión bidireccional funcionando)"
else
    echo "⚠️  Nodo 1 aún no ve a Nodo 2"
    echo "   Peers de Nodo 1: $NODE1_PEERS"
fi

# Iniciar Nodo 3 (con seed node al Nodo 1, debe descubrir también al Nodo 2)
echo ""
echo "🚀 Iniciando Nodo 3 (con seed node al Nodo 1, debe descubrir también al Nodo 2)..."
SEED_NODES="127.0.0.1:${P2P_PORT_1}" RUST_LOG=info cargo run --release ${API_PORT_3} ${P2P_PORT_3} ${DB3} > /tmp/node3_seed.log 2>&1 &
NODE3_PID=$!
sleep 5

# Verificar que Nodo 3 está corriendo
if ! kill -0 $NODE3_PID 2>/dev/null; then
    echo "❌ Error: Nodo 3 no inició correctamente"
    cat /tmp/node3_seed.log
    exit 1
fi

echo "✅ Nodo 3 iniciado (PID: $NODE3_PID)"
sleep 5

# Verificar health del Nodo 3
echo "🔍 Verificando health del Nodo 3..."
for i in {1..10}; do
    if curl -s "http://127.0.0.1:${API_PORT_3}/api/v1/health" > /dev/null 2>&1; then
        echo "✅ Nodo 3 está respondiendo"
        break
    fi
    if [ $i -eq 10 ]; then
        echo "❌ Error: Nodo 3 no responde después de 10 intentos"
        cat /tmp/node3_seed.log
        exit 1
    fi
    sleep 1
done

# Esperar a que el auto-discovery descubra y conecte
echo ""
echo "⏳ Esperando auto-discovery del Nodo 3 (puede tomar hasta 90 segundos)..."
sleep 90

# Verificar que Nodo 3 se conectó a Nodo 1 (vía seed node)
echo ""
echo "🔍 Verificando conexión Nodo 3 -> Nodo 1..."
NODE3_PEERS=$(curl -s "http://127.0.0.1:${API_PORT_3}/api/v1/peers" | jq -r '.data[]' 2>/dev/null || echo "")

CONNECTED_TO_NODE1=false
CONNECTED_TO_NODE2=false

if echo "$NODE3_PEERS" | grep -q "127.0.0.1:${P2P_PORT_1}"; then
    echo "✅ Nodo 3 se conectó a Nodo 1 vía seed node"
    CONNECTED_TO_NODE1=true
else
    echo "⚠️  Nodo 3 aún no se conectó a Nodo 1"
    echo "   Peers de Nodo 3: $NODE3_PEERS"
fi

# Verificar que Nodo 3 descubrió y se conectó a Nodo 2 (vía auto-discovery)
if echo "$NODE3_PEERS" | grep -q "127.0.0.1:${P2P_PORT_2}"; then
    echo "✅ Nodo 3 descubrió y se conectó a Nodo 2 vía auto-discovery"
    CONNECTED_TO_NODE2=true
else
    echo "⚠️  Nodo 3 aún no descubrió a Nodo 2"
    echo "   Peers de Nodo 3: $NODE3_PEERS"
fi

# Verificar que todos los nodos se ven entre sí
echo ""
echo "🔍 Verificando conectividad completa..."
sleep 5

NODE1_PEERS_FINAL=$(curl -s "http://127.0.0.1:${API_PORT_1}/api/v1/peers" | jq -r '.data[]' 2>/dev/null || echo "")
NODE2_PEERS_FINAL=$(curl -s "http://127.0.0.1:${API_PORT_2}/api/v1/peers" | jq -r '.data[]' 2>/dev/null || echo "")
NODE3_PEERS_FINAL=$(curl -s "http://127.0.0.1:${API_PORT_3}/api/v1/peers" | jq -r '.data[]' 2>/dev/null || echo "")

echo ""
echo "📊 Estado Final de Conexiones:"
echo "  Nodo 1 peers: $NODE1_PEERS_FINAL"
echo "  Nodo 2 peers: $NODE2_PEERS_FINAL"
echo "  Nodo 3 peers: $NODE3_PEERS_FINAL"
echo ""

# Resultados finales
SUCCESS=true

if [ "$CONNECTED_TO_NODE1" = true ] && [ "$CONNECTED_TO_NODE2" = true ]; then
    echo "✅ TEST EXITOSO: Seed nodes funcionan correctamente"
    echo "   - Nodo 3 se conectó a Nodo 1 vía seed node"
    echo "   - Nodo 3 descubrió a Nodo 2 vía auto-discovery"
else
    echo "⚠️  TEST PARCIAL: Algunas conexiones no se establecieron"
    if [ "$CONNECTED_TO_NODE1" = false ]; then
        echo "   - Nodo 3 NO se conectó a Nodo 1"
    fi
    if [ "$CONNECTED_TO_NODE2" = false ]; then
        echo "   - Nodo 3 NO descubrió a Nodo 2"
    fi
    SUCCESS=false
fi

# Verificar logs para debugging
if [ "$SUCCESS" = false ]; then
    echo ""
    echo "📋 Últimas líneas de logs:"
    echo "--- Nodo 1 ---"
    tail -10 /tmp/node1_seed.log
    echo ""
    echo "--- Nodo 2 ---"
    tail -10 /tmp/node2_seed.log
    echo ""
    echo "--- Nodo 3 ---"
    tail -10 /tmp/node3_seed.log
fi

if [ "$SUCCESS" = true ]; then
    echo ""
    echo "🎉 Todos los tests pasaron exitosamente!"
    exit 0
else
    echo ""
    echo "❌ Algunos tests fallaron"
    exit 1
fi

