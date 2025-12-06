#!/bin/bash

set +e

API_URL="http://127.0.0.1:8080/api/v1"
DURATION=60
CONCURRENT=10
TIMEOUT=5
RESULTS_FILE="load_test_results_$(date +%Y%m%d_%H%M%S).txt"

echo "🔥 PRUEBA DE CARGA PROLONGADA"
echo "=============================="
echo "Duración: ${DURATION} segundos"
echo "Concurrencia: $CONCURRENT requests simultáneos"
echo "Resultados: $RESULTS_FILE"
echo ""

start_time=$(date +%s)
end_time=$((start_time + DURATION))
total_requests=0
success_requests=0
error_requests=0
rate_limited=0

cleanup() {
    echo ""
    echo "📊 ESTADÍSTICAS FINALES"
    echo "======================"
    echo "Tiempo total: ${DURATION}s"
    echo "Total requests: $total_requests"
    echo "✅ Exitosos: $success_requests"
    echo "❌ Errores: $error_requests"
    echo "🚫 Rate Limited: $rate_limited"
    
    if [ $total_requests -gt 0 ]; then
        rps=$((total_requests / DURATION))
        success_rate=$((success_requests * 100 / total_requests))
        echo "📈 Requests/segundo: $rps"
        echo "📊 Tasa de éxito: ${success_rate}%"
    fi
    
    echo "" >> "$RESULTS_FILE"
    echo "ESTADÍSTICAS FINALES" >> "$RESULTS_FILE"
    echo "Total requests: $total_requests" >> "$RESULTS_FILE"
    echo "Exitosos: $success_requests" >> "$RESULTS_FILE"
    echo "Errores: $error_requests" >> "$RESULTS_FILE"
    echo "Rate Limited: $rate_limited" >> "$RESULTS_FILE"
    if [ $total_requests -gt 0 ]; then
        echo "RPS: $rps" >> "$RESULTS_FILE"
        echo "Tasa de éxito: ${success_rate}%" >> "$RESULTS_FILE"
    fi
    
    exit 0
}

trap cleanup SIGINT SIGTERM

> "$RESULTS_FILE"
echo "LOAD TEST - $(date)" >> "$RESULTS_FILE"
echo "Duración: ${DURATION}s, Concurrencia: $CONCURRENT" >> "$RESULTS_FILE"
echo "" >> "$RESULTS_FILE"

worker() {
    local worker_id=$1
    local worker_requests=0
    local worker_success=0
    local worker_errors=0
    local worker_rate_limited=0
    
    while [ $(date +%s) -lt $end_time ]; do
        endpoint=$((RANDOM % 4))
        case $endpoint in
            0) url="$API_URL/health" ;;
            1) url="$API_URL/stats" ;;
            2) url="$API_URL/mempool" ;;
            3) url="$API_URL/blocks" ;;
        esac
        
        start_ns=$(date +%s%N)
        response=$(curl -s -w "\n%{http_code}\n%{time_total}" -X GET "$url" --max-time $TIMEOUT 2>/dev/null || echo "000\n0.000")
        end_ns=$(date +%s%N)
        
        http_code=$(echo "$response" | tail -n2 | head -n1)
        time_total=$(echo "$response" | tail -n1)
        
        worker_requests=$((worker_requests + 1))
        
        case $http_code in
            200|201)
                worker_success=$((worker_success + 1))
                ;;
            429)
                worker_rate_limited=$((worker_rate_limited + 1))
                ;;
            *)
                worker_errors=$((worker_errors + 1))
                echo "[Worker $worker_id] Error: HTTP $http_code en $url" >> "$RESULTS_FILE"
                ;;
        esac
        
        if [ $(($worker_requests % 10)) -eq 0 ]; then
            echo "[Worker $worker_id] $worker_requests requests (${worker_success} OK, ${worker_errors} errors, ${worker_rate_limited} rate-limited)" >> "$RESULTS_FILE"
        fi
        
        sleep 0.1
    done
    
    echo "$worker_requests $worker_success $worker_errors $worker_rate_limited" > "/tmp/worker_${worker_id}_stats"
}

echo "🚀 Iniciando workers..."
for i in $(seq 1 $CONCURRENT); do
    worker $i &
done

echo "⏳ Ejecutando prueba de carga por ${DURATION} segundos..."
echo "Presiona Ctrl+C para detener temprano"
echo ""

while [ $(date +%s) -lt $end_time ]; do
    sleep 5
    elapsed=$(( $(date +%s) - start_time ))
    remaining=$((DURATION - elapsed))
    echo "⏱️  Tiempo transcurrido: ${elapsed}s / ${DURATION}s (restan ${remaining}s)"
done

echo "🛑 Deteniendo workers..."
wait

total_requests=0
success_requests=0
error_requests=0
rate_limited=0

for i in $(seq 1 $CONCURRENT); do
    if [ -f "/tmp/worker_${i}_stats" ]; then
        stats=$(cat "/tmp/worker_${i}_stats")
        worker_total=$(echo $stats | cut -d' ' -f1)
        worker_success=$(echo $stats | cut -d' ' -f2)
        worker_errors=$(echo $stats | cut -d' ' -f3)
        worker_rl=$(echo $stats | cut -d' ' -f4)
        
        total_requests=$((total_requests + worker_total))
        success_requests=$((success_requests + worker_success))
        error_requests=$((error_requests + worker_errors))
        rate_limited=$((rate_limited + worker_rl))
        
        rm -f "/tmp/worker_${i}_stats"
    fi
done

cleanup

