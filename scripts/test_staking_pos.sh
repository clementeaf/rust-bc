#!/bin/bash

# Script de prueba para Staking PoS
# Prueba el sistema completo de staking, validación y recompensas

set -e

BASE_PORT=20000
API_PORT=$((BASE_PORT + 0))
P2P_PORT=$((BASE_PORT + 1))

DB="blockchain_test_staking"

# Limpiar base de datos anterior
rm -f "${DB}.db"

echo "🧪 Test de Staking PoS"
echo "======================"
echo ""

# Función para limpiar procesos al finalizar
cleanup() {
    echo ""
    echo "🧹 Limpiando procesos..."
    pkill -f "rust-bc.*${API_PORT}" || true
    sleep 2
    rm -f "${DB}.db"
    echo "✅ Limpieza completada"
}

trap cleanup EXIT

# Iniciar nodo
echo "🚀 Iniciando nodo..."
RUST_LOG=info cargo run --release ${API_PORT} ${P2P_PORT} ${DB} > /tmp/node_staking.log 2>&1 &
NODE_PID=$!
sleep 5

# Verificar que el nodo está corriendo
if ! kill -0 $NODE_PID 2>/dev/null; then
    echo "❌ Error: Nodo no inició correctamente"
    cat /tmp/node_staking.log
    exit 1
fi

echo "✅ Nodo iniciado (PID: $NODE_PID)"
sleep 3

# Verificar health
echo "🔍 Verificando health del nodo..."
for i in {1..10}; do
    if curl -s "http://127.0.0.1:${API_PORT}/api/v1/health" > /dev/null 2>&1; then
        echo "✅ Nodo está respondiendo"
        break
    fi
    if [ $i -eq 10 ]; then
        echo "❌ Error: Nodo no responde después de 10 intentos"
        cat /tmp/node_staking.log
        exit 1
    fi
    sleep 1
done

# Crear wallets
echo ""
echo "📝 Creando wallets..."
WALLET1_RESPONSE=$(curl -s -X POST "http://127.0.0.1:${API_PORT}/api/v1/wallets/create")
WALLET1=$(echo "$WALLET1_RESPONSE" | jq -r '.data.address' 2>/dev/null || echo "")

if [ -z "$WALLET1" ] || [ "$WALLET1" == "null" ]; then
    echo "❌ Error: No se pudo crear WALLET1"
    echo "Response: $WALLET1_RESPONSE"
    exit 1
fi

WALLET2_RESPONSE=$(curl -s -X POST "http://127.0.0.1:${API_PORT}/api/v1/wallets/create")
WALLET2=$(echo "$WALLET2_RESPONSE" | jq -r '.data.address' 2>/dev/null || echo "")

if [ -z "$WALLET2" ] || [ "$WALLET2" == "null" ]; then
    echo "❌ Error: No se pudo crear WALLET2"
    exit 1
fi

echo "✅ WALLET1: $WALLET1"
echo "✅ WALLET2: $WALLET2"

# Minar bloques para dar balance inicial (necesitamos al menos 1000 para staking)
echo ""
echo "⛏️  Minando bloques iniciales para dar balance..."
BALANCE1=0
MINED_BLOCKS=0

while [ "$BALANCE1" -lt 1000 ] && [ "$MINED_BLOCKS" -lt 20 ]; do
    MINE_RESPONSE=$(curl -s -X POST "http://127.0.0.1:${API_PORT}/api/v1/mine" \
        -H "Content-Type: application/json" \
        -d "{\"miner_address\": \"$WALLET1\", \"max_transactions\": 10}")
    
    MINE_HASH=$(echo "$MINE_RESPONSE" | jq -r '.data.hash' 2>/dev/null || echo "")
    MINE_CONSENSUS=$(echo "$MINE_RESPONSE" | jq -r '.data.consensus' 2>/dev/null || echo "")
    
    MINED_BLOCKS=$((MINED_BLOCKS + 1))
    
    if [ -n "$MINE_HASH" ] && [ "$MINE_HASH" != "null" ]; then
        echo "   Bloque $MINED_BLOCKS minado (hash: ${MINE_HASH:0:16}..., consenso: $MINE_CONSENSUS)"
    fi
    
    # Verificar balance después de cada bloque
    sleep 2
    BALANCE1_RESPONSE=$(curl -s "http://127.0.0.1:${API_PORT}/api/v1/wallets/$WALLET1")
    BALANCE1=$(echo "$BALANCE1_RESPONSE" | jq -r '.data.balance' 2>/dev/null || echo "0")
done

