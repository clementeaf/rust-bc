#!/bin/bash

echo "🧪 PRUEBA DE CONSENSO DISTRIBUIDO"
echo "================================="
echo ""

# Limpiar procesos anteriores
pkill -f "target/release/rust-bc" 2>/dev/null || true
sleep 2

cd /Users/clementefalcone/Desktop/personal/rust-bc
export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"

# Función helper para API
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

# Función para esperar que un servidor esté listo
wait_for_server() {
    local port=$1
    local max_attempts=30
    local attempt=0
    
    while [ $attempt -lt $max_attempts ]; do
        if curl -s "http://127.0.0.1:${port}/api/v1/chain/info" > /dev/null 2>&1; then
            return 0
        fi
        sleep 1
        attempt=$((attempt + 1))
    done
    return 1
}

echo "🚀 TEST 1: Iniciar 3 nodos"
echo "-------------------------"
echo ""

echo "📡 Iniciando Nodo 1 (API:8080, P2P:8081)..."
cargo run --release 8080 8081 node1 > /tmp/nodo1.log 2>&1 &
NODE1_PID=$!
sleep 5

echo "📡 Iniciando Nodo 2 (API:8082, P2P:8083)..."
cargo run --release 8082 8083 node2 > /tmp/nodo2.log 2>&1 &
NODE2_PID=$!
sleep 5

echo "📡 Iniciando Nodo 3 (API:8084, P2P:8085)..."
cargo run --release 8084 8085 node3 > /tmp/nodo3.log 2>&1 &
NODE3_PID=$!
sleep 5

# Verificar que todos están corriendo
if ! wait_for_server 8080 || ! wait_for_server 8082 || ! wait_for_server 8084; then
    echo "❌ Error: Algunos nodos no iniciaron correctamente"
    exit 1
fi

echo "✅ Todos los nodos están corriendo"
echo ""

echo "📊 Estado inicial de los nodos:"
INFO1=$(api 8080 GET "/api/v1/chain/info")
INFO2=$(api 8082 GET "/api/v1/chain/info")
INFO3=$(api 8084 GET "/api/v1/chain/info")

BLOCKS1=$(echo "$INFO1" | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['block_count'])" 2>/dev/null || echo "0")
BLOCKS2=$(echo "$INFO2" | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['block_count'])" 2>/dev/null || echo "0")
BLOCKS3=$(echo "$INFO3" | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['block_count'])" 2>/dev/null || echo "0")

HASH1=$(echo "$INFO1" | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['latest_block_hash'])" 2>/dev/null || echo "N/A")
HASH2=$(echo "$INFO2" | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['latest_block_hash'])" 2>/dev/null || echo "N/A")
HASH3=$(echo "$INFO3" | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['latest_block_hash'])" 2>/dev/null || echo "N/A")

echo "   Nodo 1: $BLOCKS1 bloques, hash: ${HASH1:0:16}..."
echo "   Nodo 2: $BLOCKS2 bloques, hash: ${HASH2:0:16}..."
echo "   Nodo 3: $BLOCKS3 bloques, hash: ${HASH3:0:16}..."
echo ""

echo "🔗 TEST 2: Conectar nodos (sincronización automática)"
echo "-----------------------------------------------------"
echo ""

echo "Conectando Nodo 1 → Nodo 2..."
api 8080 POST "/api/v1/peers/127.0.0.1:8083/connect" > /dev/null
sleep 3

echo "Conectando Nodo 2 → Nodo 3..."
api 8082 POST "/api/v1/peers/127.0.0.1:8085/connect" > /dev/null
sleep 3

echo "Conectando Nodo 3 → Nodo 1..."
api 8084 POST "/api/v1/peers/127.0.0.1:8081/connect" > /dev/null
sleep 3

echo "📊 Estado después de conectar:"
INFO1=$(api 8080 GET "/api/v1/chain/info")
INFO2=$(api 8082 GET "/api/v1/chain/info")
INFO3=$(api 8084 GET "/api/v1/chain/info")

BLOCKS1=$(echo "$INFO1" | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['block_count'])" 2>/dev/null || echo "0")
BLOCKS2=$(echo "$INFO2" | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['block_count'])" 2>/dev/null || echo "0")
BLOCKS3=$(echo "$INFO3" | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['block_count'])" 2>/dev/null || echo "0")

echo "   Nodo 1: $BLOCKS1 bloques"
echo "   Nodo 2: $BLOCKS2 bloques"
echo "   Nodo 3: $BLOCKS3 bloques"
echo ""

echo "💰 TEST 3: Crear bloques en diferentes nodos"
echo "---------------------------------------------"
echo ""

echo "Creando wallet y bloque en Nodo 1..."
WALLET1=$(api 8080 POST "/api/v1/wallets/create")
WALLET1_ADDR=$(echo "$WALLET1" | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['address'])" 2>/dev/null)

if [ -n "$WALLET1_ADDR" ]; then
    BLOCK_DATA="{\"transactions\":[{\"from\":\"0\",\"to\":\"$WALLET1_ADDR\",\"amount\":1000}]}"
    api 8080 POST "/api/v1/blocks" "$BLOCK_DATA" > /dev/null
    echo "   ✅ Bloque creado en Nodo 1"
    sleep 3
fi

echo "Creando wallet y bloque en Nodo 2..."
WALLET2=$(api 8082 POST "/api/v1/wallets/create")
WALLET2_ADDR=$(echo "$WALLET2" | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['address'])" 2>/dev/null)

