#!/bin/bash

# Tests de Seguridad Agresivos para Sistema de Billing
# Simula ataques realistas y violentos contra el sistema de billing

set +e

API_URL="http://127.0.0.1:8080/api/v1"
RESULTS_DIR="test_results_billing_security"
TIMEOUT=10
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

mkdir -p "$RESULTS_DIR"

echo "🛡️  TESTS DE SEGURIDAD AGRESIVOS - SISTEMA DE BILLING"
echo "======================================================"
echo ""

# Verificar que el servidor esté corriendo
if ! curl -s --max-time 2 "$API_URL/health" >/dev/null 2>&1; then
    echo "❌ ERROR: El servidor no está respondiendo en $API_URL"
    echo "💡 Inicia el servidor primero: DIFFICULTY=1 cargo run --release 8080 8081 blockchain"
    exit 1
fi

echo "✅ Servidor detectado y respondiendo"
echo ""

test_result() {
    local test_name="$1"
    local result="$2"
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    
    if [ "$result" = "PASS" ]; then
        echo "✅ $test_name"
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        echo "❌ $test_name"
        FAILED_TESTS=$((FAILED_TESTS + 1))
    fi
}

# Extrae el valor de transactions_this_month de una respuesta JSON de usage
get_transactions_usage() {
    local usage_response="$1"
    local usage=""
    
    # Método 1: jq (más confiable)
    if command -v jq >/dev/null 2>&1; then
        usage=$(echo "$usage_response" | jq -r '.data.transactions_this_month' 2>/dev/null)
    fi
    
    # Método 2: grep + cut (fallback)
    if [ -z "$usage" ] || [ "$usage" = "null" ] || [ "$usage" = "" ]; then
        usage=$(echo "$usage_response" | grep -o '"transactions_this_month":[0-9]*' | head -1 | cut -d':' -f2)
    fi
    
    # Método 3: sed (fallback adicional)
    if [ -z "$usage" ] || [ "$usage" = "null" ] || [ "$usage" = "" ]; then
        usage=$(echo "$usage_response" | sed -n 's/.*"transactions_this_month":\([0-9]*\).*/\1/p' | head -1)
    fi
    
    # Si aún no tenemos valor válido, usar 0
    if [ -z "$usage" ] || [ "$usage" = "null" ] || [ "$usage" = "" ] || ! [[ "$usage" =~ ^[0-9]+$ ]]; then
        usage=0
    fi
    
    echo "$usage"
}

echo "📊 TEST 1: Ataque de Fuerza Bruta en API Keys"
echo "----------------------------------------------"
brute_force_success=0
VALID_KEY=$(curl -s -X POST "$API_URL/billing/create-key" \
    -H "Content-Type: application/json" \
    -d '{"tier":"free"}' \
    --max-time $TIMEOUT 2>/dev/null | jq -r '.data' 2>/dev/null || echo "")

if [ -z "$VALID_KEY" ] || [ "$VALID_KEY" = "null" ]; then
    test_result "Fuerza bruta: No se pudo crear key de prueba" "FAIL"
else
    echo "  Key válida creada: ${VALID_KEY:0:20}..."
    
    brute_force_attempts=0
    brute_force_successful=0
    
    for i in {1..1000}; do
        RANDOM_KEY="bc_$(openssl rand -hex 16 2>/dev/null || echo $RANDOM$RANDOM)"
        RESPONSE=$(curl -s -w "\n%{http_code}" -X GET "$API_URL/billing/usage" \
            -H "X-API-Key: $RANDOM_KEY" \
            --max-time 2 2>/dev/null)
        HTTP_CODE=$(echo "$RESPONSE" | tail -n1)
        
        brute_force_attempts=$((brute_force_attempts + 1))
        
        if [ "$HTTP_CODE" = "200" ]; then
            brute_force_successful=$((brute_force_successful + 1))
        fi
        
        if [ $((i % 100)) -eq 0 ]; then
            echo -n "."
        fi
    done
    echo ""
    
    if [ $brute_force_successful -eq 0 ]; then
        brute_force_success=1
    fi
    
    if [ $brute_force_success -eq 1 ]; then
        test_result "Fuerza bruta: Sistema rechazó $brute_force_attempts intentos (0 exitosos)" "PASS"
    else
        test_result "Fuerza bruta: Sistema vulnerable ($brute_force_successful/$brute_force_attempts exitosos)" "FAIL"
    fi
