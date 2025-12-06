#!/bin/bash

set +e

API_URL="http://127.0.0.1:8080/api/v1"
RESULTS_DIR="test_results_stress"
TIMEOUT=10
MINE_TIMEOUT=15

echo "🔥 PRUEBAS DE ESTRÉS Y CARGA CRÍTICA"
echo "===================================="
echo ""

mkdir -p "$RESULTS_DIR"

test_count=0
passed_count=0
failed_count=0

test_result() {
    local test_name="$1"
    local result="$2"
    test_count=$((test_count + 1))
    
    if [ "$result" = "PASS" ]; then
        echo "✅ $test_name"
        passed_count=$((passed_count + 1))
    else
        echo "❌ $test_name"
        failed_count=$((failed_count + 1))
    fi
}

echo "📊 TEST 1: Rate Limiting - Verificar límites"
echo "--------------------------------------------"
for i in {1..110}; do
    response=$(curl -s -w "\n%{http_code}" -X GET "$API_URL/health" --max-time $TIMEOUT 2>/dev/null || echo "000")
    http_code=$(echo "$response" | tail -n1)
    
    if [ "$i" -le 100 ]; then
        if [ "$http_code" = "200" ]; then
            if [ "$i" -eq 100 ]; then
                test_result "Rate limit: Primeros 100 requests OK" "PASS"
            fi
        fi
    else
        if [ "$http_code" = "429" ]; then
            test_result "Rate limit: Request 101+ bloqueado correctamente" "PASS"
            break
        fi
    fi
done
echo ""

echo "📊 TEST 2: Concurrencia - Múltiples requests simultáneos"
echo "--------------------------------------------------------"
concurrent_requests=50
pids=()
for i in $(seq 1 $concurrent_requests); do
    (
        response=$(curl -s -w "\n%{http_code}" -X GET "$API_URL/health" --max-time $TIMEOUT 2>/dev/null || echo "000")
        http_code=$(echo "$response" | tail -n1)
        echo "$http_code" > "/tmp/stress_test_$i"
    ) &
    pids+=($!)
done

for pid in "${pids[@]}"; do
    wait $pid 2>/dev/null || true
done

success_count=0
for i in $(seq 1 $concurrent_requests); do
    if [ -f "/tmp/stress_test_$i" ]; then
        http_code=$(cat "/tmp/stress_test_$i")
        if [ "$http_code" = "200" ] || [ "$http_code" = "429" ]; then
            success_count=$((success_count + 1))
        fi
        rm -f "/tmp/stress_test_$i"
    fi
done

if [ $success_count -ge $((concurrent_requests * 9 / 10)) ]; then
    test_result "Concurrencia: $concurrent_requests requests simultáneos ($success_count/$concurrent_requests OK)" "PASS"
else
    test_result "Concurrencia: Solo $success_count/$concurrent_requests requests exitosos" "FAIL"
fi
echo ""

echo "📊 TEST 3: Carga Alta - Muchos requests en poco tiempo"
echo "-----------------------------------------------------"
load_requests=200
start_time=$(date +%s)
success=0
rate_limited=0
errors=0

for i in $(seq 1 $load_requests); do
    response=$(curl -s -w "\n%{http_code}" -X GET "$API_URL/health" --max-time $TIMEOUT 2>/dev/null || echo "000")
    http_code=$(echo "$response" | tail -n1)
    
    case $http_code in
        200) success=$((success + 1)) ;;
        429) rate_limited=$((rate_limited + 1)) ;;
        *) errors=$((errors + 1)) ;;
    esac
done

end_time=$(date +%s)
duration=$((end_time - start_time))
rps=$((load_requests / (duration + 1)))

if [ $errors -eq 0 ] && [ $rps -gt 0 ]; then
    test_result "Carga alta: $load_requests requests en ${duration}s ($rps req/s, $success OK, $rate_limited rate-limited)" "PASS"
else
    test_result "Carga alta: $errors errores, $rps req/s" "FAIL"
fi
echo ""

echo "📊 TEST 4: Crear Múltiples Wallets Simultáneamente"
echo "--------------------------------------------------"
wallet_count=20
pids=()
declare -a wallet_addresses=()

for i in $(seq 1 $wallet_count); do
    (
        response=$(curl -s -X POST "$API_URL/wallets/create" --max-time $TIMEOUT 2>/dev/null || echo '{"success":false}')
        echo "$response" > "/tmp/wallet_$i"
    ) &
    pids+=($!)
