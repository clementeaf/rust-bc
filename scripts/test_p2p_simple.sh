#!/bin/bash

# Script simplificado para probar P2P con 2 nodos

echo "🧪 PRUEBA SIMPLE DE RED P2P"
echo "============================"
echo ""

# Limpiar procesos anteriores
pkill -f "target/release/rust-bc" 2>/dev/null || true
sleep 2

cd /Users/clementefalcone/Desktop/personal/rust-bc
export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"

echo "🚀 Iniciando Nodo 1 (API:8080, P2P:8081)..."
cargo run --release 8080 8081 node1 > /tmp/node1.log 2>&1 &
NODE1_PID=$!
sleep 5

echo "🚀 Iniciando Nodo 2 (API:8082, P2P:8083)..."
cargo run --release 8082 8083 node2 > /tmp/node2.log 2>&1 &
NODE2_PID=$!
sleep 5

echo ""
echo "✅ Nodos iniciados"
echo ""

# Función helper
api() {
    local port=$1
    local method=$2
    local endpoint=$3
    local data=$4
    
    if [ -z "$data" ]; then
        curl -s -X "$method" "http://127.0.0.1:${port}${endpoint}"
    else
        curl -s -X "$method" "http://127.0.0.1:${port}${endpoint}" \
            -H "Content-Type: application/json" \
            -d "$data"
    fi
}

# Verificar que los nodos están corriendo
echo "📊 Verificando nodos..."
NODE1_OK=$(api 8080 GET "/api/v1/chain/info" | grep -q "success" && echo "✅" || echo "❌")
NODE2_OK=$(api 8082 GET "/api/v1/chain/info" | grep -q "success" && echo "✅" || echo "❌")

echo "   Nodo 1: $NODE1_OK"
echo "   Nodo 2: $NODE2_OK"
echo ""

if [ "$NODE1_OK" != "✅" ] || [ "$NODE2_OK" != "✅" ]; then
    echo "❌ Los nodos no están respondiendo. Revisa los logs:"
    echo "   tail -f /tmp/node1.log"
    echo "   tail -f /tmp/node2.log"
    exit 1
fi

# Conectar nodos
echo "🔗 Conectando Nodo 1 → Nodo 2..."
CONNECT_RESPONSE=$(api 8080 POST "/api/v1/peers/127.0.0.1:8083/connect")
if echo "$CONNECT_RESPONSE" | grep -q "success\|Conectando"; then
    echo "   ✅ Conexión iniciada"
else
    echo "   ⚠️  Respuesta: $CONNECT_RESPONSE"
fi
sleep 3

# Verificar peers
echo ""
echo "👥 Verificando peers conectados..."
PEERS1=$(api 8080 GET "/api/v1/peers")
PEERS2=$(api 8082 GET "/api/v1/peers")
echo "   Nodo 1 peers:"
echo "$PEERS1" | python3 -m json.tool 2>/dev/null || echo "$PEERS1"
echo ""
echo "   Nodo 2 peers:"
echo "$PEERS2" | python3 -m json.tool 2>/dev/null || echo "$PEERS2"
echo ""

# Crear wallet y bloque
echo "💰 Creando wallet y bloque en Nodo 1..."
WALLET=$(api 8080 POST "/api/v1/wallets/create")
WALLET_ADDR=$(echo "$WALLET" | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['address'])" 2>/dev/null)

if [ -n "$WALLET_ADDR" ]; then
    echo "   Wallet: ${WALLET_ADDR:0:20}..."
    
    BLOCK_DATA="{\"transactions\":[{\"from\":\"0\",\"to\":\"$WALLET_ADDR\",\"amount\":1000}]}"
    BLOCK_RESPONSE=$(api 8080 POST "/api/v1/blocks" "$BLOCK_DATA")
    
    if echo "$BLOCK_RESPONSE" | grep -q "success"; then
        echo "   ✅ Bloque creado"
        echo "   ⏳ Esperando propagación (5 segundos)..."
        sleep 5
    else
        echo "   ⚠️  Respuesta: $BLOCK_RESPONSE"
    fi
else
    echo "   ❌ Error al crear wallet"
fi

# Verificar sincronización
echo ""
echo "🔄 Verificando sincronización..."
INFO1=$(api 8080 GET "/api/v1/chain/info")
INFO2=$(api 8082 GET "/api/v1/chain/info")

BLOCKS1=$(echo "$INFO1" | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['block_count'])" 2>/dev/null || echo "0")
BLOCKS2=$(echo "$INFO2" | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['block_count'])" 2>/dev/null || echo "0")

echo "   Nodo 1: $BLOCKS1 bloques"
echo "   Nodo 2: $BLOCKS2 bloques"

if [ "$BLOCKS1" = "$BLOCKS2" ]; then
    echo "   ✅ Nodos sincronizados"
else
    echo "   ⚠️  Los nodos tienen diferente número de bloques"
    echo "      (Puede ser normal si la sincronización es asíncrona)"
fi

echo ""
echo "=========================================="
echo "✅ PRUEBA COMPLETADA"
echo "=========================================="
echo ""
echo "📝 Logs disponibles en:"
echo "   /tmp/node1.log"
echo "   /tmp/node2.log"
echo ""
echo "💡 Los nodos siguen corriendo."
echo "   Para detenerlos: pkill -f 'target/release/rust-bc'"
echo ""
echo "🔍 Para ver logs en tiempo real:"
echo "   tail -f /tmp/node1.log"
echo "   tail -f /tmp/node2.log"
echo ""