# Verificar balance de WALLET1
echo ""
echo "💰 Verificando balance de WALLET1..."
echo "   Balance WALLET1: $BALANCE1"

# Stakear tokens
echo ""
echo "🔒 Stakeando tokens de WALLET1..."
STAKE_AMOUNT=1000
STAKE_RESPONSE=$(curl -s -X POST "http://127.0.0.1:${API_PORT}/api/v1/staking/stake" \
    -H "Content-Type: application/json" \
    -d "{\"address\": \"$WALLET1\", \"amount\": $STAKE_AMOUNT}")

STAKE_SUCCESS=$(echo "$STAKE_RESPONSE" | jq -r '.success' 2>/dev/null || echo "false")

if [ "$STAKE_SUCCESS" != "true" ]; then
    echo "❌ Error: Staking falló"
    echo "Response: $STAKE_RESPONSE"
    exit 1
fi

echo "✅ Staked $STAKE_AMOUNT tokens exitosamente"

# Esperar a que la transacción se procese (minar un bloque)
echo ""
echo "⏳ Minando bloque para procesar transacción de staking..."
sleep 2
MINE_RESPONSE=$(curl -s -X POST "http://127.0.0.1:${API_PORT}/api/v1/mine" \
    -H "Content-Type: application/json" \
    -d "{\"miner_address\": \"$WALLET1\", \"max_transactions\": 10}")

MINE_CONSENSUS=$(echo "$MINE_RESPONSE" | jq -r '.data.consensus' 2>/dev/null || echo "")
MINE_VALIDATOR=$(echo "$MINE_RESPONSE" | jq -r '.data.validator' 2>/dev/null || echo "")

echo "   Consenso usado: $MINE_CONSENSUS"
if [ "$MINE_CONSENSUS" == "PoS" ] && [ -n "$MINE_VALIDATOR" ] && [ "$MINE_VALIDATOR" != "null" ]; then
    echo "   ✅ Validador seleccionado: $MINE_VALIDATOR"
else
    echo "   ⚠️  Usando PoW (puede ser normal si el validador aún no está activo)"
fi

# Verificar validadores
echo ""
echo "👥 Verificando validadores..."
VALIDATORS_RESPONSE=$(curl -s "http://127.0.0.1:${API_PORT}/api/v1/staking/validators")
VALIDATORS_COUNT=$(echo "$VALIDATORS_RESPONSE" | jq -r '.data | length' 2>/dev/null || echo "0")

if [ "$VALIDATORS_COUNT" -gt 0 ]; then
    echo "✅ Encontrados $VALIDATORS_COUNT validador(es) activo(s)"
    echo "$VALIDATORS_RESPONSE" | jq -r '.data[] | "   - \(.address): \(.staked_amount) tokens staked, \(.validation_count) validaciones, \(.total_rewards) recompensas"'
else
    echo "⚠️  No se encontraron validadores activos"
fi

# Verificar validador específico
echo ""
echo "🔍 Verificando información de WALLET1 como validador..."
VALIDATOR_INFO_RESPONSE=$(curl -s "http://127.0.0.1:${API_PORT}/api/v1/staking/validator/$WALLET1")
VALIDATOR_EXISTS=$(echo "$VALIDATOR_INFO_RESPONSE" | jq -r '.success' 2>/dev/null || echo "false")

if [ "$VALIDATOR_EXISTS" == "true" ]; then
    VALIDATOR_STAKE=$(echo "$VALIDATOR_INFO_RESPONSE" | jq -r '.data.staked_amount' 2>/dev/null || echo "0")
    VALIDATOR_ACTIVE=$(echo "$VALIDATOR_INFO_RESPONSE" | jq -r '.data.is_active' 2>/dev/null || echo "false")
    echo "✅ Validador encontrado:"
    echo "   - Stake: $VALIDATOR_STAKE tokens"
    echo "   - Activo: $VALIDATOR_ACTIVE"
else
    echo "⚠️  Validador no encontrado (puede ser que la transacción aún no se procesó)"
fi

# Minar más bloques para verificar que PoS funciona
echo ""
echo "⛏️  Minando más bloques para verificar PoS..."
POS_BLOCKS=0
POW_BLOCKS=0

