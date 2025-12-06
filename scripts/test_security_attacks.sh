#!/bin/bash

# Pruebas de Seguridad y Ataques Agresivos
# Este script prueba la resistencia del sistema ante diversos ataques

set +e

API_URL="http://127.0.0.1:8080/api/v1"
RESULTS_DIR="test_results_security"
TIMEOUT=10

echo "🛡️  PRUEBAS DE SEGURIDAD Y ATAQUES AGRESIVOS"
echo "=============================================="
echo ""

# Verificar que el servidor esté corriendo
if ! curl -s --max-time 2 "$API_URL/health" >/dev/null 2>&1; then
    echo "❌ ERROR: El servidor no está respondiendo en $API_URL"
    echo "💡 Inicia el servidor primero: DIFFICULTY=1 cargo run --release 8080 8081 blockchain"
    exit 1
fi

echo "✅ Servidor detectado y respondiendo"
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

echo "📊 TEST 1: Ataque de Doble Gasto"
echo "--------------------------------"
double_spend_success=0
double_spend_attempts=10

WALLET1=$(curl -s --max-time 5 -X POST "$API_URL/wallets/create" 2>/dev/null | jq -r '.data.address' 2>/dev/null || echo "")
WALLET2=$(curl -s --max-time 5 -X POST "$API_URL/wallets/create" 2>/dev/null | jq -r '.data.address' 2>/dev/null || echo "")

if [ -n "$WALLET1" ] && [ -n "$WALLET2" ] && [ "$WALLET1" != "null" ] && [ "$WALLET2" != "null" ]; then
    echo "  Creando wallets: $WALLET1, $WALLET2"
    curl -s --max-time 20 -X POST "$API_URL/mine" \
        -H "Content-Type: application/json" \
        -d "{\"miner_address\":\"$WALLET1\",\"max_transactions\":10}" \
        >/dev/null 2>&1 &
    MINE_PID=$!
    
    sleep 2
    
    BALANCE=$(curl -s "$API_URL/wallets/$WALLET1" | jq -r '.data.balance' 2>/dev/null || echo "0")
    
    if [ "$BALANCE" -gt 10 ]; then
        TX_AMOUNT=$((BALANCE / 2))
        
        TX1_RESPONSE=$(curl -s -w "\n%{http_code}" -X POST "$API_URL/transactions" \
            -H "Content-Type: application/json" \
            -d "{\"from\":\"$WALLET1\",\"to\":\"$WALLET2\",\"amount\":$TX_AMOUNT,\"fee\":0}" \
            --max-time $TIMEOUT 2>/dev/null)
        TX1_CODE=$(echo "$TX1_RESPONSE" | tail -n1)
        
        TX2_RESPONSE=$(curl -s -w "\n%{http_code}" -X POST "$API_URL/transactions" \
            -H "Content-Type: application/json" \
            -d "{\"from\":\"$WALLET1\",\"to\":\"$WALLET2\",\"amount\":$TX_AMOUNT,\"fee\":0}" \
            --max-time $TIMEOUT 2>/dev/null)
        TX2_CODE=$(echo "$TX2_RESPONSE" | tail -n1)
        
        if [ "$TX1_CODE" = "201" ] && [ "$TX2_CODE" != "201" ]; then
            double_spend_success=$((double_spend_success + 1))
        fi
    fi
fi

if [ $double_spend_success -gt 0 ]; then
    test_result "Doble gasto: Sistema rechazó correctamente el segundo intento" "PASS"
else
    test_result "Doble gasto: Sistema no detectó correctamente el ataque" "FAIL"
fi
echo ""

echo "📊 TEST 2: Ataque de Saldo Insuficiente"
echo "----------------------------------------"
insufficient_balance_success=0

WALLET3=$(curl -s --max-time 5 -X POST "$API_URL/wallets/create" 2>/dev/null | jq -r '.data.address' 2>/dev/null || echo "")
WALLET4=$(curl -s --max-time 5 -X POST "$API_URL/wallets/create" 2>/dev/null | jq -r '.data.address' 2>/dev/null || echo "")