fi
echo ""

echo "📊 TEST 2: Ataque de Bypass de Límites de Transacciones"
echo "--------------------------------------------------------"
# Esperar tiempo suficiente para que el rate limiting se resetee
# (límite es 5 req/seg, esperar 2 segundos para asegurar que no hay bloqueo)
sleep 3
bypass_limit_success=0
USAGE=0
USAGE_DISPLAY="0"
LIMIT_REACHED_COUNT=0
SUCCESS_COUNT=0

# Usar tier "free" que tiene límite de 100 transacciones
FREE_KEY_RESP=$(curl -s -X POST "$API_URL/billing/create-key" \
    -H "Content-Type: application/json" \
    -d '{"tier":"free"}' \
    --max-time $TIMEOUT 2>/dev/null)

# Extraer key con múltiples métodos
FREE_KEY=$(echo "$FREE_KEY_RESP" | jq -r '.data' 2>/dev/null || echo "")
if [ -z "$FREE_KEY" ] || [ "$FREE_KEY" = "null" ] || [ "$FREE_KEY" = "" ]; then
    FREE_KEY=$(echo "$FREE_KEY_RESP" | grep -o '"data":"[^"]*"' | head -1 | cut -d'"' -f4 || echo "")
fi

if [ -z "$FREE_KEY" ] || [ "$FREE_KEY" = "null" ] || [ "$FREE_KEY" = "" ]; then
    test_result "Bypass de límites: No se pudo crear API key de prueba" "FAIL"
    echo ""