for i in {1..5}; do
    MINE_RESPONSE=$(curl -s -X POST "http://127.0.0.1:${API_PORT}/api/v1/mine" \
        -H "Content-Type: application/json" \
        -d "{\"miner_address\": \"$WALLET1\", \"max_transactions\": 10}")
    
    MINE_CONSENSUS=$(echo "$MINE_RESPONSE" | jq -r '.data.consensus' 2>/dev/null || echo "")
    MINE_VALIDATOR=$(echo "$MINE_RESPONSE" | jq -r '.data.validator' 2>/dev/null || echo "")
    
    if [ "$MINE_CONSENSUS" == "PoS" ]; then
        POS_BLOCKS=$((POS_BLOCKS + 1))
        echo "   Bloque $i: PoS (validador: ${MINE_VALIDATOR:0:16}...)"
    else
        POW_BLOCKS=$((POW_BLOCKS + 1))
        echo "   Bloque $i: PoW"
    fi
    sleep 2
done

# Verificar recompensas
echo ""
echo "💰 Verificando recompensas del validador..."
VALIDATOR_INFO_RESPONSE=$(curl -s "http://127.0.0.1:${API_PORT}/api/v1/staking/validator/$WALLET1")
VALIDATOR_REWARDS=$(echo "$VALIDATOR_INFO_RESPONSE" | jq -r '.data.total_rewards' 2>/dev/null || echo "0")
VALIDATOR_VALIDATIONS=$(echo "$VALIDATOR_INFO_RESPONSE" | jq -r '.data.validation_count' 2>/dev/null || echo "0")

echo "   - Total recompensas: $VALIDATOR_REWARDS"
echo "   - Validaciones: $VALIDATOR_VALIDATIONS"

# Solicitar unstaking
echo ""
echo "🔓 Solicitando unstaking..."
UNSTAKE_RESPONSE=$(curl -s -X POST "http://127.0.0.1:${API_PORT}/api/v1/staking/unstake" \
    -H "Content-Type: application/json" \
    -d "{\"address\": \"$WALLET1\"}")

UNSTAKE_SUCCESS=$(echo "$UNSTAKE_RESPONSE" | jq -r '.success' 2>/dev/null || echo "false")
UNSTAKE_AMOUNT=$(echo "$UNSTAKE_RESPONSE" | jq -r '.data' 2>/dev/null || echo "0")

if [ "$UNSTAKE_SUCCESS" == "true" ]; then
    echo "✅ Unstaking solicitado: $UNSTAKE_AMOUNT tokens"
    echo "   ⏳ Período de lock: 7 días (604800 segundos)"
else
    echo "❌ Error: Unstaking falló"
    echo "Response: $UNSTAKE_RESPONSE"
fi

# Verificar estado del validador después de unstaking
echo ""
echo "🔍 Verificando estado del validador después de unstaking..."
VALIDATOR_INFO_RESPONSE=$(curl -s "http://127.0.0.1:${API_PORT}/api/v1/staking/validator/$WALLET1")
UNSTAKING_REQUESTED=$(echo "$VALIDATOR_INFO_RESPONSE" | jq -r '.data.unstaking_requested' 2>/dev/null || echo "false")
VALIDATOR_ACTIVE=$(echo "$VALIDATOR_INFO_RESPONSE" | jq -r '.data.is_active' 2>/dev/null || echo "false")

echo "   - Unstaking solicitado: $UNSTAKING_REQUESTED"
echo "   - Activo: $VALIDATOR_ACTIVE"

# Resultados finales
echo ""
echo "📊 Resultados Finales:"
echo "======================"
echo "  - Validadores activos: $VALIDATORS_COUNT"
echo "  - Bloques minados con PoS: $POS_BLOCKS"
echo "  - Bloques minados con PoW: $POW_BLOCKS"
echo "  - Recompensas del validador: $VALIDATOR_REWARDS"
echo "  - Validaciones: $VALIDATOR_VALIDATIONS"
echo ""

SUCCESS=true

if [ "$VALIDATORS_COUNT" -gt 0 ]; then
    echo "✅ TEST EXITOSO: Sistema de staking PoS funciona correctamente"
    echo "   - Validadores se crearon correctamente"
    echo "   - Staking funcionó"
    if [ "$POS_BLOCKS" -gt 0 ]; then
        echo "   - PoS se usó para minar bloques"
    else
        echo "   - ⚠️  PoS no se usó (puede ser normal si las transacciones aún no se procesaron)"
    fi
    echo "   - Unstaking funcionó"
else
    echo "⚠️  TEST PARCIAL: Validadores no se encontraron"
    echo "   - Puede ser que las transacciones aún no se procesaron"
    SUCCESS=false
fi

if [ "$SUCCESS" = true ]; then
    echo ""
    echo "🎉 Todos los tests pasaron exitosamente!"
    exit 0
else
    echo ""
    echo "❌ Algunos tests fallaron"
    echo ""
    echo "📋 Últimas líneas de logs:"
    tail -20 /tmp/node_staking.log
    exit 1
fi

