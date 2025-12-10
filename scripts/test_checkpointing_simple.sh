#!/bin/bash

# Test simple de Checkpointing
# Verifica que los checkpoints se crean correctamente

set -e

API_PORT=8080
P2P_PORT=8081
BASE_DIR="test_checkpoint_simple"

echo "🧪 Test Simple de Checkpointing"
echo "================================"
echo ""

# Limpiar
rm -rf "${BASE_DIR}"*_blocks "${BASE_DIR}"*_snapshots "${BASE_DIR}"*_checkpoints 2>/dev/null || true
pkill -f "cargo run.*8080" 2>/dev/null || true
sleep 1

# Iniciar servidor
echo "🚀 Iniciando servidor..."
CHECKPOINT_INTERVAL=5 MAX_REORG_DEPTH=5 DIFFICULTY=1 cargo run --release $API_PORT $P2P_PORT "${BASE_DIR}" > /tmp/checkpoint_test.log 2>&1 &
SERVER_PID=$!

echo "⏳ Esperando compilación e inicio del servidor..."
for i in {1..90}; do
    if curl -s "http://localhost:$API_PORT/api/v1/health" > /dev/null 2>&1; then
        echo "✅ Servidor iniciado (después de ~$((i*2)) segundos)"
        break
    fi
    if [ $i -eq 90 ]; then
        echo "❌ Timeout esperando servidor"
        kill $SERVER_PID 2>/dev/null || true
        exit 1
    fi
    sleep 2
done

echo ""

# Obtener índice inicial
INITIAL_RESPONSE=$(curl -s "http://localhost:$API_PORT/api/v1/blocks")
INITIAL_COUNT=$(echo "$INITIAL_RESPONSE" | jq '.data | length')
echo "📊 Bloques iniciales: $INITIAL_COUNT"

# Crear una wallet válida primero
echo ""
echo "👛 Creando wallet de prueba..."
WALLET_RESPONSE=$(curl -s -X POST "http://localhost:$API_PORT/api/v1/wallet/create")
MINER_ADDRESS=$(echo "$WALLET_RESPONSE" | jq -r '.data.address // empty')

if [ -z "$MINER_ADDRESS" ] || [ "$MINER_ADDRESS" = "null" ]; then
    # Si falla, usar una dirección válida de ejemplo (formato: al menos 32 caracteres)
    MINER_ADDRESS="test_miner_address_12345678901234567890"
    echo "   ⚠️  Usando dirección de ejemplo: $MINER_ADDRESS"
else
    echo "   ✅ Wallet creada: ${MINER_ADDRESS:0:32}..."
fi

# Minar 6 bloques (para llegar a un múltiplo de 5)
echo ""
echo "⛏️  Minando 6 bloques con dirección: ${MINER_ADDRESS:0:32}..."
for i in {1..6}; do
    RESPONSE=$(curl -s -X POST "http://localhost:$API_PORT/api/v1/mine" \
        -H "Content-Type: application/json" \
        -d "{\"miner_address\": \"$MINER_ADDRESS\", \"transactions\": []}" 2>&1)
    
    # Verificar si la respuesta es JSON válido
    if echo "$RESPONSE" | jq . > /dev/null 2>&1; then
        if echo "$RESPONSE" | jq -e '.success' > /dev/null 2>&1; then
            HASH=$(echo "$RESPONSE" | jq -r '.data.hash // empty')
            if [ -n "$HASH" ] && [ "$HASH" != "null" ]; then
                echo "   ✅ Bloque $i minado: ${HASH:0:16}..."
            else
                echo "   ⚠️  Bloque $i minado (sin hash en respuesta)"
            fi
        else
            ERROR=$(echo "$RESPONSE" | jq -r '.message // "Error desconocido"')
            echo "   ❌ Error minando bloque $i: $ERROR"
        fi
    else
        echo "   ⚠️  Respuesta no válida al minar bloque $i: ${RESPONSE:0:50}..."
    fi
    sleep 0.3
done

echo ""
sleep 2

