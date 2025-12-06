#!/bin/bash

# Script de prueba funcional de endpoints API
# Prueba el flujo completo: wallet -> transacción -> minería -> verificación

echo "🧪 Testing Funcional de Endpoints API"
echo "======================================"
echo ""

# Colores
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

API_URL="http://127.0.0.1:8080"
PASSED=0
FAILED=0

# Función para hacer requests
make_request() {
    local method=$1
    local endpoint=$2
    local data=$3
    local description=$4
    
    echo -e "${BLUE}Testing: $description${NC}"
    
    if [ -z "$data" ]; then
        response=$(curl -s -w "\n%{http_code}" -X $method "$API_URL$endpoint" 2>/dev/null)
    else
        response=$(curl -s -w "\n%{http_code}" -X $method "$API_URL$endpoint" \
            -H "Content-Type: application/json" \
            -d "$data" 2>/dev/null)
    fi
    
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | sed '$d')
    
    if [ "$http_code" -ge 200 ] && [ "$http_code" -lt 300 ]; then
        echo -e "${GREEN}✅ Success (HTTP $http_code)${NC}"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
        ((PASSED++))
        echo ""
        return 0
    else
        echo -e "${RED}❌ Failed (HTTP $http_code)${NC}"
        echo "$body"
        ((FAILED++))
        echo ""
        return 1
    fi
}

# Verificar que el servidor esté corriendo
echo -e "${YELLOW}⚠️  IMPORTANTE: Asegúrate de que el servidor esté corriendo en $API_URL${NC}"
echo "   Ejecuta: cargo run"
echo ""
read -p "¿El servidor está corriendo? (y/n): " -n 1 -r
echo ""
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo -e "${RED}Por favor inicia el servidor primero${NC}"
    exit 1
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📋 Pruebas de Endpoints"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# 1. Verificar información de la blockchain
make_request "GET" "/api/v1/chain/info" "" "GET /api/v1/chain/info"

# 2. Crear wallet 1
echo "📝 Creando wallet 1..."
wallet1_response=$(curl -s -X POST "$API_URL/api/v1/wallets/create" 2>/dev/null)
wallet1_address=$(echo "$wallet1_response" | grep -o '"address":"[^"]*' | cut -d'"' -f4)
if [ -z "$wallet1_address" ]; then
    echo -e "${RED}❌ Error obteniendo dirección del wallet 1${NC}"
    exit 1
fi
echo -e "${GREEN}✅ Wallet 1 creado: $wallet1_address${NC}"
echo ""

# 3. Crear wallet 2
echo "📝 Creando wallet 2..."
wallet2_response=$(curl -s -X POST "$API_URL/api/v1/wallets/create" 2>/dev/null)
wallet2_address=$(echo "$wallet2_response" | grep -o '"address":"[^"]*' | cut -d'"' -f4)
if [ -z "$wallet2_address" ]; then
    echo -e "${RED}❌ Error obteniendo dirección del wallet 2${NC}"
    exit 1
fi
echo -e "${GREEN}✅ Wallet 2 creado: $wallet2_address${NC}"
echo ""

# 4. Verificar balances iniciales
make_request "GET" "/api/v1/wallets/$wallet1_address" "" "GET Balance wallet 1"
make_request "GET" "/api/v1/wallets/$wallet2_address" "" "GET Balance wallet 2"

# 5. Verificar mempool vacío
make_request "GET" "/api/v1/mempool" "" "GET Mempool (debe estar vacío)"

# 6. Minar bloque inicial para dar fondos al wallet 1
echo "⛏️  Minando bloque inicial para wallet 1..."
make_request "POST" "/api/v1/mine" "{\"miner_address\":\"$wallet1_address\",\"max_transactions\":10}" "Mine block (recompensa para wallet 1)"

# 7. Verificar balance después de minería
echo "💰 Verificando balance después de minería..."
make_request "GET" "/api/v1/wallets/$wallet1_address" "" "GET Balance wallet 1 (debe tener recompensa)"

# 8. Crear transacción (se agregará al mempool)
echo "📤 Creando transacción wallet1 -> wallet2..."
make_request "POST" "/api/v1/transactions" "{\"from\":\"$wallet1_address\",\"to\":\"$wallet2_address\",\"amount\":25}" "POST Transaction (wallet1 -> wallet2)"

# 9. Verificar mempool tiene la transacción
make_request "GET" "/api/v1/mempool" "" "GET Mempool (debe tener 1 transacción)"

# 10. Minar bloque con la transacción
echo "⛏️  Minando bloque con transacción del mempool..."
make_request "POST" "/api/v1/mine" "{\"miner_address\":\"$wallet1_address\",\"max_transactions\":10}" "Mine block (con transacción del mempool)"

# 11. Verificar mempool vacío después de minar
make_request "GET" "/api/v1/mempool" "" "GET Mempool (debe estar vacío después de minar)"

# 12. Verificar balances finales
echo "💰 Verificando balances finales..."
make_request "GET" "/api/v1/wallets/$wallet1_address" "" "GET Balance wallet 1 (final)"
make_request "GET" "/api/v1/wallets/$wallet2_address" "" "GET Balance wallet 2 (debe tener 25)"

# 13. Verificar cadena
make_request "GET" "/api/v1/chain/verify" "" "GET Verify chain"

# 14. Obtener información final
make_request "GET" "/api/v1/chain/info" "" "GET Chain info (final)"

# Resumen
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📊 Resumen de Pruebas"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo -e "${GREEN}✅ Pruebas pasadas: $PASSED${NC}"
echo -e "${RED}❌ Pruebas fallidas: $FAILED${NC}"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}🎉 ¡Todas las pruebas pasaron!${NC}"
    exit 0
else
    echo -e "${YELLOW}⚠️  Algunas pruebas fallaron${NC}"
    exit 1
fi

