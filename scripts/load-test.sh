#!/usr/bin/env bash
# Sustained load test for the rust-bc blockchain network.
#
# Sends transactions at a configurable rate for a configurable duration,
# then reports throughput, latency percentiles, and error rate.
#
# Requires: curl, jq, bc
# Requires: running Docker network (docker compose up -d)
#
# Usage:
#   ./scripts/load-test.sh                    # defaults: 60s, 100 tx/s
#   ./scripts/load-test.sh --duration 300     # 5 minutes
#   ./scripts/load-test.sh --rate 500         # 500 tx/s target
#   ./scripts/load-test.sh --duration 3600 --rate 1000  # 1 hour at 1K tx/s

set -uo pipefail

# ── Config ───────────────────────────────────────────────────────────────────

NODE="https://127.0.0.1:8080"
CURL="curl -sk --max-time 5"
DURATION=60
RATE=100

while [[ $# -gt 0 ]]; do
    case "$1" in
        --duration) DURATION="$2"; shift 2 ;;
        --rate)     RATE="$2"; shift 2 ;;
        --node)     NODE="$2"; shift 2 ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

INTERVAL=$(echo "scale=6; 1.0 / $RATE" | bc)
TOTAL_TX=$((DURATION * RATE))

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Load Test Configuration"
echo "  Node:     $NODE"
echo "  Duration: ${DURATION}s"
echo "  Rate:     ${RATE} tx/s"
echo "  Total:    ~${TOTAL_TX} transactions"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# ── Pre-flight ───────────────────────────────────────────────────────────────

echo "[pre-flight] Checking node health..."
health=$($CURL "$NODE/api/v1/health" 2>/dev/null)
if ! echo "$health" | jq -e '.data.status == "healthy"' >/dev/null 2>&1; then
    echo "ERROR: Node is not healthy. Start the network first."
    echo "  docker compose up -d"
    exit 1
fi
echo "[pre-flight] Node is healthy."

# Create a wallet for the test
echo "[pre-flight] Creating test wallet..."
wallet=$($CURL -X POST "$NODE/api/v1/wallets" -H "Content-Type: application/json" 2>/dev/null)
WALLET_ADDR=$(echo "$wallet" | jq -r '.data.address // empty' 2>/dev/null)
if [[ -z "$WALLET_ADDR" ]]; then
    echo "WARNING: Could not create wallet. Proceeding with raw transactions."
    WALLET_ADDR="load-test-wallet"
fi
echo "[pre-flight] Wallet: ${WALLET_ADDR:0:16}..."

# Record starting block height
START_HEIGHT=$($CURL "$NODE/api/v1/store/blocks/latest" 2>/dev/null | jq -r '.data // 0')
echo "[pre-flight] Starting block height: $START_HEIGHT"
echo ""

# ── Load generation ──────────────────────────────────────────────────────────

LATENCY_FILE=$(mktemp)
ERROR_COUNT=0
SUCCESS_COUNT=0
START_TIME=$(date +%s)

echo "[load] Sending transactions for ${DURATION}s at ${RATE} tx/s..."
echo ""

tx_count=0
end_time=$((START_TIME + DURATION))

while [[ $(date +%s) -lt $end_time ]]; do
    batch_start=$(date +%s%N)

    # Send a batch of transactions (10 concurrent via background subshells)
    for i in $(seq 1 10); do
        tx_count=$((tx_count + 1))
        tx_id="load-test-tx-${tx_count}"
        ts_now=$(date +%s)

        (
            tx_start=$(date +%s%N)
            # Retry once on 429 (rate limited)
            for attempt in 1 2; do
                http_code=$(curl -sk --max-time 10 -o /dev/null -w "%{http_code}" \
                    -X POST "$NODE/api/v1/store/transactions" \
                    -H "Content-Type: application/json" \
                    -d "{\"id\":\"${tx_id}\",\"block_height\":0,\"timestamp\":${ts_now},\"input_did\":\"did:bc:${WALLET_ADDR}\",\"output_recipient\":\"did:bc:recipient\",\"amount\":1,\"state\":\"pending\"}")
                if [[ "$http_code" != "429" ]]; then break; fi
                sleep 0.1
            done
            tx_end=$(date +%s%N)
            latency_ns=$((tx_end - tx_start))
            latency_ms=$(echo "scale=2; $latency_ns / 1000000" | bc)

            if [[ "$http_code" == "201" || "$http_code" == "200" ]]; then
                echo "OK $latency_ms" >> "$LATENCY_FILE"
            elif [[ "$http_code" == "429" ]]; then
                echo "THROTTLED $latency_ms" >> "$LATENCY_FILE"
            else
                echo "ERR $http_code" >> "$LATENCY_FILE"
            fi
        ) &
    done
    wait

    # Progress every 10 seconds
    elapsed=$(($(date +%s) - START_TIME))
    if [[ $((elapsed % 10)) -eq 0 && $elapsed -gt 0 ]]; then
        actual_rate=$((tx_count / elapsed))
        printf "\r  [%3ds/%ds] sent: %d  ok: %d  err: %d  rate: %d tx/s" \
            "$elapsed" "$DURATION" "$tx_count" "$SUCCESS_COUNT" "$ERROR_COUNT" "$actual_rate"
    fi

    # Sleep to approximate target rate (10 tx per batch)
    target_batch_ns=$(echo "$INTERVAL * 10 * 1000000000" | bc | cut -d. -f1)
    batch_elapsed=$(($(date +%s%N) - batch_start))
    sleep_ns=$((target_batch_ns - batch_elapsed))
    if [[ $sleep_ns -gt 0 ]]; then
        sleep "$(echo "scale=6; $sleep_ns / 1000000000" | bc)"
    fi
