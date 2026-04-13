#!/usr/bin/env bash
# Stress test — ramp load until the node breaks.
# Finds the throughput ceiling, error threshold, and failure mode.
#
# Usage: ./scripts/stress-test.sh

set -uo pipefail

NODE="https://127.0.0.1:8080"
CURL="curl -sk --max-time 10"

red()    { printf "\033[31m%s\033[0m" "$1"; }
green()  { printf "\033[32m%s\033[0m" "$1"; }
yellow() { printf "\033[33m%s\033[0m" "$1"; }
bold()   { printf "\033[1m%s\033[0m" "$1"; }

echo ""
bold "═══ rust-bc Stress Test (ramp to failure) ═══"
echo ""

# ── Pre-flight ───────────────────────────────────────────────────────────────
health=$($CURL "$NODE/api/v1/health" 2>/dev/null | jq -r '.data.status // empty')
if [[ "$health" != "healthy" ]]; then
    echo "$(red "ERROR"): Node not healthy. Start with: docker compose up -d"
    exit 1
fi

WALLET=$($CURL -X POST "$NODE/api/v1/wallets/create" -H 'Content-Type: application/json' -d '{}' 2>/dev/null | jq -r '.data.address')
echo "  Wallet: ${WALLET:0:16}..."
echo ""

# ── Phase 1: Ramp throughput ─────────────────────────────────────────────────
bold "Phase 1: Ramp throughput (find ceiling)"
echo ""

RESULTS_FILE=$(mktemp)
SEQ=0

for CONCURRENCY in 1 5 10 20 50 100 200; do
    # Send $CONCURRENCY requests in parallel, measure how many succeed in 10s
    ok=0
    err=0
    throttled=0
    latency_sum=0
    latency_count=0
    BATCH_FILE=$(mktemp)

    start_ts=$(date +%s)
    end_ts=$((start_ts + 10))

    while [[ $(date +%s) -lt $end_ts ]]; do
        for i in $(seq 1 "$CONCURRENCY"); do
            SEQ=$((SEQ + 1))
            (
                t_start=$(date +%s%N)
                http_code=$(curl -sk --max-time 10 -o /dev/null -w "%{http_code}" \
                    -X POST "$NODE/api/v1/store/transactions" \
                    -H "Content-Type: application/json" \
                    -d "{\"id\":\"stress-${SEQ}\",\"block_height\":0,\"timestamp\":$(date +%s),\"input_did\":\"did:bc:${WALLET}\",\"output_recipient\":\"did:bc:target\",\"amount\":1,\"state\":\"pending\"}" 2>/dev/null)
                t_end=$(date +%s%N)
                lat_ms=$(( (t_end - t_start) / 1000000 ))
                echo "$http_code $lat_ms" >> "$BATCH_FILE"
            ) &
        done
        wait
    done

    actual_duration=$(($(date +%s) - start_ts))
    [[ $actual_duration -lt 1 ]] && actual_duration=1

    # Parse results
    ok=$(grep -c "^200\|^201" "$BATCH_FILE" 2>/dev/null || true)
    throttled=$(grep -c "^429" "$BATCH_FILE" 2>/dev/null || true)
    err=$(grep -cEv "^200|^201|^429" "$BATCH_FILE" 2>/dev/null || true)
    total=$((ok + throttled + err))
    tps=$((ok / actual_duration))

    # Latency stats (OK requests only)
    avg_lat="N/A"
    p99_lat="N/A"
    if [[ $ok -gt 0 ]]; then
        avg_lat=$(grep "^200\|^201" "$BATCH_FILE" | awk '{sum+=$2; n++} END {if(n>0) printf "%d", sum/n; else print "N/A"}')
        p99_lat=$(grep "^200\|^201" "$BATCH_FILE" | awk '{print $2}' | sort -n | awk "NR==int($(grep -c "^200\|^201" "$BATCH_FILE")*0.99) {print}")
    fi

    # Error rate
    if [[ $total -gt 0 ]]; then
        err_pct=$((err * 100 / total))
        throttle_pct=$((throttled * 100 / total))
    else
        err_pct=0
        throttle_pct=0
    fi

    # Status indicator
    if [[ $err_pct -gt 10 ]]; then
        status="$(red "BREAKING")"
    elif [[ $throttle_pct -gt 50 ]]; then
        status="$(yellow "THROTTLED")"
    elif [[ $err_pct -gt 0 ]]; then
        status="$(yellow "DEGRADED")"
    else
        status="$(green "OK")"
    fi

    printf "  concurrency=%3d  tps=%4d  ok=%5d  throttled=%5d  err=%3d  avg=%4sms  p99=%4sms  %s\n" \
        "$CONCURRENCY" "$tps" "$ok" "$throttled" "$err" "$avg_lat" "$p99_lat" "$status"

    echo "$CONCURRENCY $tps $ok $throttled $err $avg_lat $p99_lat" >> "$RESULTS_FILE"
    rm -f "$BATCH_FILE"

    # Stop if error rate > 20%
    if [[ $err_pct -gt 20 ]]; then
        echo ""
        echo "  $(red "STOPPED"): Error rate ${err_pct}% > 20% at concurrency=$CONCURRENCY"
        break
    fi
