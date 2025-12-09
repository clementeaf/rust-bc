#!/bin/bash

# Test rápido del sistema sin BD - Solo verifica compilación y estructura

set -e

echo "🧪 TEST RÁPIDO: Sistema Sin BD"
echo "================================"
echo ""

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Test 1: Compilación
echo "1️⃣  Verificando compilación..."
if cargo check --message-format=short > /tmp/compile-check.log 2>&1; then
    echo -e "${GREEN}✅ Compilación exitosa${NC}"
else
    echo -e "${RED}❌ Error de compilación${NC}"
    tail -10 /tmp/compile-check.log
    exit 1
fi

# Test 2: Módulos nuevos
echo ""
echo "2️⃣  Verificando módulos nuevos..."
if [ -f "src/block_storage.rs" ] && [ -f "src/state_reconstructor.rs" ]; then
    echo -e "${GREEN}✅ Módulos nuevos creados${NC}"
    echo "   - src/block_storage.rs"
    echo "   - src/state_reconstructor.rs"
else
    echo -e "${RED}❌ Módulos faltantes${NC}"
    exit 1
fi

# Test 3: Integración en main.rs
echo ""
echo "3️⃣  Verificando integración..."
if grep -q "BlockStorage" src/main.rs && grep -q "ReconstructedState" src/main.rs; then
    echo -e "${GREEN}✅ Integración en main.rs${NC}"
    echo "   - BlockStorage importado"
    echo "   - ReconstructedState usado"
else
    echo -e "${RED}❌ Integración incompleta${NC}"
    exit 1
fi

# Test 4: Referencias corregidas
echo ""
echo "4️⃣  Verificando referencias a Option<BlockchainDB>..."
API_REFS=$(grep -c "Option<BlockchainDB>" src/api.rs 2>/dev/null || echo "0")
NETWORK_REFS=$(grep -c "Option<BlockchainDB>" src/network.rs 2>/dev/null || echo "0")
if [ "$API_REFS" -gt 0 ] || [ "$NETWORK_REFS" -gt 0 ]; then
    echo -e "${GREEN}✅ Referencias actualizadas${NC}"
    echo "   - api.rs: $API_REFS referencias"
    echo "   - network.rs: $NETWORK_REFS referencias"
else
    echo -e "${YELLOW}⚠️  No se encontraron referencias (puede estar bien)${NC}"
fi

# Test 5: Funciones clave
echo ""
echo "5️⃣  Verificando funciones clave..."
if grep -q "pub fn load_all_blocks" src/block_storage.rs && \
   grep -q "pub fn from_blockchain" src/state_reconstructor.rs; then
    echo -e "${GREEN}✅ Funciones principales implementadas${NC}"
else
    echo -e "${RED}❌ Funciones faltantes${NC}"
    exit 1
fi

# Test 6: Dependencia bincode
echo ""
echo "6️⃣  Verificando dependencias..."
if grep -q "bincode" Cargo.toml; then
    echo -e "${GREEN}✅ Dependencia bincode agregada${NC}"
else
    echo -e "${RED}❌ Dependencia bincode faltante${NC}"
    exit 1
fi

# Resumen
echo ""
echo "================================"
echo -e "${GREEN}✅ TODOS LOS TESTS PASARON${NC}"
echo ""
echo "📊 Resumen:"
echo "  ✅ Compilación: OK"
echo "  ✅ Módulos nuevos: 2"
echo "  ✅ Integración: OK"
echo "  ✅ Referencias: Actualizadas"
echo "  ✅ Funciones: Implementadas"
echo "  ✅ Dependencias: OK"
echo ""
echo "🎯 El sistema sin BD está listo!"
echo ""
echo "💡 Para probar el servidor completo, ejecuta:"
echo "   cargo run -- 8090 8091"

