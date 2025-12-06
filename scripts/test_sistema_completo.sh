#!/bin/bash

# 🧪 Script de Prueba Completa del Sistema
# Este script prueba todas las funcionalidades principales de la blockchain

set -e

echo "🚀 Iniciando Prueba Completa del Sistema"
echo "=========================================="
echo ""

# Colores para output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuración
API_URL="http://127.0.0.1:8080/api/v1"
TIMEOUT=5

# Función para verificar si el servidor está corriendo
check_server() {
    echo -n "Verificando servidor... "
    if curl -s --connect-timeout $TIMEOUT "$API_URL/chain/info" > /dev/null 2>&1; then
        echo -e "${GREEN}✓ Servidor activo${NC}"
        return 0
    else
        echo -e "${RED}✗ Servidor no responde${NC}"
        echo ""
        echo "Por favor, inicia el servidor primero:"
        echo "  cargo run 8080 8081 blockchain"
        return 1
    fi
}

# Función para probar endpoint
test_endpoint() {
    local method=$1
    local endpoint=$2
    local data=$3
    local description=$4
    
    echo -n "Probando $description... "
    
    if [ "$method" = "GET" ]; then
        response=$(curl -s -w "\n%{http_code}" "$API_URL$endpoint" 2>&1)
    else
        response=$(curl -s -w "\n%{http_code}" -X "$method" \
            -H "Content-Type: application/json" \
            -d "$data" \
            "$API_URL$endpoint" 2>&1)
    fi
    
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | sed '$d')
    
    if [ "$http_code" = "200" ] || [ "$http_code" = "201" ]; then
        echo -e "${GREEN}✓ OK (HTTP $http_code)${NC}"
        return 0
    else
        echo -e "${RED}✗ Error (HTTP $http_code)${NC}"
        echo "  Respuesta: $body"
        return 1
    fi
}

# Verificar servidor
if ! check_server; then
    exit 1
fi

echo ""
echo "📋 Ejecutando Pruebas"
echo "====================="
echo ""

# Contador de pruebas
PASSED=0
FAILED=0

# 1. Crear wallet
echo "1. Creando wallet..."
if test_endpoint "POST" "/wallets/create" "" "Crear wallet"; then
    WALLET1=$(curl -s -X POST "$API_URL/wallets/create" | grep -o '"address":"[^"]*' | cut -d'"' -f4)
    echo "   Wallet creado: $WALLET1"
    ((PASSED++))
else
    ((FAILED++))
    exit 1
fi

echo ""

# 2. Obtener información de blockchain
echo "2. Obteniendo información de blockchain..."
if test_endpoint "GET" "/chain/info" "" "Información de blockchain"; then
    ((PASSED++))
else
    ((FAILED++))
fi

echo ""

# 3. Verificar cadena
echo "3. Verificando cadena..."
if test_endpoint "GET" "/chain/verify" "" "Verificar cadena"; then
    ((PASSED++))
else
    ((FAILED++))
fi

echo ""

# 4. Obtener estadísticas
echo "4. Obteniendo estadísticas..."
if test_endpoint "GET" "/stats" "" "Estadísticas del sistema"; then
    ((PASSED++))
else
    ((FAILED++))
fi

echo ""

# 5. Ver mempool
echo "5. Consultando mempool..."
if test_endpoint "GET" "/mempool" "" "Mempool"; then
    ((PASSED++))
else
    ((FAILED++))
fi

echo ""

# 6. Minar bloque
echo "6. Minando bloque con recompensa..."
if test_endpoint "POST" "/mine" "{\"miner_address\":\"$WALLET1\",\"max_transactions\":10}" "Minar bloque"; then
    ((PASSED++))
    echo "   Bloque minado exitosamente"
else
    ((FAILED++))
fi

echo ""

# 7. Verificar balance después de minar
echo "7. Verificando balance del wallet..."
if test_endpoint "GET" "/wallets/$WALLET1" "" "Balance del wallet"; then
    BALANCE=$(curl -s "$API_URL/wallets/$WALLET1" | grep -o '"balance":[0-9]*' | cut -d':' -f2)
    echo "   Balance: $BALANCE"
    ((PASSED++))
else
    ((FAILED++))
fi

echo ""

# 8. Crear segundo wallet
echo "8. Creando segundo wallet..."
if test_endpoint "POST" "/wallets/create" "" "Crear segundo wallet"; then
    WALLET2=$(curl -s -X POST "$API_URL/wallets/create" | grep -o '"address":"[^"]*' | cut -d'"' -f4)
    echo "   Wallet creado: $WALLET2"
    ((PASSED++))
else
    ((FAILED++))
fi

echo ""

# 9. Crear transacción
echo "9. Creando transacción..."
if [ -n "$WALLET1" ] && [ -n "$WALLET2" ]; then
    TX_DATA="{\"from\":\"$WALLET1\",\"to\":\"$WALLET2\",\"amount\":25,\"fee\":1}"
    if test_endpoint "POST" "/transactions" "$TX_DATA" "Crear transacción"; then
        ((PASSED++))
        echo "   Transacción creada y agregada al mempool"
    else
        ((FAILED++))
    fi
else
    echo -e "${YELLOW}⚠ Saltando: Wallets no disponibles${NC}"
fi

echo ""

# 10. Minar bloque con transacción
echo "10. Minando bloque con transacción..."
if test_endpoint "POST" "/mine" "{\"miner_address\":\"$WALLET1\",\"max_transactions\":10}" "Minar bloque con transacción"; then
    ((PASSED++))
    echo "   Bloque minado con transacción"
else
    ((FAILED++))
fi

echo ""

# 11. Verificar balances finales
echo "11. Verificando balances finales..."
if [ -n "$WALLET1" ] && [ -n "$WALLET2" ]; then
    echo -n "   Wallet 1: "
    BALANCE1=$(curl -s "$API_URL/wallets/$WALLET1" | grep -o '"balance":[0-9]*' | cut -d':' -f2)
    echo "$BALANCE1"
    
    echo -n "   Wallet 2: "
    BALANCE2=$(curl -s "$API_URL/wallets/$WALLET2" | grep -o '"balance":[0-9]*' | cut -d':' -f2)
    echo "$BALANCE2"
    ((PASSED++))
else
    echo -e "${YELLOW}⚠ Saltando: Wallets no disponibles${NC}"
fi

echo ""

# 12. Obtener todos los bloques
echo "12. Obteniendo todos los bloques..."
if test_endpoint "GET" "/blocks" "" "Listar bloques"; then
    BLOCK_COUNT=$(curl -s "$API_URL/blocks" | grep -o '"index":[0-9]*' | wc -l | tr -d ' ')
    echo "   Total de bloques: $BLOCK_COUNT"
    ((PASSED++))
else
    ((FAILED++))
fi

echo ""
echo "=========================================="
echo "📊 Resumen de Pruebas"
echo "=========================================="
echo -e "${GREEN}Pruebas exitosas: $PASSED${NC}"
if [ $FAILED -gt 0 ]; then
    echo -e "${RED}Pruebas fallidas: $FAILED${NC}"
else
    echo -e "${GREEN}Pruebas fallidas: $FAILED${NC}"
fi
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✅ Todas las pruebas pasaron exitosamente${NC}"
    exit 0
else
    echo -e "${RED}❌ Algunas pruebas fallaron${NC}"
    exit 1
fi