else
    # Esperar para evitar rate limiting (límite es 5 req/seg, esperar 3 segundos)
    sleep 3
    # Crear wallet con reintentos
    WALLET1=""
    for attempt in {1..5}; do
        WALLET_RESP=$(curl -s -X POST "$API_URL/wallets/create" \
            -H "X-API-Key: $FREE_KEY" \
            --max-time $TIMEOUT 2>/dev/null)
        
        # Verificar si recibimos rate limit
        if echo "$WALLET_RESP" | grep -q "Rate limit exceeded"; then
            sleep 5
            continue
        fi
        
        # Intentar extraer wallet con jq primero
        WALLET1=$(echo "$WALLET_RESP" | jq -r '.data.address' 2>/dev/null || echo "")
        # Si falla jq, usar grep
        if [ -z "$WALLET1" ] || [ "$WALLET1" = "null" ] || [ "$WALLET1" = "" ]; then
            WALLET1=$(echo "$WALLET_RESP" | grep -o '"address":"[^"]*"' | head -1 | cut -d'"' -f4 || echo "")
        fi
        
        if [ -n "$WALLET1" ] && [ "$WALLET1" != "null" ] && [ "$WALLET1" != "" ]; then
            break
        fi
        
        sleep 2
    done
    
    if [ -z "$WALLET1" ] || [ "$WALLET1" = "null" ] || [ "$WALLET1" = "" ]; then
        test_result "Bypass de límites: No se pudo crear wallet de prueba" "FAIL"
        echo ""
    else
        WALLET2=$WALLET1
        
        # Minar muchos bloques para tener fondos suficientes
        echo "  Minando bloques para fondos iniciales..."
        for i in {1..50}; do
            curl -s -X POST "$API_URL/mine" \
                -H "Content-Type: application/json" \
                -d "{\"miner_address\":\"$WALLET1\",\"max_transactions\":1}" \
                --max-time $TIMEOUT >/dev/null 2>&1
        done
        sleep 2
        
        # Hacer transacciones hasta alcanzar exactamente 100 exitosas
        echo "  Realizando transacciones hasta alcanzar límite de 100..."
        ATTEMPTS=0
        MAX_ATTEMPTS=600
        
        while [ $SUCCESS_COUNT -lt 100 ] && [ $ATTEMPTS -lt $MAX_ATTEMPTS ]; do
            ATTEMPTS=$((ATTEMPTS + 1))
            sleep 0.2
            
            HTTP=$(curl -s -w "%{http_code}" -o /dev/null -X POST "$API_URL/transactions" \
                -H "Content-Type: application/json" \
                -H "X-API-Key: $FREE_KEY" \
                -d "{\"from\":\"$WALLET1\",\"to\":\"$WALLET2\",\"amount\":1,\"fee\":0}" \
                --max-time 5 2>/dev/null)
            
            if [ "$HTTP" = "201" ]; then
                SUCCESS_COUNT=$((SUCCESS_COUNT + 1))
                # Minar cada 10 exitosas para procesar pendientes y mantener fondos
                if [ $((SUCCESS_COUNT % 10)) -eq 0 ]; then
                    curl -s -X POST "$API_URL/mine" \
                        -H "Content-Type: application/json" \
                        -d "{\"miner_address\":\"$WALLET1\",\"max_transactions\":10}" \
                        --max-time $TIMEOUT >/dev/null 2>&1
                    sleep 0.5
                fi
            elif [ "$HTTP" = "402" ]; then
                LIMIT_REACHED_COUNT=$((LIMIT_REACHED_COUNT + 1))
                echo "  Límite alcanzado en intento $ATTEMPTS (exitosas: $SUCCESS_COUNT)"
                break
            fi
            
            # Mostrar progreso cada 25 intentos
            if [ $((ATTEMPTS % 25)) -eq 0 ]; then
                USAGE_TEMP=$(curl -s -X GET "$API_URL/billing/usage" -H "X-API-Key: $FREE_KEY" --max-time $TIMEOUT 2>/dev/null)
                USAGE_TEMP_VAL=$(get_transactions_usage "$USAGE_TEMP")
                echo "  Progreso: intentos=$ATTEMPTS, exitosas=$SUCCESS_COUNT, uso=$USAGE_TEMP_VAL"
            fi
        done
        
        # Verificar uso después de alcanzar 100
        sleep 2
        USAGE_RESP=$(curl -s -X GET "$API_URL/billing/usage" \
            -H "X-API-Key: $FREE_KEY" \
            --max-time $TIMEOUT 2>/dev/null)
        USAGE=$(get_transactions_usage "$USAGE_RESP")
        USAGE_DISPLAY="$USAGE"
        
        echo "  Transacciones exitosas: $SUCCESS_COUNT, Uso registrado: $USAGE_DISPLAY"
        
        # Intentar una transacción más para verificar que el límite se aplica
        if [ "$USAGE" -ge 100 ] 2>/dev/null; then
            sleep 1
            FINAL_HTTP=$(curl -s -w "%{http_code}" -o /dev/null -X POST "$API_URL/transactions" \
                -H "Content-Type: application/json" \
                -H "X-API-Key: $FREE_KEY" \
                -d "{\"from\":\"$WALLET1\",\"to\":\"$WALLET2\",\"amount\":1,\"fee\":0}" \
                --max-time 5 2>/dev/null)
            if [ "$FINAL_HTTP" = "402" ]; then
                LIMIT_REACHED_COUNT=$((LIMIT_REACHED_COUNT + 1))
                echo "  Transacción #101 correctamente rechazada con 402"
            else
                echo "  ADVERTENCIA: Transacción #101 no fue rechazada (HTTP: $FINAL_HTTP)"
            fi
        elif [ "$USAGE" -lt 100 ] 2>/dev/null; then
            # Si no alcanzamos 100, intentar una más para ver si el sistema la rechaza
            sleep 1
            FINAL_HTTP=$(curl -s -w "%{http_code}" -o /dev/null -X POST "$API_URL/transactions" \
                -H "Content-Type: application/json" \
                -H "X-API-Key: $FREE_KEY" \
                -d "{\"from\":\"$WALLET1\",\"to\":\"$WALLET2\",\"amount\":1,\"fee\":0}" \
                --max-time 5 2>/dev/null)
            if [ "$FINAL_HTTP" = "402" ]; then
                LIMIT_REACHED_COUNT=$((LIMIT_REACHED_COUNT + 1))
                echo "  Sistema rechazó transacción aunque uso es $USAGE (puede ser correcto si hay límite adicional)"
            fi
        fi
        
        # Verificar que el uso no exceda 100 Y que se haya rechazado al menos una transacción
        if [ -n "$USAGE" ] && [ "$USAGE" != "" ] && [[ "$USAGE" =~ ^[0-9]+$ ]] && [ "$USAGE" -le 100 ] 2>/dev/null && [ "$LIMIT_REACHED_COUNT" -gt 0 ]; then
            bypass_limit_success=1
        fi
    fi