done

echo ""

# ── Phase 2: Large payload test ──────────────────────────────────────────────
bold "Phase 2: Large payload handling"
echo ""

for size_kb in 1 10 100 1000 5000; do
    payload=$(python3 -c "
import json, sys
data = 'X' * ($size_kb * 1024)
obj = {'id':'big-$size_kb','block_height':0,'timestamp':1,'input_did':'did:bc:test','output_recipient':'did:bc:target','amount':1,'state':'pending','data':data}
sys.stdout.write(json.dumps(obj))
")
    http_code=$(curl -sk --max-time 30 -o /dev/null -w "%{http_code}" \
        -X POST "$NODE/api/v1/store/transactions" \
        -H "Content-Type: application/json" \
        -d "$payload" 2>/dev/null)

    if [[ "$http_code" == "200" || "$http_code" == "201" ]]; then
        printf "  %5d KB  $(green "ACCEPTED") (%s)\n" "$size_kb" "$http_code"
    elif [[ "$http_code" == "413" ]]; then
        printf "  %5d KB  $(yellow "REJECTED") (413 Payload Too Large)\n" "$size_kb"
    elif [[ "$http_code" == "400" ]]; then
        printf "  %5d KB  $(yellow "REJECTED") (400 Bad Request)\n" "$size_kb"
    else
        printf "  %5d KB  $(red "ERROR") (%s)\n" "$size_kb" "$http_code"
    fi
done

echo ""

# ── Phase 3: Connection exhaustion ───────────────────────────────────────────
bold "Phase 3: Connection exhaustion (500 concurrent connections)"
echo ""

CONN_FILE=$(mktemp)
for i in $(seq 1 500); do
    (
        http_code=$(curl -sk --max-time 5 -o /dev/null -w "%{http_code}" \
            "$NODE/api/v1/health" 2>/dev/null)
        echo "$http_code" >> "$CONN_FILE"
    ) &
    # Batch in groups of 50 to avoid local fd exhaustion
    if (( i % 50 == 0 )); then
        wait
    fi
done
wait

conn_ok=$(grep -c "^200" "$CONN_FILE" 2>/dev/null || true)
conn_err=$(grep -cEv "^200" "$CONN_FILE" 2>/dev/null || true)
conn_total=$((conn_ok + conn_err))
printf "  Connections: %d OK / %d failed (of %d)\n" "$conn_ok" "$conn_err" "$conn_total"
if [[ $conn_err -gt $((conn_total / 4)) ]]; then
    echo "  $(red "WARN"): >25% connection failures under 500 concurrent connections"
else
    echo "  $(green "OK"): Node handles 500 concurrent connections"
fi
rm -f "$CONN_FILE"

echo ""

# ── Phase 4: Malformed input resilience ──────────────────────────────────────
bold "Phase 4: Malformed input resilience"
echo ""

HUGE_ID=$(python3 -c "print('A'*10000)")

test_malformed() {
    local label="$1" payload="$2"
    local http_code
    http_code=$(curl -sk --max-time 10 -o /dev/null -w "%{http_code}" \
        -X POST "$NODE/api/v1/store/transactions" \
        -H "Content-Type: application/json" \
        -d "$payload" 2>/dev/null)

    if [[ "$http_code" == "400" || "$http_code" == "413" || "$http_code" == "415" || "$http_code" == "422" ]]; then
        printf "  %-20s $(green "REJECTED") (%s)\n" "$label" "$http_code"
    elif [[ "$http_code" == "200" || "$http_code" == "201" ]]; then
        printf "  %-20s $(yellow "ACCEPTED") (%s) — should have been rejected\n" "$label" "$http_code"
    elif [[ "$http_code" == "000" || -z "$http_code" ]]; then
        printf "  %-20s $(red "NO RESPONSE") — node may have crashed!\n" "$label"
        return 1
    else
        printf "  %-20s $(yellow "OTHER") (%s)\n" "$label" "$http_code"
    fi
    return 0
}

node_alive=true
test_malformed "empty body"       ""                                                                    || node_alive=false
test_malformed "null JSON"        "null"                                                                || node_alive=false
test_malformed "invalid JSON"     "{not json}"                                                          || node_alive=false
test_malformed "missing fields"   '{"id":"x"}'                                                          || node_alive=false
test_malformed "negative amount"  '{"id":"neg","block_height":0,"timestamp":1,"input_did":"did:bc:a","output_recipient":"did:bc:b","amount":-1,"state":"pending"}'  || node_alive=false
test_malformed "overflow amount"  '{"id":"ovf","block_height":0,"timestamp":1,"input_did":"did:bc:a","output_recipient":"did:bc:b","amount":99999999999999999999,"state":"pending"}'  || node_alive=false
test_malformed "XSS in field"     '{"id":"<script>alert(1)</script>","block_height":0,"timestamp":1,"input_did":"did:bc:a","output_recipient":"did:bc:b","amount":1,"state":"pending"}'  || node_alive=false
test_malformed "SQL injection"    '{"id":"x OR 1=1--","block_height":0,"timestamp":1,"input_did":"did:bc:a","output_recipient":"did:bc:b","amount":1,"state":"pending"}'  || node_alive=false
test_malformed "null bytes"       '{"id":"null\u0000byte","block_height":0,"timestamp":1,"input_did":"did:bc:a","output_recipient":"did:bc:b","amount":1,"state":"pending"}'  || node_alive=false
test_malformed "huge ID"          "{\"id\":\"${HUGE_ID}\",\"block_height\":0,\"timestamp\":1,\"input_did\":\"did:bc:a\",\"output_recipient\":\"did:bc:b\",\"amount\":1,\"state\":\"pending\"}"  || node_alive=false

# Verify node still alive after malformed inputs
if $node_alive; then
    final_health=$($CURL "$NODE/api/v1/health" 2>/dev/null | jq -r '.data.status // empty')
    if [[ "$final_health" == "healthy" ]]; then
        echo ""
        echo "  $(green "OK"): Node survived all malformed inputs"
    else
        echo ""
        echo "  $(red "FAIL"): Node unhealthy after malformed inputs"
    fi
fi

echo ""

# ── Summary ──────────────────────────────────────────────────────────────────
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
bold "  Throughput ramp results:"
echo ""
printf "  %-12s %-8s %-8s %-10s %-6s\n" "Concurrency" "TPS" "OK" "Throttled" "Errors"
while IFS=' ' read -r conc tps ok thr err avg p99; do
    printf "  %-12s %-8s %-8s %-10s %-6s\n" "$conc" "$tps" "$ok" "$thr" "$err"
done < "$RESULTS_FILE"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

rm -f "$RESULTS_FILE"