done

END_TIME=$(date +%s)
ACTUAL_DURATION=$((END_TIME - START_TIME))
echo ""
echo ""

# ── Results ──────────────────────────────────────────────────────────────────

END_HEIGHT=$($CURL "$NODE/api/v1/store/blocks/latest" 2>/dev/null | jq -r '.data // 0')
BLOCKS_CREATED=$((END_HEIGHT - START_HEIGHT))

# Parse results from latency file (lines: "OK 12.34", "THROTTLED 12.34", or "ERR 415")
SUCCESS_COUNT=$(grep -c "^OK " "$LATENCY_FILE" 2>/dev/null | tr -d '[:space:]' || true)
THROTTLED_COUNT=$(grep -c "^THROTTLED " "$LATENCY_FILE" 2>/dev/null | tr -d '[:space:]' || true)
ERROR_COUNT=$(grep -c "^ERR " "$LATENCY_FILE" 2>/dev/null | tr -d '[:space:]' || true)
SUCCESS_COUNT=${SUCCESS_COUNT:-0}
THROTTLED_COUNT=${THROTTLED_COUNT:-0}
ERROR_COUNT=${ERROR_COUNT:-0}

# Extract latency values from successful requests
LATENCY_ONLY=$(mktemp)
grep "^OK " "$LATENCY_FILE" 2>/dev/null | awk '{print $2}' > "$LATENCY_ONLY"

if [[ -s "$LATENCY_ONLY" ]]; then
    TOTAL_SAMPLES=$(wc -l < "$LATENCY_ONLY" | tr -d '[:space:]')
    P50=$(sort -n "$LATENCY_ONLY" | awk "NR==int($TOTAL_SAMPLES*0.50)")
    P95=$(sort -n "$LATENCY_ONLY" | awk "NR==int($TOTAL_SAMPLES*0.95)")
    P99=$(sort -n "$LATENCY_ONLY" | awk "NR==int($TOTAL_SAMPLES*0.99)")
    MIN=$(sort -n "$LATENCY_ONLY" | head -1)
    MAX=$(sort -n "$LATENCY_ONLY" | tail -1)
    AVG=$(awk '{sum+=$1} END {printf "%.2f", sum/NR}' "$LATENCY_ONLY")
else
    P50="N/A"; P95="N/A"; P99="N/A"; MIN="N/A"; MAX="N/A"; AVG="N/A"
fi
rm -f "$LATENCY_ONLY"

TOTAL_SENT=$((SUCCESS_COUNT + THROTTLED_COUNT + ERROR_COUNT))
ACTUAL_TPS=$((SUCCESS_COUNT / (ACTUAL_DURATION > 0 ? ACTUAL_DURATION : 1)))
if [[ $TOTAL_SENT -gt 0 ]]; then
    ERROR_RATE=$(echo "scale=2; $ERROR_COUNT * 100 / $TOTAL_SENT" | bc)
else
    ERROR_RATE="0"
fi

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Load Test Results"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "  Duration:        ${ACTUAL_DURATION}s"
echo "  Transactions:    ${SUCCESS_COUNT} ok / ${THROTTLED_COUNT} throttled / ${ERROR_COUNT} errors"
echo "  Throughput:      ${ACTUAL_TPS} tx/s (actual)"
echo "  Error rate:      ${ERROR_RATE}%"
echo "  Blocks created:  ${BLOCKS_CREATED} (height ${START_HEIGHT} → ${END_HEIGHT})"
echo ""
echo "  Latency (ms):"
echo "    min:  ${MIN}"
echo "    avg:  ${AVG}"
echo "    p50:  ${P50}"
echo "    p95:  ${P95}"
echo "    p99:  ${P99}"
echo "    max:  ${MAX}"
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# ── Cleanup ──────────────────────────────────────────────────────────────────
rm -f "$LATENCY_FILE"

if [[ $ERROR_COUNT -gt $((SUCCESS_COUNT / 10)) ]]; then
    echo ""
    echo "WARNING: Error rate > 10%. Review node logs:"
    echo "  docker compose logs node1 --tail 50"
    exit 1
fi
