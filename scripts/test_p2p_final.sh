#!/bin/bash

echo "🧪 PRUEBA FINAL DE RED P2P CON MEJORAS"
echo "========================================"
echo ""

pkill -f "target/release/rust-bc" 2>/dev/null || true
sleep 2

cd /Users/clementefalcone/Desktop/personal/rust-bc
export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"

echo "🚀 Iniciando Nodo 1 (API:8080, P2P:8081)..."
cargo run --release 8080 8081 node1 > /tmp/node1_final.log 2>&1 &
NODE1_PID=$!
sleep 5

echo "🚀 Iniciando Nodo 2 (API:8082, P2P:8083)..."
cargo run --release 8082 8083 node2 > /tmp/node2_final.log 2>&1 &
NODE2_PID=$!
sleep 5

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

echo "📊 Estado inicial:"
INFO1=$(api 8080 GET "/api/v1/chain/info")
INFO2=$(api 8082 GET "/api/v1/chain/info")
BLOCKS1=$(echo "$INFO1" | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['block_count'])" 2>/dev/null || echo "0")
BLOCKS2=$(echo "$INFO2" | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['block_count'])" 2>/dev/null || echo "0")
echo "   Nodo 1: $BLOCKS1 bloques"
echo "   Nodo 2: $BLOCKS2 bloques"
echo ""

echo "🔗 Conectando Nodo 1 → Nodo 2..."
api 8080 POST "/api/v1/peers/127.0.0.1:8083/connect" > /dev/null
sleep 3

echo "📊 Después de conectar:"
INFO1=$(api 8080 GET "/api/v1/chain/info")
INFO2=$(api 8082 GET "/api/v1/chain/info")
BLOCKS1=$(echo "$INFO1" | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['block_count'])" 2>/dev/null || echo "0")
BLOCKS2=$(echo "$INFO2" | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['block_count'])" 2>/dev/null || echo "0")
echo "   Nodo 1: $BLOCKS1 bloques"
echo "   Nodo 2: $BLOCKS2 bloques"
echo ""

echo "💰 Creando wallet y bloque en Nodo 1..."
WALLET=$(api 8080 POST "/api/v1/wallets/create")
WALLET_ADDR=$(echo "$WALLET" | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['address'])" 2>/dev/null)

if [ -n "$WALLET_ADDR" ]; then
    echo "   Wallet: ${WALLET_ADDR:0:20}..."
    
    BLOCK_DATA="{\"transactions\":[{\"from\":\"0\",\"to\":\"$WALLET_ADDR\",\"amount\":1000}]}"
    BLOCK_RESPONSE=$(api 8080 POST "/api/v1/blocks" "$BLOCK_DATA")
    
    if echo "$BLOCK_RESPONSE" | grep -q "success"; then
        echo "   ✅ Bloque creado"
        echo "   ⏳ Esperando propagación (7 segundos)..."
        sleep 7
    fi
fi

echo ""
echo "🔄 Verificando sincronización final:"
INFO1=$(api 8080 GET "/api/v1/chain/info")
INFO2=$(api 8082 GET "/api/v1/chain/info")
BLOCKS1=$(echo "$INFO1" | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['block_count'])" 2>/dev/null || echo "0")
BLOCKS2=$(echo "$INFO2" | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['block_count'])" 2>/dev/null || echo "0")

echo "   Nodo 1: $BLOCKS1 bloques"
echo "   Nodo 2: $BLOCKS2 bloques"

if [ "$BLOCKS1" = "$BLOCKS2" ] && [ "$BLOCKS1" -gt 1 ]; then
    echo "   ✅ Nodos sincronizados correctamente!"
    echo ""
    echo "📝 Revisando logs para confirmar procesamiento:"
    echo "   Nodo 2 recibió bloque:" $(grep -c "Nuevo bloque recibido" /tmp/node2_final.log 2>/dev/null || echo "0") "veces"
    echo "   Nodo 2 guardó en BD:" $(grep -c "guardando bloque" /tmp/node2_final.log 2>/dev/null || echo "0") "veces"
else
    echo "   ⚠️  Los nodos no están sincronizados"
    echo ""
    echo "📝 Revisando logs:"
    tail -10 /tmp/node1_final.log | grep -E "(bloque|Error|recibido)" || tail -5 /tmp/node1_final.log
    echo ""
    tail -10 /tmp/node2_final.log | grep -E "(bloque|Error|recibido)" || tail -5 /tmp/node2_final.log
fi

echo ""
echo "=========================================="
echo "✅ PRUEBA COMPLETADA"
echo "=========================================="
echo ""
pkill -f "target/release/rust-bc" 2>/dev/null || true