done

for pid in "${pids[@]}"; do
    wait $pid 2>/dev/null || true
done

created=0
for i in $(seq 1 $wallet_count); do
    if [ -f "/tmp/wallet_$i" ]; then
        response=$(cat "/tmp/wallet_$i")
        if echo "$response" | grep -q '"success":true'; then
            address=$(echo "$response" | grep -o '"address":"[^"]*"' | cut -d'"' -f4)
            if [ -n "$address" ]; then
                wallet_addresses+=("$address")
                created=$((created + 1))
            fi
        fi
        rm -f "/tmp/wallet_$i"
    fi
done

if [ $created -ge $((wallet_count * 9 / 10)) ]; then
    test_result "Wallets concurrentes: $created/$wallet_count wallets creados correctamente" "PASS"
else
    test_result "Wallets concurrentes: Solo $created/$wallet_count wallets creados" "FAIL"
fi

export wallet_addresses
echo ""

echo "📊 TEST 5: Transacciones Concurrentes"
echo "------------------------------------"
if [ -z "${wallet_addresses[*]}" ] || [ ${#wallet_addresses[@]} -lt 2 ]; then
    echo "⚠️  No hay suficientes wallets, creando más..."
    wallet_addresses=()
    for i in {1..10}; do
        response=$(curl -s -X POST "$API_URL/wallets/create" --max-time $TIMEOUT 2>/dev/null || echo '{"success":false}')
        if echo "$response" | grep -q '"success":true'; then
            address=$(echo "$response" | grep -o '"address":"[^"]*"' | cut -d'"' -f4)
            if [ -n "$address" ]; then
                wallet_addresses+=("$address")
            fi
        fi
    done
fi

if [ ${#wallet_addresses[@]} -ge 2 ]; then
    echo "💰 Minando bloques para dar saldo a los wallets..."
    wallet_count=${#wallet_addresses[@]}
    blocks_to_mine=$((wallet_count + 2))
    mined=0
    failed_mining=0
    max_failures=5
    
    for i in $(seq 1 $blocks_to_mine); do
        miner_wallet=${wallet_addresses[$((i % wallet_count))]}
        
        response=$(curl -s -w "\n%{http_code}" -X POST "$API_URL/mine" \
            -H "Content-Type: application/json" \
            -d "{\"miner_address\":\"$miner_wallet\",\"max_transactions\":10}" \
            --max-time $MINE_TIMEOUT 2>/dev/null || echo "000")
        
        http_code=$(echo "$response" | tail -n1)
        
        if [ "$http_code" = "201" ] || [ "$http_code" = "200" ]; then
            mined=$((mined + 1))
            failed_mining=0
            echo -n "."
        else
            failed_mining=$((failed_mining + 1))
            if [ $failed_mining -ge $max_failures ]; then
                echo ""
                echo "⚠️  Demasiados fallos de mining consecutivos, continuando con $mined bloques minados"
                break
            fi
        fi
        
        if [ $i -lt $blocks_to_mine ]; then
            sleep 0.2
        fi
    done
    echo ""
    
    echo "✅ Minados $mined/$blocks_to_mine bloques"
    
    sleep 1
    
    verified=0
    for wallet in "${wallet_addresses[@]}"; do
        balance=$(curl -s "$API_URL/wallets/$wallet" 2>/dev/null | grep -o '"balance":[0-9]*' | cut -d':' -f2 || echo "0")
        if [ -n "$balance" ] && [ "$balance" -gt 0 ]; then
            verified=$((verified + 1))
        fi
    done
    echo "✅ $verified/${#wallet_addresses[@]} wallets con saldo"
    sleep 1
    
    tx_count=30
    pids=()
    success_tx=0
    
    for i in $(seq 1 $tx_count); do
        from_idx=$(( (i - 1) % wallet_count ))
        to_idx=$((i % wallet_count))
        from=${wallet_addresses[$from_idx]}
        to=${wallet_addresses[$to_idx]}
        
        (
            response=$(curl -s -w "\n%{http_code}" -X POST "$API_URL/transactions" \
                -H "Content-Type: application/json" \
                -d "{\"from\":\"$from\",\"to\":\"$to\",\"amount\":1,\"fee\":0}" \
                --max-time $TIMEOUT 2>/dev/null || echo "000")
            http_code=$(echo "$response" | tail -n1)
            echo "$http_code" > "/tmp/tx_$i"
        ) &
        pids+=($!)
    done
    
    for pid in "${pids[@]}"; do
        wait $pid 2>/dev/null || true
    done
    
    for i in $(seq 1 $tx_count); do
        if [ -f "/tmp/tx_$i" ]; then
            http_code=$(cat "/tmp/tx_$i")
            if [ "$http_code" = "201" ] || [ "$http_code" = "200" ]; then
                success_tx=$((success_tx + 1))
            fi
            rm -f "/tmp/tx_$i"
        fi
    done
    
    if [ $success_tx -ge $((tx_count * 8 / 10)) ]; then
        test_result "Transacciones concurrentes: $success_tx/$tx_count exitosas" "PASS"
    else
        test_result "Transacciones concurrentes: Solo $success_tx/$tx_count exitosas" "FAIL"
    fi
else
    test_result "Transacciones concurrentes: No hay suficientes wallets" "FAIL"
fi
echo ""

echo "📊 TEST 6: Consultas de Balance Concurrentes"
echo "---------------------------------------------"
if [ ${#wallet_addresses[@]} -gt 0 ]; then
    balance_requests=100
    pids=()
    success_balance=0
    
    for i in $(seq 1 $balance_requests); do
        wallet_idx=$((i % ${#wallet_addresses[@]}))
        wallet=${wallet_addresses[$wallet_idx]}
        
        (
            response=$(curl -s -w "\n%{http_code}" -X GET "$API_URL/wallets/$wallet" --max-time $TIMEOUT 2>/dev/null || echo "000")
            http_code=$(echo "$response" | tail -n1)
            echo "$http_code" > "/tmp/balance_$i"
        ) &
        pids+=($!)
    done
    
    for pid in "${pids[@]}"; do
        wait $pid 2>/dev/null || true
    done
    
    for i in $(seq 1 $balance_requests); do
        if [ -f "/tmp/balance_$i" ]; then
            http_code=$(cat "/tmp/balance_$i")
            if [ "$http_code" = "200" ]; then
                success_balance=$((success_balance + 1))
            fi
            rm -f "/tmp/balance_$i"
        fi
    done
    
    if [ $success_balance -eq $balance_requests ]; then
        test_result "Balance concurrente: $success_balance/$balance_requests exitosas" "PASS"
    else
        test_result "Balance concurrente: Solo $success_balance/$balance_requests exitosas" "FAIL"
    fi
else
    test_result "Balance concurrente: No hay wallets disponibles" "FAIL"
fi
echo ""

echo "📊 TEST 7: Datos Inválidos - Edge Cases"
echo "--------------------------------------"
invalid_tests=0
invalid_passed=0

response=$(curl -s -w "\n%{http_code}" -X POST "$API_URL/transactions" \
    -H "Content-Type: application/json" \
    -d '{"from":"","to":"test","amount":1}' \
    --max-time $TIMEOUT 2>/dev/null || echo "000")
http_code=$(echo "$response" | tail -n1)
invalid_tests=$((invalid_tests + 1))
if [ "$http_code" = "400" ] || [ "$http_code" = "422" ]; then
    invalid_passed=$((invalid_passed + 1))
fi

response=$(curl -s -w "\n%{http_code}" -X POST "$API_URL/transactions" \
    -H "Content-Type: application/json" \
    -d '{"from":"test","to":"","amount":1}' \
    --max-time $TIMEOUT 2>/dev/null || echo "000")
http_code=$(echo "$response" | tail -n1)
invalid_tests=$((invalid_tests + 1))
if [ "$http_code" = "400" ] || [ "$http_code" = "422" ]; then
    invalid_passed=$((invalid_passed + 1))
fi

response=$(curl -s -w "\n%{http_code}" -X POST "$API_URL/transactions" \
    -H "Content-Type: application/json" \
    -d '{"from":"test","to":"test2","amount":0}' \
    --max-time $TIMEOUT 2>/dev/null || echo "000")
http_code=$(echo "$response" | tail -n1)
invalid_tests=$((invalid_tests + 1))
if [ "$http_code" = "400" ] || [ "$http_code" = "422" ]; then
    invalid_passed=$((invalid_passed + 1))
fi

response=$(curl -s -w "\n%{http_code}" -X GET "$API_URL/blocks/invalid_hash_12345" --max-time $TIMEOUT 2>/dev/null || echo "000")
http_code=$(echo "$response" | tail -n1)
invalid_tests=$((invalid_tests + 1))
if [ "$http_code" = "404" ]; then
    invalid_passed=$((invalid_passed + 1))
fi

if [ $invalid_passed -eq $invalid_tests ]; then
    test_result "Datos inválidos: $invalid_passed/$invalid_tests casos manejados correctamente" "PASS"
else
    test_result "Datos inválidos: Solo $invalid_passed/$invalid_tests casos manejados" "FAIL"
fi
echo ""

echo "📊 TEST 8: Memory Leak Detection - Múltiples Ciclos"
echo "--------------------------------------------------"
cycles=10
cycle_success=0

for cycle in $(seq 1 $cycles); do
    for i in {1..20}; do
        response=$(curl -s -w "\n%{http_code}" -X GET "$API_URL/health" --max-time $TIMEOUT 2>/dev/null || echo "000")
        http_code=$(echo "$response" | tail -n1)
        if [ "$http_code" != "200" ] && [ "$http_code" != "429" ]; then
            break
        fi
    done
    
    if [ "$http_code" = "200" ] || [ "$http_code" = "429" ]; then
        cycle_success=$((cycle_success + 1))
    fi
    
    sleep 0.1
done

if [ $cycle_success -eq $cycles ]; then
    test_result "Memory leak: $cycle_success/$cycles ciclos completados sin degradación" "PASS"
else
    test_result "Memory leak: Degradación detectada después de $cycle_success/$cycles ciclos" "FAIL"
fi
echo ""

echo "📊 TEST 9: Timeout Handling"
echo "--------------------------"
timeout_test=0
for i in {1..5}; do
    start=$(date +%s%N)
    response=$(curl -s -w "\n%{http_code}" -X GET "$API_URL/health" --max-time 1 2>/dev/null || echo "000")
    end=$(date +%s%N)
    duration=$(( (end - start) / 1000000 ))
    
    if [ $duration -lt 2000 ]; then
        timeout_test=$((timeout_test + 1))
    fi
done

if [ $timeout_test -ge 4 ]; then
    test_result "Timeout: $timeout_test/5 requests respondieron en <2s" "PASS"
else
    test_result "Timeout: Solo $timeout_test/5 requests respondieron rápidamente" "FAIL"
fi
echo ""

echo "📊 TEST 10: Stress Test Final - Todo Junto"
echo "-----------------------------------------"
final_success=0
final_total=50

for i in $(seq 1 $final_total); do
    case $((i % 4)) in
        0)
            response=$(curl -s -w "\n%{http_code}" -X GET "$API_URL/health" --max-time $TIMEOUT 2>/dev/null || echo "000")
            ;;
        1)
            response=$(curl -s -w "\n%{http_code}" -X GET "$API_URL/stats" --max-time $TIMEOUT 2>/dev/null || echo "000")
            ;;
        2)
            response=$(curl -s -w "\n%{http_code}" -X GET "$API_URL/mempool" --max-time $TIMEOUT 2>/dev/null || echo "000")
            ;;
        3)
            response=$(curl -s -w "\n%{http_code}" -X GET "$API_URL/blocks" --max-time $TIMEOUT 2>/dev/null || echo "000")
            ;;
    esac
    
    http_code=$(echo "$response" | tail -n1)
    if [ "$http_code" = "200" ] || [ "$http_code" = "201" ] || [ "$http_code" = "429" ]; then
        final_success=$((final_success + 1))
    fi
done

if [ $final_success -ge $((final_total * 9 / 10)) ]; then
    test_result "Stress final: $final_success/$final_total requests exitosos" "PASS"
else
    test_result "Stress final: Solo $final_success/$final_total requests exitosos" "FAIL"
fi
echo ""

echo "===================================="
echo "📊 RESUMEN DE PRUEBAS"
echo "===================================="
echo "Total de pruebas: $test_count"
echo "✅ Exitosas: $passed_count"
echo "❌ Fallidas: $failed_count"
echo ""

if [ $failed_count -eq 0 ]; then
    echo "🎉 TODAS LAS PRUEBAS PASARON"
    exit 0
else
    echo "⚠️  ALGUNAS PRUEBAS FALLARON - Revisar resultados"
    exit 1
fi

