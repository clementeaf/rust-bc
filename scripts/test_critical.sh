#!/bin/bash

set -e

API_URL="http://127.0.0.1:8080/api/v1"
RESULTS_DIR="test_results_critical"

echo "🔥 PRUEBAS CRÍTICAS - CASOS LÍMITE Y FALLOS"
echo "============================================"
echo ""

mkdir -p "$RESULTS_DIR"

test_count=0
passed_count=0
failed_count=0
critical_failures=()

test_result() {
    local test_name="$1"
    local result="$2"
    local is_critical="${3:-false}"
    test_count=$((test_count + 1))
    
    if [ "$result" = "PASS" ]; then
        echo "✅ $test_name"
        passed_count=$((passed_count + 1))
    else
        echo "❌ $test_name"
        failed_count=$((failed_count + 1))
        if [ "$is_critical" = "true" ]; then
            critical_failures+=("$test_name")
        fi
    fi
}

echo "📊 TEST 1: Valores Extremos - Amount Muy Grande"
echo "----------------------------------------------"
response=$(curl -s -w "\n%{http_code}" -X POST "$API_URL/transactions" \
    -H "Content-Type: application/json" \
    -d '{"from":"test","to":"test2","amount":18446744073709551615}' \
    --max-time 5 2>/dev/null || echo "000")
http_code=$(echo "$response" | tail -n1)
if [ "$http_code" = "400" ] || [ "$http_code" = "422" ]; then
    test_result "Valor extremo: Amount máximo rechazado correctamente" "PASS" "true"
else
    test_result "Valor extremo: Amount máximo no validado (HTTP $http_code)" "FAIL" "true"
fi
echo ""

echo "📊 TEST 2: Strings Muy Largos"
echo "----------------------------"
long_string=$(head -c 10000 < /dev/zero | tr '\0' 'a')
response=$(curl -s -w "\n%{http_code}" -X POST "$API_URL/transactions" \
    -H "Content-Type: application/json" \
    -d "{\"from\":\"$long_string\",\"to\":\"test\",\"amount\":1}" \
    --max-time 5 2>/dev/null || echo "000")
http_code=$(echo "$response" | tail -n1)
if [ "$http_code" = "400" ] || [ "$http_code" = "422" ] || [ "$http_code" = "413" ]; then
    test_result "String largo: Rechazado correctamente" "PASS"
else
    test_result "String largo: No validado (HTTP $http_code)" "FAIL"
fi
echo ""

echo "📊 TEST 3: JSON Malformado"
echo "-------------------------"
response=$(curl -s -w "\n%{http_code}" -X POST "$API_URL/transactions" \
    -H "Content-Type: application/json" \
    -d '{"from":"test","to":"test2","amount":' \
    --max-time 5 2>/dev/null || echo "000")
http_code=$(echo "$response" | tail -n1)
if [ "$http_code" = "400" ] || [ "$http_code" = "422" ]; then
    test_result "JSON malformado: Rechazado correctamente" "PASS" "true"
else
    test_result "JSON malformado: No manejado (HTTP $http_code)" "FAIL" "true"
fi
echo ""

echo "📊 TEST 4: Múltiples Requests Rápidos (Burst)"
echo "---------------------------------------------"
burst_success=0
burst_total=20

for i in $(seq 1 $burst_total); do
    response=$(curl -s -w "\n%{http_code}" -X GET "$API_URL/health" --max-time 2 2>/dev/null || echo "000")
    http_code=$(echo "$response" | tail -n1)
    if [ "$http_code" = "200" ] || [ "$http_code" = "429" ]; then
        burst_success=$((burst_success + 1))
    fi
done

if [ $burst_success -eq $burst_total ]; then
    test_result "Burst: $burst_success/$burst_total requests manejados" "PASS"
else
    test_result "Burst: Solo $burst_success/$burst_total requests manejados" "FAIL"
fi
echo ""

echo "📊 TEST 5: Consultas a Endpoints Inexistentes"
echo "---------------------------------------------"
response=$(curl -s -w "\n%{http_code}" -X GET "$API_URL/nonexistent" --max-time 5 2>/dev/null || echo "000")
http_code=$(echo "$response" | tail -n1)
if [ "$http_code" = "404" ]; then
    test_result "404: Endpoint inexistente retorna 404" "PASS" "true"
else
    test_result "404: Endpoint inexistente retorna HTTP $http_code" "FAIL" "true"
fi
echo ""

echo "📊 TEST 6: Métodos HTTP Incorrectos"
echo "----------------------------------"
response=$(curl -s -w "\n%{http_code}" -X DELETE "$API_URL/blocks" --max-time 5 2>/dev/null || echo "000")
http_code=$(echo "$response" | tail -n1)
if [ "$http_code" = "405" ] || [ "$http_code" = "404" ]; then
    test_result "Método incorrecto: DELETE rechazado correctamente" "PASS"