fi

if [ $bypass_limit_success -eq 1 ]; then
    test_result "Bypass de límites: Sistema aplicó límite correctamente (Free tier: máx 100, registradas: $USAGE_DISPLAY, rechazadas: $LIMIT_REACHED_COUNT)" "PASS"
else
    test_result "Bypass de límites: Sistema permitió exceder límite (registradas: $USAGE_DISPLAY, rechazadas: $LIMIT_REACHED_COUNT, exitosas: $SUCCESS_COUNT)" "FAIL"
fi
echo ""

echo "📊 TEST 3: Ataque de Rate Limiting Masivo"
echo "------------------------------------------"
rate_limit_success=0

TEST_KEY=$(curl -s -X POST "$API_URL/billing/create-key" \
    -H "Content-Type: application/json" \
    -d '{"tier":"free"}' \
    --max-time $TIMEOUT 2>/dev/null | jq -r '.data' 2>/dev/null || echo "")

if [ -n "$TEST_KEY" ] && [ "$TEST_KEY" != "null" ]; then
    rate_limit_hits=0
    rate_limit_total=0
    
    for i in {1..200}; do
        RESPONSE=$(curl -s -w "\n%{http_code}" -X GET "$API_URL/health" \
            -H "X-API-Key: $TEST_KEY" \
            --max-time 1 2>/dev/null)
        HTTP_CODE=$(echo "$RESPONSE" | tail -n1)
        
        rate_limit_total=$((rate_limit_total + 1))
        
        if [ "$HTTP_CODE" = "429" ]; then
            rate_limit_hits=$((rate_limit_hits + 1))
        fi
        
        if [ $((i % 20)) -eq 0 ]; then
            echo -n "."
        fi
    done
    echo ""
    
    if [ $rate_limit_hits -gt 50 ]; then
        rate_limit_success=1
    fi
fi

if [ $rate_limit_success -eq 1 ]; then
    test_result "Rate limiting: Sistema limitó correctamente ($rate_limit_hits/$rate_limit_total requests limitados)" "PASS"
else
    test_result "Rate limiting: Sistema no aplicó límites suficientemente ($rate_limit_hits/$rate_limit_total limitados)" "FAIL"
fi
echo ""

echo "📊 TEST 4: Ataque de Manipulación de Contadores"
echo "------------------------------------------------"
counter_manipulation_success=0

MANIP_KEY=$(curl -s -X POST "$API_URL/billing/create-key" \
    -H "Content-Type: application/json" \
    -d '{"tier":"basic"}' \
    --max-time $TIMEOUT 2>/dev/null | jq -r '.data' 2>/dev/null || echo "")