if [ -n "$WALLET2_ADDR" ]; then
    BLOCK_DATA="{\"transactions\":[{\"from\":\"0\",\"to\":\"$WALLET2_ADDR\",\"amount\":500}]}"
    api 8082 POST "/api/v1/blocks" "$BLOCK_DATA" > /dev/null
    echo "   ✅ Bloque creado en Nodo 2"
    sleep 3
fi

echo "Creando wallet y bloque en Nodo 3..."
WALLET3=$(api 8084 POST "/api/v1/wallets/create")
WALLET3_ADDR=$(echo "$WALLET3" | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['address'])" 2>/dev/null)

if [ -n "$WALLET3_ADDR" ]; then
    BLOCK_DATA="{\"transactions\":[{\"from\":\"0\",\"to\":\"$WALLET3_ADDR\",\"amount\":750}]}"
    api 8084 POST "/api/v1/blocks" "$BLOCK_DATA" > /dev/null
    echo "   ✅ Bloque creado en Nodo 3"
    sleep 3
fi

echo ""
echo "📊 Estado después de crear bloques:"
INFO1=$(api 8080 GET "/api/v1/chain/info")
INFO2=$(api 8082 GET "/api/v1/chain/info")
INFO3=$(api 8084 GET "/api/v1/chain/info")

BLOCKS1=$(echo "$INFO1" | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['block_count'])" 2>/dev/null || echo "0")
BLOCKS2=$(echo "$INFO2" | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['block_count'])" 2>/dev/null || echo "0")
BLOCKS3=$(echo "$INFO3" | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['block_count'])" 2>/dev/null || echo "0")

HASH1=$(echo "$INFO1" | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['latest_block_hash'])" 2>/dev/null || echo "N/A")
HASH2=$(echo "$INFO2" | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['latest_block_hash'])" 2>/dev/null || echo "N/A")
HASH3=$(echo "$INFO3" | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['latest_block_hash'])" 2>/dev/null || echo "N/A")

echo "   Nodo 1: $BLOCKS1 bloques, hash: ${HASH1:0:16}..."
echo "   Nodo 2: $BLOCKS2 bloques, hash: ${HASH2:0:16}..."
echo "   Nodo 3: $BLOCKS3 bloques, hash: ${HASH3:0:16}..."
echo ""

echo "🔄 TEST 4: Sincronización manual"
echo "---------------------------------"
echo ""

echo "Sincronizando Nodo 1 con todos los peers..."
api 8080 POST "/api/v1/sync" > /dev/null
sleep 3

echo "Sincronizando Nodo 2 con todos los peers..."
api 8082 POST "/api/v1/sync" > /dev/null
sleep 3

echo "Sincronizando Nodo 3 con todos los peers..."
api 8084 POST "/api/v1/sync" > /dev/null
sleep 3

echo "📊 Estado final después de sincronización:"
INFO1=$(api 8080 GET "/api/v1/chain/info")
INFO2=$(api 8082 GET "/api/v1/chain/info")
INFO3=$(api 8084 GET "/api/v1/chain/info")

BLOCKS1=$(echo "$INFO1" | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['block_count'])" 2>/dev/null || echo "0")
BLOCKS2=$(echo "$INFO2" | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['block_count'])" 2>/dev/null || echo "0")
BLOCKS3=$(echo "$INFO3" | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['block_count'])" 2>/dev/null || echo "0")

HASH1=$(echo "$INFO1" | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['latest_block_hash'])" 2>/dev/null || echo "N/A")
HASH2=$(echo "$INFO2" | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['latest_block_hash'])" 2>/dev/null || echo "N/A")
HASH3=$(echo "$INFO3" | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['latest_block_hash'])" 2>/dev/null || echo "N/A")

echo "   Nodo 1: $BLOCKS1 bloques, hash: ${HASH1:0:16}..."
echo "   Nodo 2: $BLOCKS2 bloques, hash: ${HASH2:0:16}..."
echo "   Nodo 3: $BLOCKS3 bloques, hash: ${HASH3:0:16}..."
echo ""

# Verificar consenso
if [ "$BLOCKS1" = "$BLOCKS2" ] && [ "$BLOCKS2" = "$BLOCKS3" ]; then
    if [ "$HASH1" = "$HASH2" ] && [ "$HASH2" = "$HASH3" ]; then
        echo "✅ CONSENSO ALCANZADO: Todos los nodos tienen la misma cadena"
    else
        echo "⚠️  Mismo número de bloques pero diferentes hashes (posible fork)"
    fi
else
    echo "⚠️  Los nodos no están sincronizados"
fi

echo ""
echo "📝 Revisando logs para eventos de consenso:"
echo "   Forks detectados:"
grep -c "Fork detectado" /tmp/nodo*.log 2>/dev/null | head -3 || echo "   0"
echo "   Sincronizaciones:"
grep -c "Sincronizando" /tmp/nodo*.log 2>/dev/null | head -3 || echo "   0"
echo "   Bloques recibidos:"
grep -c "Nuevo bloque recibido" /tmp/nodo*.log 2>/dev/null | head -3 || echo "   0"

echo ""
echo "=========================================="
echo "✅ PRUEBA DE CONSENSO COMPLETADA"
echo "=========================================="
echo ""

# Detener nodos
pkill -f "target/release/rust-bc" 2>/dev/null || true

echo "📝 Logs disponibles en:"
echo "   /tmp/nodo1.log"
echo "   /tmp/nodo2.log"
echo "   /tmp/nodo3.log"