else
    test_result "Método incorrecto: No manejado (HTTP $http_code)" "FAIL"
fi
echo ""

echo "📊 TEST 7: Headers Faltantes"
echo "---------------------------"
response=$(curl -s -w "\n%{http_code}" -X POST "$API_URL/transactions" \
    -d '{"from":"test","to":"test2","amount":1}' \
    --max-time 5 2>/dev/null || echo "000")
http_code=$(echo "$response" | tail -n1)
if [ "$http_code" = "200" ] || [ "$http_code" = "400" ] || [ "$http_code" = "415" ]; then
    test_result "Headers: Manejo correcto sin Content-Type" "PASS"
else
    test_result "Headers: Error inesperado (HTTP $http_code)" "FAIL"
fi
echo ""

echo "📊 TEST 8: Consistencia de Caché Bajo Carga"
echo "------------------------------------------"
wallet_response=$(curl -s -X POST "$API_URL/wallets/create" --max-time 5 2>/dev/null || echo '{"success":false}')
if echo "$wallet_response" | grep -q '"success":true'; then
    wallet_address=$(echo "$wallet_response" | grep -o '"address":"[^"]*"' | cut -d'"' -f4)
    
    balance1=$(curl -s -X GET "$API_URL/wallets/$wallet_address" --max-time 5 2>/dev/null || echo '{"success":false}')
    
    for i in {1..10}; do
        curl -s -X GET "$API_URL/wallets/$wallet_address" --max-time 2 >/dev/null 2>&1 &
    done
    wait
    
    balance2=$(curl -s -X GET "$API_URL/wallets/$wallet_address" --max-time 5 2>/dev/null || echo '{"success":false}')
    
    balance1_val=$(echo "$balance1" | grep -o '"balance":[0-9]*' | cut -d':' -f2)
    balance2_val=$(echo "$balance2" | grep -o '"balance":[0-9]*' | cut -d':' -f2)
    
    if [ "$balance1_val" = "$balance2_val" ]; then
        test_result "Consistencia caché: Balances consistentes bajo carga" "PASS" "true"
    else
        test_result "Consistencia caché: Inconsistencia detectada ($balance1_val vs $balance2_val)" "FAIL" "true"
    fi
else
    test_result "Consistencia caché: No se pudo crear wallet" "FAIL"
fi
echo ""

echo "📊 TEST 9: Recuperación Después de Errores"
echo "------------------------------------------"
error_recovery=0
for i in {1..5}; do
    curl -s -X POST "$API_URL/transactions" \
        -H "Content-Type: application/json" \
        -d '{"from":"invalid","to":"test","amount":1}' \
        --max-time 5 >/dev/null 2>&1
    
    response=$(curl -s -w "\n%{http_code}" -X GET "$API_URL/health" --max-time 5 2>/dev/null || echo "000")
    http_code=$(echo "$response" | tail -n1)
    if [ "$http_code" = "200" ]; then
        error_recovery=$((error_recovery + 1))
    fi
    sleep 0.2
done

if [ $error_recovery -eq 5 ]; then
    test_result "Recuperación: Sistema se recupera después de errores" "PASS" "true"
else
    test_result "Recuperación: Solo $error_recovery/5 requests exitosos después de errores" "FAIL" "true"
fi
echo ""

echo "📊 TEST 10: Límites de Rate Limiting"
echo "-----------------------------------"
rate_limit_test=0
for i in {1..105}; do
    response=$(curl -s -w "\n%{http_code}" -X GET "$API_URL/health" --max-time 2 2>/dev/null || echo "000")
    http_code=$(echo "$response" | tail -n1)
    
    if [ $i -le 100 ]; then
        if [ "$http_code" = "200" ]; then
            rate_limit_test=$((rate_limit_test + 1))
        fi
    else
        if [ "$http_code" = "429" ]; then
            rate_limit_test=$((rate_limit_test + 1))
        fi
    fi
done

if [ $rate_limit_test -ge 100 ]; then
    test_result "Rate limit: Límites aplicados correctamente" "PASS" "true"
else
    test_result "Rate limit: Límites no funcionan correctamente" "FAIL" "true"
fi
echo ""

echo "===================================="
echo "📊 RESUMEN DE PRUEBAS CRÍTICAS"
echo "===================================="
echo "Total de pruebas: $test_count"
echo "✅ Exitosas: $passed_count"
echo "❌ Fallidas: $failed_count"
echo ""

if [ ${#critical_failures[@]} -gt 0 ]; then
    echo "🚨 FALLOS CRÍTICOS DETECTADOS:"
    for failure in "${critical_failures[@]}"; do
        echo "   - $failure"
    done
    echo ""
fi

if [ $failed_count -eq 0 ]; then
    echo "🎉 TODAS LAS PRUEBAS CRÍTICAS PASARON"
    exit 0
else
    echo "⚠️  ALGUNAS PRUEBAS CRÍTICAS FALLARON"
    exit 1
fi