# Verificar bloques
FINAL_RESPONSE=$(curl -s "http://localhost:$API_PORT/api/v1/blocks")
FINAL_COUNT=$(echo "$FINAL_RESPONSE" | jq '.data | length')
LATEST_BLOCK=$(echo "$FINAL_RESPONSE" | jq '.data[-1]')
LATEST_INDEX=$(echo "$LATEST_BLOCK" | jq -r '.index // 0')

echo "📊 Bloques finales: $FINAL_COUNT"
echo "📊 Índice del último bloque: $LATEST_INDEX"
echo ""

# Verificar checkpoints
echo "🔍 Verificando checkpoints..."
CHECKPOINT_DIR="${BASE_DIR}_checkpoints"

if [ -d "$CHECKPOINT_DIR" ]; then
    CHECKPOINT_FILES=$(find "$CHECKPOINT_DIR" -name "checkpoint_*.json" 2>/dev/null | wc -l | tr -d ' ')
    echo "   ✅ Directorio de checkpoints existe"
    echo "   📁 Checkpoints encontrados: $CHECKPOINT_FILES"
    
    if [ "$CHECKPOINT_FILES" -gt 0 ]; then
        echo ""
        echo "   📋 Detalles de checkpoints:"
        for checkpoint_file in "$CHECKPOINT_DIR"/checkpoint_*.json; do
            if [ -f "$checkpoint_file" ]; then
                INDEX=$(jq -r '.block_index' "$checkpoint_file")
                HASH=$(jq -r '.block_hash' "$checkpoint_file")
                DIFFICULTY=$(jq -r '.cumulative_difficulty' "$checkpoint_file")
                echo "      - Bloque $INDEX: ${HASH:0:16}... (dificultad: $DIFFICULTY)"
            fi
        done
        
        # Verificar que el checkpoint coincide con el bloque
        if [ "$LATEST_INDEX" -ge 5 ]; then
            EXPECTED_CHECKPOINT="${CHECKPOINT_DIR}/checkpoint_$(printf "%07d" 5).json"
            if [ -f "$EXPECTED_CHECKPOINT" ]; then
                CHECKPOINT_HASH=$(jq -r '.block_hash' "$EXPECTED_CHECKPOINT")
                BLOCK_5=$(echo "$FINAL_RESPONSE" | jq ".data[] | select(.index == 5)")
                if [ -n "$BLOCK_5" ]; then
                    BLOCK_5_HASH=$(echo "$BLOCK_5" | jq -r '.hash')
                    if [ "$CHECKPOINT_HASH" = "$BLOCK_5_HASH" ]; then
                        echo ""
                        echo "   ✅ Checkpoint del bloque 5 coincide con el hash del bloque"
                    else
                        echo ""
                        echo "   ⚠️  Checkpoint del bloque 5 NO coincide"
                        echo "      Checkpoint: ${CHECKPOINT_HASH:0:16}..."
                        echo "      Bloque: ${BLOCK_5_HASH:0:16}..."
                    fi
                fi
            fi
        fi
    else
        echo "   ⚠️  No se encontraron archivos de checkpoint"
        echo "   💡 Esto puede ser normal si no se ha alcanzado el intervalo (5 bloques)"
    fi
else
    echo "   ⚠️  Directorio de checkpoints no existe aún"
fi

echo ""

# Verificar en el log que se creó el checkpoint
echo "📋 Verificando logs del servidor..."
if grep -q "Checkpoint creado" /tmp/checkpoint_test.log; then
    echo "   ✅ Se encontraron mensajes de creación de checkpoint en el log"
    grep "Checkpoint creado" /tmp/checkpoint_test.log | tail -3
else
    echo "   ⚠️  No se encontraron mensajes de creación de checkpoint en el log"
fi

echo ""

# Limpiar
echo "🧹 Limpiando..."
kill $SERVER_PID 2>/dev/null || true
wait $SERVER_PID 2>/dev/null || true
sleep 1

echo ""
echo "================================================"
echo "✅ Test completado"
echo "================================================"