if [ -n "$WALLET3" ] && [ -n "$WALLET4" ]; then
    BALANCE=$(curl -s "$API_URL/wallets/$WALLET3" | jq -r '.data.balance' 2>/dev/null || echo "0")
    
    ATTEMPT=$(curl -s -w "\n%{http_code}" -X POST "$API_URL/transactions" \
        -H "Content-Type: application/json" \
        -d "{\"from\":\"$WALLET3\",\"to\":\"$WALLET4\",\"amount\":$((BALANCE + 1000)),\"fee\":0}" \
        --max-time $TIMEOUT 2>/dev/null)
    ATTEMPT_CODE=$(echo "$ATTEMPT" | tail -n1)
    
    if [ "$ATTEMPT_CODE" != "201" ] && [ "$ATTEMPT_CODE" != "200" ]; then
        insufficient_balance_success=1
    fi
fi

if [ $insufficient_balance_success -eq 1 ]; then
    test_result "Saldo insuficiente: Sistema rechazó correctamente la transacción" "PASS"
else
    test_result "Saldo insuficiente: Sistema permitió transacción inválida" "FAIL"
fi
echo ""

echo "📊 TEST 3: Ataque de Spam de Transacciones"
echo "------------------------------------------"
spam_success=0
spam_attempts=100
spam_accepted=0

WALLET5=$(curl -s --max-time 5 -X POST "$API_URL/wallets/create" 2>/dev/null | jq -r '.data.address' 2>/dev/null || echo "")
WALLET6=$(curl -s --max-time 5 -X POST "$API_URL/wallets/create" 2>/dev/null | jq -r '.data.address' 2>/dev/null || echo "")

if [ -n "$WALLET5" ] && [ -n "$WALLET6" ] && [ "$WALLET5" != "null" ] && [ "$WALLET6" != "null" ]; then
    curl -s --max-time 20 -X POST "$API_URL/mine" \
        -H "Content-Type: application/json" \
        -d "{\"miner_address\":\"$WALLET5\",\"max_transactions\":10}" \
        >/dev/null 2>&1 &
    
    sleep 2
    
    for i in $(seq 1 $spam_attempts); do
        RESPONSE=$(curl -s -w "\n%{http_code}" -X POST "$API_URL/transactions" \
            -H "Content-Type: application/json" \
            -d "{\"from\":\"$WALLET5\",\"to\":\"$WALLET6\",\"amount\":1,\"fee\":0}" \
            --max-time $TIMEOUT 2>/dev/null)
        HTTP_CODE=$(echo "$RESPONSE" | tail -n1)
        
        if [ "$HTTP_CODE" = "201" ] || [ "$HTTP_CODE" = "200" ]; then
            spam_accepted=$((spam_accepted + 1))
        fi
    done
    
    if [ $spam_accepted -lt $spam_attempts ]; then
        spam_success=1
    fi
fi

if [ $spam_success -eq 1 ]; then
    test_result "Spam de transacciones: Sistema limitó correctamente ($spam_accepted/$spam_attempts aceptadas)" "PASS"
else
    test_result "Spam de transacciones: Sistema no limitó el spam ($spam_accepted/$spam_attempts aceptadas)" "FAIL"
fi
echo ""

echo "📊 TEST 4: Ataque de Rate Limiting"
echo "----------------------------------"
rate_limit_success=0
rate_limit_attempts=30
rate_limited=0

echo "  Enviando $rate_limit_attempts requests rápidamente (límite: 20/min)..."
for i in $(seq 1 $rate_limit_attempts); do
    RESPONSE=$(curl -s -w "\n%{http_code}" -X GET "$API_URL/health" \
        --max-time 2 2>/dev/null)
    HTTP_CODE=$(echo "$RESPONSE" | tail -n1)
    
    if [ "$HTTP_CODE" = "429" ]; then
        rate_limited=$((rate_limited + 1))
    fi
    
    if [ $((i % 10)) -eq 0 ]; then
        echo -n "."
    fi
done
echo ""

if [ $rate_limited -ge 5 ]; then
    rate_limit_success=1
fi

if [ $rate_limit_success -eq 1 ]; then
    test_result "Rate limiting: Sistema aplicó límites correctamente ($rate_limited/$rate_limit_attempts requests limitados)" "PASS"
else
    test_result "Rate limiting: Sistema no aplicó límites suficientes ($rate_limited/$rate_limit_attempts limitados, esperado: >=5)" "FAIL"
fi
echo ""

echo "📊 TEST 5: Ataque de Firma Inválida"
echo "------------------------------------"
invalid_signature_success=0

WALLET7=$(curl -s --max-time 5 -X POST "$API_URL/wallets/create" 2>/dev/null | jq -r '.data.address' 2>/dev/null || echo "")
WALLET8=$(curl -s --max-time 5 -X POST "$API_URL/wallets/create" 2>/dev/null | jq -r '.data.address' 2>/dev/null || echo "")