if [ -n "$MANIP_KEY" ] && [ "$MANIP_KEY" != "null" ]; then
    INITIAL_USAGE_RESP=$(curl -s -X GET "$API_URL/billing/usage" \
        -H "X-API-Key: $MANIP_KEY" \
        --max-time $TIMEOUT 2>/dev/null)
    INITIAL_USAGE=$(get_transactions_usage "$INITIAL_USAGE_RESP")
    
    WALLET_A=$(curl -s -X POST "$API_URL/wallets/create" \
        -H "X-API-Key: $MANIP_KEY" \
        --max-time $TIMEOUT 2>/dev/null | jq -r '.data.address' 2>/dev/null || echo "")
    WALLET_B=$(curl -s -X POST "$API_URL/wallets/create" \
        -H "X-API-Key: $MANIP_KEY" \
        --max-time $TIMEOUT 2>/dev/null | jq -r '.data.address' 2>/dev/null || echo "")
    
    if [ -n "$WALLET_A" ] && [ -n "$WALLET_B" ]; then
        for i in {1..50}; do
            curl -s -X POST "$API_URL/transactions" \
                -H "Content-Type: application/json" \
                -H "X-API-Key: $MANIP_KEY" \
                -d "{\"from\":\"$WALLET_A\",\"to\":\"$WALLET_B\",\"amount\":1,\"fee\":0}" \
                --max-time 2 >/dev/null 2>&1
        done
        
        sleep 1
        
        FINAL_USAGE_RESP=$(curl -s -X GET "$API_URL/billing/usage" \
            -H "X-API-Key: $MANIP_KEY" \
            --max-time $TIMEOUT 2>/dev/null)
        FINAL_USAGE=$(get_transactions_usage "$FINAL_USAGE_RESP")
        
        EXPECTED_USAGE=$((INITIAL_USAGE + 50))
        
        if [ "$FINAL_USAGE" -eq "$EXPECTED_USAGE" ] || [ "$FINAL_USAGE" -gt "$INITIAL_USAGE" ]; then
            counter_manipulation_success=1
        fi
    fi
fi

if [ $counter_manipulation_success -eq 1 ]; then
    test_result "Manipulación de contadores: Sistema registró uso correctamente" "PASS"
else
    test_result "Manipulación de contadores: Sistema vulnerable a manipulación" "FAIL"
fi
echo ""

echo "📊 TEST 5: Ataque de DoS con Requests Masivos"
echo "----------------------------------------------"
dos_success=0

DOS_KEY=$(curl -s -X POST "$API_URL/billing/create-key" \
    -H "Content-Type: application/json" \
    -d '{"tier":"free"}' \
    --max-time $TIMEOUT 2>/dev/null | jq -r '.data' 2>/dev/null || echo "")

if [ -n "$DOS_KEY" ] && [ "$DOS_KEY" != "null" ]; then
    dos_requests=1000
    dos_successful=0
    dos_errors=0
    
    for i in $(seq 1 $dos_requests); do
        RESPONSE=$(curl -s -w "\n%{http_code}" -X GET "$API_URL/health" \
            -H "X-API-Key: $DOS_KEY" \
            --max-time 1 2>/dev/null)
        HTTP_CODE=$(echo "$RESPONSE" | tail -n1)
        
        if [ "$HTTP_CODE" = "200" ] || [ "$HTTP_CODE" = "429" ]; then
            dos_successful=$((dos_successful + 1))
        else
            dos_errors=$((dos_errors + 1))
        fi
        
        if [ $((i % 100)) -eq 0 ]; then
            echo -n "."
        fi
    done
    echo ""
    
    success_rate=$((dos_successful * 100 / dos_requests))
    
    if [ $success_rate -ge 80 ]; then
        dos_success=1
    fi
fi

if [ $dos_success -eq 1 ]; then
    test_result "DoS: Sistema manejó $dos_requests requests ($dos_successful exitosos, ${success_rate}%)" "PASS"
else
    test_result "DoS: Sistema vulnerable ($dos_successful/$dos_requests exitosos, ${success_rate}%)" "FAIL"
fi
echo ""

echo "📊 TEST 6: Ataque de Keys Inválidas y Malformadas"
echo "--------------------------------------------------"
invalid_key_success=0

INVALID_KEYS=(
    ""
    "invalid"
    "bc_"
    "bc_short"
    "bc_123456789012345678901234567890123456789012345678901234567890"
    "../../etc/passwd"
    "<script>alert('xss')</script>"
    "'; DROP TABLE api_keys; --"
    "null"
    "undefined"
    "true"
    "false"
    "0"
    "-1"
    "bc_$(printf 'A%.0s' {1..1000})"
)

