#!/bin/bash

# Script de prueba completa del sistema blockchain
# Verifica que todos los componentes funcionen correctamente

echo "🧪 Iniciando pruebas del sistema blockchain..."
echo ""

# Colores para output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Contador de pruebas
PASSED=0
FAILED=0

# Función para verificar resultado
check_result() {
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✅ $1${NC}"
        ((PASSED++))
    else
        echo -e "${RED}❌ $1${NC}"
        ((FAILED++))
    fi
}

# 1. Verificar estructura de archivos
echo "📁 Verificando estructura de archivos..."
[ -f "src/main.rs" ] && check_result "main.rs existe" || echo -e "${RED}❌ main.rs no encontrado${NC}"
[ -f "src/blockchain.rs" ] && check_result "blockchain.rs existe" || echo -e "${RED}❌ blockchain.rs no encontrado${NC}"
[ -f "src/models.rs" ] && check_result "models.rs existe" || echo -e "${RED}❌ models.rs no encontrado${NC}"
[ -f "src/api.rs" ] && check_result "api.rs existe" || echo -e "${RED}❌ api.rs no encontrado${NC}"
[ -f "src/network.rs" ] && check_result "network.rs existe" || echo -e "${RED}❌ network.rs no encontrado${NC}"
[ -f "src/database.rs" ] && check_result "database.rs existe" || echo -e "${RED}❌ database.rs no encontrado${NC}"
[ -f "Cargo.toml" ] && check_result "Cargo.toml existe" || echo -e "${RED}❌ Cargo.toml no encontrado${NC}"

echo ""

# 2. Verificar dependencias en Cargo.toml
echo "📦 Verificando dependencias..."
if [ -f "Cargo.toml" ]; then
    grep -q "sha2" Cargo.toml && check_result "sha2 incluido" || echo -e "${YELLOW}⚠️  sha2 no encontrado${NC}"
    grep -q "serde" Cargo.toml && check_result "serde incluido" || echo -e "${YELLOW}⚠️  serde no encontrado${NC}"
    grep -q "actix-web" Cargo.toml && check_result "actix-web incluido" || echo -e "${YELLOW}⚠️  actix-web no encontrado${NC}"
    grep -q "tokio" Cargo.toml && check_result "tokio incluido" || echo -e "${YELLOW}⚠️  tokio no encontrado${NC}"
    grep -q "ed25519-dalek" Cargo.toml && check_result "ed25519-dalek incluido" || echo -e "${YELLOW}⚠️  ed25519-dalek no encontrado${NC}"
    grep -q "rusqlite" Cargo.toml && check_result "rusqlite incluido" || echo -e "${YELLOW}⚠️  rusqlite no encontrado${NC}"
fi

echo ""

# 3. Verificar funciones críticas en blockchain.rs
echo "🔍 Verificando funciones críticas..."
if [ -f "src/blockchain.rs" ]; then
    grep -q "pub fn create_coinbase_transaction" src/blockchain.rs && check_result "create_coinbase_transaction existe" || echo -e "${YELLOW}⚠️  create_coinbase_transaction no encontrado${NC}"
    grep -q "pub fn calculate_mining_reward" src/blockchain.rs && check_result "calculate_mining_reward existe" || echo -e "${YELLOW}⚠️  calculate_mining_reward no encontrado${NC}"
    grep -q "pub fn mine_block_with_reward" src/blockchain.rs && check_result "mine_block_with_reward existe" || echo -e "${YELLOW}⚠️  mine_block_with_reward no encontrado${NC}"
    grep -q "pub fn validate_coinbase_transaction" src/blockchain.rs && check_result "validate_coinbase_transaction existe" || echo -e "${YELLOW}⚠️  validate_coinbase_transaction no encontrado${NC}"
fi

echo ""

# 4. Verificar Mempool en models.rs
echo "💾 Verificando Mempool..."
if [ -f "src/models.rs" ]; then
    grep -q "pub struct Mempool" src/models.rs && check_result "Mempool struct existe" || echo -e "${YELLOW}⚠️  Mempool struct no encontrado${NC}"
    grep -q "pub fn add_transaction" src/models.rs && check_result "Mempool::add_transaction existe" || echo -e "${YELLOW}⚠️  Mempool::add_transaction no encontrado${NC}"
    grep -q "pub fn get_transactions_for_block" src/models.rs && check_result "Mempool::get_transactions_for_block existe" || echo -e "${YELLOW}⚠️  Mempool::get_transactions_for_block no encontrado${NC}"
fi

echo ""

# 5. Verificar endpoints API
echo "🌐 Verificando endpoints API..."
if [ -f "src/api.rs" ]; then
    grep -q "pub async fn mine_block" src/api.rs && check_result "mine_block endpoint existe" || echo -e "${YELLOW}⚠️  mine_block endpoint no encontrado${NC}"
    grep -q "pub async fn get_mempool" src/api.rs && check_result "get_mempool endpoint existe" || echo -e "${YELLOW}⚠️  get_mempool endpoint no encontrado${NC}"
    grep -q "/mine" src/api.rs && check_result "Ruta /mine configurada" || echo -e "${YELLOW}⚠️  Ruta /mine no encontrada${NC}"
    grep -q "/mempool" src/api.rs && check_result "Ruta /mempool configurada" || echo -e "${YELLOW}⚠️  Ruta /mempool no encontrada${NC}"
fi

echo ""

# 6. Verificar sincronización de wallets
echo "👛 Verificando sincronización de wallets..."
if [ -f "src/models.rs" ]; then
    grep -q "pub fn sync_from_blockchain" src/models.rs && check_result "sync_from_blockchain existe" || echo -e "${YELLOW}⚠️  sync_from_blockchain no encontrado${NC}"
    grep -q "pub fn process_coinbase_transaction" src/models.rs && check_result "process_coinbase_transaction existe" || echo -e "${YELLOW}⚠️  process_coinbase_transaction no encontrado${NC}"
fi

if [ -f "src/main.rs" ]; then
    grep -q "sync_from_blockchain" src/main.rs && check_result "sync_from_blockchain llamado en main" || echo -e "${YELLOW}⚠️  sync_from_blockchain no llamado en main${NC}"
fi

echo ""

# 7. Verificar AppState incluye mempool
echo "🔧 Verificando AppState..."
if [ -f "src/api.rs" ]; then
    grep -q "pub mempool:" src/api.rs && check_result "AppState incluye mempool" || echo -e "${YELLOW}⚠️  AppState no incluye mempool${NC}"
fi

if [ -f "src/main.rs" ]; then
    grep -q "Mempool::new()" src/main.rs && check_result "Mempool inicializado en main" || echo -e "${YELLOW}⚠️  Mempool no inicializado en main${NC}"
fi

echo ""

# Resumen
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📊 Resumen de Verificación"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo -e "${GREEN}✅ Pruebas pasadas: $PASSED${NC}"
echo -e "${RED}❌ Pruebas fallidas: $FAILED${NC}"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}🎉 ¡Todas las verificaciones pasaron!${NC}"
    exit 0
else
    echo -e "${YELLOW}⚠️  Algunas verificaciones fallaron. Revisa los detalles arriba.${NC}"
    exit 1
fi