if [ -n "$WALLET7" ] && [ -n "$WALLET8" ]; then
    INVALID_TX=$(echo "{\"from\":\"$WALLET7\",\"to\":\"$WALLET8\",\"amount\":10,\"fee\":0,\"signature\":\"invalid_signature_12345\"}" | base64 2>/dev/null || echo "")
    
    RESPONSE=$(curl -s -w "\n%{http_code}" -X POST "$API_URL/transactions" \
        -H "Content-Type: application/json" \
        -d "{\"from\":\"$WALLET7\",\"to\":\"$WALLET8\",\"amount\":10,\"fee\":0,\"signature\":\"invalid_signature_12345\"}" \
        --max-time $TIMEOUT 2>/dev/null)
    HTTP_CODE=$(echo "$RESPONSE" | tail -n1)
    
    if [ "$HTTP_CODE" != "201" ] && [ "$HTTP_CODE" != "200" ]; then
        invalid_signature_success=1
    fi
fi

if [ $invalid_signature_success -eq 1 ]; then
    test_result "Firma inválida: Sistema rechazó correctamente la transacción" "PASS"
else
    test_result "Firma inválida: Sistema aceptó transacción con firma inválida" "FAIL"
fi
echo ""

echo "📊 TEST 6: Ataque de Carga Extrema"
echo "-----------------------------------"
load_test_success=0
load_requests=200
load_success=0
load_errors=0

echo "  Enviando $load_requests requests (con timeouts y delays)..."
for i in $(seq 1 $load_requests); do
    RESPONSE=$(curl -s -w "\n%{http_code}" -X GET "$API_URL/health" \
        --max-time 3 --connect-timeout 2 2>/dev/null)
    HTTP_CODE=$(echo "$RESPONSE" | tail -n1)
    
    if [ "$HTTP_CODE" = "200" ]; then
        load_success=$((load_success + 1))
    else
        load_errors=$((load_errors + 1))
    fi
    
    if [ $((i % 20)) -eq 0 ]; then
        echo -n "."
    fi
    
    sleep 0.1
done
echo ""

if [ $load_success -gt $((load_requests * 8 / 10)) ]; then
    load_test_success=1
fi

success_rate=$((load_success * 100 / load_requests))
if [ $load_test_success -eq 1 ]; then
    test_result "Carga extrema: Sistema manejó correctamente ($load_success/$load_requests exitosos, ${success_rate}%)" "PASS"
else
    test_result "Carga extrema: Sistema falló bajo carga ($load_success/$load_requests exitosos, ${success_rate}%, esperado: >=80%)" "FAIL"
fi
echo ""

echo "📊 TEST 7: Ataque de Validación de Cadena"
echo "------------------------------------------"
chain_validation_success=0

CHAIN_RESPONSE=$(curl -s --max-time $TIMEOUT "$API_URL/chain/verify" 2>/dev/null)
if [ -z "$CHAIN_RESPONSE" ] || [ "$CHAIN_RESPONSE" = "null" ]; then
    CHAIN_VALID="false"
    CHAIN_COUNT="0"
else
    CHAIN_VALID=$(echo "$CHAIN_RESPONSE" | jq -r '.data.valid // .data.is_valid // "false"' 2>/dev/null || echo "false")
    CHAIN_COUNT=$(echo "$CHAIN_RESPONSE" | jq -r '.data.block_count // 0' 2>/dev/null || echo "0")
fi

if [ "$CHAIN_VALID" = "true" ]; then
    chain_validation_success=1
fi

if [ $chain_validation_success -eq 1 ]; then
    test_result "Validación de cadena: Cadena es válida ($CHAIN_COUNT bloques)" "PASS"
else
    test_result "Validación de cadena: Cadena es inválida ($CHAIN_COUNT bloques)" "FAIL"
    echo "   💡 Sugerencia: Ejecuta './scripts/reset_database.sh' para limpiar la base de datos"
fi
echo ""

echo "====================================="
echo "📊 RESUMEN DE PRUEBAS DE SEGURIDAD"
echo "====================================="
echo "Total de pruebas: $test_count"
echo "✅ Exitosas: $passed_count"
echo "❌ Fallidas: $failed_count"
echo ""

if [ $failed_count -eq 0 ]; then
    echo "🎉 TODAS LAS PRUEBAS DE SEGURIDAD PASARON"
    exit 0
else
    echo "⚠️  ALGUNAS PRUEBAS DE SEGURIDAD FALLARON"
    exit 1
fi