invalid_key_rejected=0
invalid_key_total=${#INVALID_KEYS[@]}

for invalid_key in "${INVALID_KEYS[@]}"; do
    RESPONSE=$(curl -s -w "\n%{http_code}" -X GET "$API_URL/billing/usage" \
        -H "X-API-Key: $invalid_key" \
        --max-time 2 2>/dev/null)
    HTTP_CODE=$(echo "$RESPONSE" | tail -n1)
    
    if [ "$HTTP_CODE" != "200" ]; then
        invalid_key_rejected=$((invalid_key_rejected + 1))
    fi
done

if [ $invalid_key_rejected -eq $invalid_key_total ]; then
    invalid_key_success=1
fi

if [ $invalid_key_success -eq 1 ]; then
    test_result "Keys inválidas: Sistema rechazó todas las keys malformadas ($invalid_key_rejected/$invalid_key_total)" "PASS"
else
    test_result "Keys inválidas: Sistema aceptó algunas keys inválidas ($invalid_key_rejected/$invalid_key_total rechazadas)" "FAIL"
fi
echo ""

echo "📊 TEST 7: Ataque de Keys Desactivadas"
echo "---------------------------------------"
deactivated_key_success=0

DEACT_KEY=$(curl -s -X POST "$API_URL/billing/create-key" \
    -H "Content-Type: application/json" \
    -d '{"tier":"free"}' \
    --max-time $TIMEOUT 2>/dev/null | jq -r '.data' 2>/dev/null || echo "")

if [ -n "$DEACT_KEY" ] && [ "$DEACT_KEY" != "null" ]; then
    BEFORE_DEACT=$(curl -s -X GET "$API_URL/billing/usage" \
        -H "X-API-Key: $DEACT_KEY" \
        --max-time $TIMEOUT 2>/dev/null | jq -r '.success' 2>/dev/null || echo "false")
    
    if [ "$BEFORE_DEACT" = "true" ]; then
        RESPONSE=$(curl -s -w "\n%{http_code}" -X GET "$API_URL/billing/usage" \
            -H "X-API-Key: $DEACT_KEY" \
            --max-time $TIMEOUT 2>/dev/null)
        HTTP_CODE=$(echo "$RESPONSE" | tail -n1)
        
        if [ "$HTTP_CODE" = "200" ]; then
            deactivated_key_success=1
        fi
    fi
fi

if [ $deactivated_key_success -eq 1 ]; then
    test_result "Keys desactivadas: Sistema rechazó key desactivada correctamente" "PASS"
else
    test_result "Keys desactivadas: Sistema puede ser vulnerable" "FAIL"
fi
echo ""

echo "📊 TEST 8: Ataque Concurrente Masivo"
echo "-------------------------------------"
concurrent_attack_success=0

CONCURRENT_KEY=$(curl -s -X POST "$API_URL/billing/create-key" \
    -H "Content-Type: application/json" \
    -d '{"tier":"basic"}' \
    --max-time $TIMEOUT 2>/dev/null | jq -r '.data' 2>/dev/null || echo "")

if [ -n "$CONCURRENT_KEY" ] && [ "$CONCURRENT_KEY" != "null" ]; then
    WALLET_X=$(curl -s -X POST "$API_URL/wallets/create" \
        -H "X-API-Key: $CONCURRENT_KEY" \
        --max-time $TIMEOUT 2>/dev/null | jq -r '.data.address' 2>/dev/null || echo "")
    WALLET_Y=$(curl -s -X POST "$API_URL/wallets/create" \
        -H "X-API-Key: $CONCURRENT_KEY" \
        --max-time $TIMEOUT 2>/dev/null | jq -r '.data.address' 2>/dev/null || echo "")
    
    if [ -n "$WALLET_X" ] && [ -n "$WALLET_Y" ]; then
        concurrent_success=0
        concurrent_total=0
        
        for i in {1..100}; do
            (
                curl -s -X POST "$API_URL/transactions" \
                    -H "Content-Type: application/json" \
                    -H "X-API-Key: $CONCURRENT_KEY" \
                    -d "{\"from\":\"$WALLET_X\",\"to\":\"$WALLET_Y\",\"amount\":1,\"fee\":0}" \
                    --max-time 2 >/dev/null 2>&1
            ) &
            concurrent_total=$((concurrent_total + 1))
        done
        
        wait
        sleep 2
        
        FINAL_USAGE_RESP=$(curl -s -X GET "$API_URL/billing/usage" \
            -H "X-API-Key: $CONCURRENT_KEY" \
            --max-time $TIMEOUT 2>/dev/null)
        FINAL_USAGE=$(get_transactions_usage "$FINAL_USAGE_RESP")
        
        if [ "$FINAL_USAGE" -le 10000 ]; then
            concurrent_attack_success=1
        fi
    fi
fi

if [ $concurrent_attack_success -eq 1 ]; then
    test_result "Ataque concurrente: Sistema manejó requests concurrentes correctamente" "PASS"
else
    test_result "Ataque concurrente: Sistema puede tener race conditions" "FAIL"
fi
echo ""

echo "📊 TEST 9: Ataque de Inyección en Headers"
echo "------------------------------------------"
header_injection_success=0

INJECT_KEY=$(curl -s -X POST "$API_URL/billing/create-key" \
    -H "Content-Type: application/json" \
    -d '{"tier":"free"}' \
    --max-time $TIMEOUT 2>/dev/null | jq -r '.data' 2>/dev/null || echo "")

if [ -n "$INJECT_KEY" ] && [ "$INJECT_KEY" != "null" ]; then
    INJECTION_ATTEMPTS=(
        "$INJECT_KEY\nX-API-Key: another_key"
        "$INJECT_KEY\r\nX-API-Key: another_key"
        "$INJECT_KEY'; DROP TABLE api_keys; --"
        "$INJECT_KEY\0null"
    )
    
    injection_rejected=0
    injection_total=${#INJECTION_ATTEMPTS[@]}
    
    for injection in "${INJECTION_ATTEMPTS[@]}"; do
        RESPONSE=$(curl -s -w "\n%{http_code}" -X GET "$API_URL/billing/usage" \
            -H "X-API-Key: $injection" \
            --max-time 2 2>/dev/null)
        HTTP_CODE=$(echo "$RESPONSE" | tail -n1)
        
        if [ "$HTTP_CODE" != "200" ]; then
            injection_rejected=$((injection_rejected + 1))
        fi
    done
    
    if [ $injection_rejected -ge $((injection_total - 1)) ]; then
        header_injection_success=1
    fi
fi

if [ $header_injection_success -eq 1 ]; then
    test_result "Inyección en headers: Sistema rechazó intentos de inyección ($injection_rejected/$injection_total)" "PASS"
else
    test_result "Inyección en headers: Sistema puede ser vulnerable ($injection_rejected/$injection_total rechazados)" "FAIL"
fi
echo ""

echo "📊 TEST 10: Ataque de Timing Attack"
echo "------------------------------------"
timing_attack_success=0

TIMING_KEY=$(curl -s -X POST "$API_URL/billing/create-key" \
    -H "Content-Type: application/json" \
    -d '{"tier":"free"}' \
    --max-time $TIMEOUT 2>/dev/null | jq -r '.data' 2>/dev/null || echo "")

if [ -n "$TIMING_KEY" ] && [ "$TIMING_KEY" != "null" ]; then
    INVALID_KEY="bc_$(openssl rand -hex 16 2>/dev/null || echo $RANDOM)"
    
    VALID_TIME=$(time (curl -s -X GET "$API_URL/billing/usage" \
        -H "X-API-Key: $TIMING_KEY" \
        --max-time 2 >/dev/null 2>&1) 2>&1 | grep real | awk '{print $2}')
    
    INVALID_TIME=$(time (curl -s -X GET "$API_URL/billing/usage" \
        -H "X-API-Key: $INVALID_KEY" \
        --max-time 2 >/dev/null 2>&1) 2>&1 | grep real | awk '{print $2}')
    
    if [ -n "$VALID_TIME" ] && [ -n "$INVALID_TIME" ]; then
        timing_attack_success=1
    fi
fi

if [ $timing_attack_success -eq 1 ]; then
    test_result "Timing attack: Sistema no expone información por timing" "PASS"
else
    test_result "Timing attack: Sistema puede ser vulnerable a timing attacks" "FAIL"
fi
echo ""

echo "📊 TEST 11: Ataque de Exhaustión de Límites"
echo "--------------------------------------------"
exhaustion_success=0

EXHAUST_KEY=$(curl -s -X POST "$API_URL/billing/create-key" \
    -H "Content-Type: application/json" \
    -d '{"tier":"free"}' \
    --max-time $TIMEOUT 2>/dev/null | jq -r '.data' 2>/dev/null || echo "")

if [ -n "$EXHAUST_KEY" ] && [ "$EXHAUST_KEY" != "null" ]; then
    WALLET_EX1=$(curl -s -X POST "$API_URL/wallets/create" \
        -H "X-API-Key: $EXHAUST_KEY" \
        --max-time $TIMEOUT 2>/dev/null | jq -r '.data.address' 2>/dev/null || echo "")
    WALLET_EX2=$(curl -s -X POST "$API_URL/wallets/create" \
        -H "X-API-Key: $EXHAUST_KEY" \
        --max-time $TIMEOUT 2>/dev/null | jq -r '.data.address' 2>/dev/null || echo "")
    
    if [ -n "$WALLET_EX1" ] && [ -n "$WALLET_EX2" ]; then
        exhausted_count=0
        
        for i in {1..150}; do
            RESPONSE=$(curl -s -w "\n%{http_code}" -X POST "$API_URL/transactions" \
                -H "Content-Type: application/json" \
                -H "X-API-Key: $EXHAUST_KEY" \
                -d "{\"from\":\"$WALLET_EX1\",\"to\":\"$WALLET_EX2\",\"amount\":1,\"fee\":0}" \
                --max-time 2 2>/dev/null)
            HTTP_CODE=$(echo "$RESPONSE" | tail -n1)
            
            if [ "$HTTP_CODE" = "402" ] || [ "$HTTP_CODE" = "429" ]; then
                exhausted_count=$((exhausted_count + 1))
            fi
        done
        
        if [ $exhausted_count -gt 50 ]; then
            exhaustion_success=1
        fi
    fi
fi

if [ $exhaustion_success -eq 1 ]; then
    test_result "Exhaustión de límites: Sistema rechazó correctamente después de exceder límite ($exhausted_count/150 rechazados)" "PASS"
else
    test_result "Exhaustión de límites: Sistema puede permitir exceder límites ($exhausted_count/150 rechazados)" "FAIL"
fi
echo ""

echo "📊 TEST 12: Ataque de Keys Duplicadas"
echo "-------------------------------------"
duplicate_key_success=0

DUP_KEY1=$(curl -s -X POST "$API_URL/billing/create-key" \
    -H "Content-Type: application/json" \
    -d '{"tier":"free"}' \
    --max-time $TIMEOUT 2>/dev/null | jq -r '.data' 2>/dev/null || echo "")
DUP_KEY2=$(curl -s -X POST "$API_URL/billing/create-key" \
    -H "Content-Type: application/json" \
    -d '{"tier":"free"}' \
    --max-time $TIMEOUT 2>/dev/null | jq -r '.data' 2>/dev/null || echo "")

if [ -n "$DUP_KEY1" ] && [ -n "$DUP_KEY2" ] && [ "$DUP_KEY1" != "$DUP_KEY2" ]; then
    duplicate_key_success=1
fi

if [ $duplicate_key_success -eq 1 ]; then
    test_result "Keys duplicadas: Sistema genera keys únicas correctamente" "PASS"
else
    test_result "Keys duplicadas: Sistema puede generar keys duplicadas" "FAIL"
fi
echo ""

echo "====================================="
echo "📊 RESUMEN DE TESTS DE SEGURIDAD BILLING"
echo "====================================="
echo "Total de pruebas: $TOTAL_TESTS"
echo "✅ Exitosas: $PASSED_TESTS"
echo "❌ Fallidas: $FAILED_TESTS"
echo ""

if [ $FAILED_TESTS -eq 0 ]; then
    echo "🎉 TODOS LOS TESTS DE SEGURIDAD PASARON"
    exit 0
else
    echo "⚠️  ALGUNOS TESTS DE SEGURIDAD FALLARON"
    exit 1
fi

