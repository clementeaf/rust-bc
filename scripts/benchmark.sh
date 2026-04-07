#!/usr/bin/env bash
# benchmark.sh — Run performance benchmarks against a live rust-bc network.
#
# Usage:
#   ./scripts/benchmark.sh [node_url]
#
# Requirements:
#   - Running rust-bc network (docker compose up -d)
#   - curl, jq, bc
#
# Runs:
#   1. Criterion micro-benchmarks (ordering, endorsement, event bus, RocksDB)
#   2. Gateway latency (single request)
#   3. Sequential throughput (N transactions via gateway)
#   4. Block propagation time

set -euo pipefail

NODE="${1:-https://localhost:8080}"
CURL="curl -sk --max-time 30"
N=50

echo "══════════════════════════════════════════════════════"
echo "  rust-bc Performance Benchmarks"
echo "  Target: $NODE"
echo "══════════════════════════════════════════════════════"
echo ""

# ── 1. Criterion micro-benchmarks ────────────────────────────────────────────

echo "▶ 1. Criterion micro-benchmarks"
echo "   Running: cargo bench (this takes ~60 seconds)..."
cargo bench --bench ordering_throughput 2>&1 | grep -E "time:|thrpt:" | head -20
echo ""

# ── 2. Gateway latency (single request) ─────────────────────────────────────

echo "▶ 2. Gateway submit latency (single request)"

# Ensure node is healthy
status=$($CURL "$NODE/api/v1/health" | jq -r '.data.status // "down"' 2>/dev/null)
if [[ "$status" != "healthy" && "$status" != "degraded" ]]; then
    echo "   ERROR: Node not healthy ($status). Start the network first."
    exit 1
fi

# Measure single gateway submit
start_ms=$(python3 -c 'import time; print(int(time.time()*1000))')
resp=$($CURL "$NODE/api/v1/gateway/submit" -X POST \
    -H 'Content-Type: application/json' \
    -d "{
        \"chaincode_id\": \"bench-cc\",
        \"channel_id\": \"\",
        \"transaction\": {
            \"id\": \"bench-latency-$(date +%s%N)\",
            \"input_did\": \"did:bc:bench\",
            \"output_recipient\": \"did:bc:target\",
            \"amount\": 1
        }
    }" 2>&1)
end_ms=$(python3 -c 'import time; print(int(time.time()*1000))')
latency=$((end_ms - start_ms))
echo "   Single gateway submit: ${latency}ms"
echo ""

# ── 3. Sequential throughput ─────────────────────────────────────────────────

echo "▶ 3. Sequential throughput ($N transactions)"

start_ms=$(python3 -c 'import time; print(int(time.time()*1000))')
success=0
for i in $(seq 1 $N); do
    resp=$($CURL "$NODE/api/v1/gateway/submit" -X POST \
        -H 'Content-Type: application/json' \
        -d "{
            \"chaincode_id\": \"bench-cc\",
            \"channel_id\": \"\",
            \"transaction\": {
                \"id\": \"bench-tx-$i-$(date +%s%N)\",
                \"input_did\": \"did:bc:bench\",
                \"output_recipient\": \"did:bc:target\",
                \"amount\": 1
            }
        }" 2>&1)
    code=$(echo "$resp" | jq -r '.status_code // 0' 2>/dev/null)
    if [[ "$code" == "200" ]]; then
        success=$((success + 1))
    fi
done
end_ms=$(python3 -c 'import time; print(int(time.time()*1000))')
total_ms=$((end_ms - start_ms))
tps=$(echo "scale=1; $success * 1000 / $total_ms" | bc 2>/dev/null || echo "?")

echo "   Submitted: $N, Succeeded: $success"
echo "   Total time: ${total_ms}ms"
echo "   Throughput: ${tps} TPS"
echo ""

# ── 4. Block propagation ────────────────────────────────────────────────────

echo "▶ 4. Block propagation (node1 → node2)"

# Get current height on node1 and node2
h1=$($CURL "https://localhost:8080/api/v1/chain/info" | jq -r '.data.block_count // 0' 2>/dev/null)
h2=$($CURL "https://localhost:8082/api/v1/chain/info" | jq -r '.data.block_count // 0' 2>/dev/null)

if [[ "$h2" == "0" ]] || [[ "$h2" == "" ]]; then
    echo "   SKIP: node2 not reachable (single-node setup?)"
else
    # Mine a block on node1
    wallet=$($CURL "$NODE/api/v1/wallets/create" -X POST | jq -r '.data.address // "bench"' 2>/dev/null)
    start_ms=$(python3 -c 'import time; print(int(time.time()*1000))')
    $CURL "$NODE/api/v1/mine" -X POST \
        -H 'Content-Type: application/json' \
        -d "{\"miner_address\": \"$wallet\"}" > /dev/null 2>&1

    # Wait for propagation to node2
    new_h1=$($CURL "https://localhost:8080/api/v1/chain/info" | jq -r '.data.block_count // 0' 2>/dev/null)
    for i in $(seq 1 20); do
        new_h2=$($CURL "https://localhost:8082/api/v1/chain/info" | jq -r '.data.block_count // 0' 2>/dev/null)
        if [[ "$new_h2" -ge "$new_h1" ]]; then
            end_ms=$(python3 -c 'import time; print(int(time.time()*1000))')
            prop_ms=$((end_ms - start_ms))
            echo "   Propagation time: ${prop_ms}ms (node1 height=$new_h1, node2 height=$new_h2)"
            break
        fi
        sleep 0.5
    done
fi
echo ""

# ── 5. Health check latency ─────────────────────────────────────────────────

echo "▶ 5. Health check latency (10 requests)"
total=0
for i in $(seq 1 10); do
    s=$(python3 -c 'import time; print(int(time.time()*1000))')
    $CURL "$NODE/api/v1/health" > /dev/null 2>&1
    e=$(python3 -c 'import time; print(int(time.time()*1000))')
    total=$((total + e - s))
done
avg=$((total / 10))
echo "   Average: ${avg}ms"
echo ""

echo "══════════════════════════════════════════════════════"
echo "  Benchmark complete"
echo "══════════════════════════════════════════════════════"
