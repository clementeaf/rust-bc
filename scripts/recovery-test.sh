#!/usr/bin/env bash
# Fault recovery test suite for rust-bc.
# Requires: docker compose running (all nodes healthy).
#
# Tests:
#   1. Peer crash & resync
#   2. Raft orderer crash (1-of-3 tolerance)
#   3. State persistence across restart
#
# Usage: ./scripts/recovery-test.sh

set -uo pipefail

# ── Config ───────────────────────────────────────────────────────────────────
NODE1="https://127.0.0.1:8080"
NODE2="https://127.0.0.1:8082"
NODE3="https://127.0.0.1:8084"
ORDERER1="https://127.0.0.1:8086"
ORDERER2="https://127.0.0.1:8088"
ORDERER3="https://127.0.0.1:8090"
CURL="curl -sk --max-time 30"

PASSED=0
FAILED=0

# ── Helpers ──────────────────────────────────────────────────────────────────
red()    { printf "\033[31m%s\033[0m" "$1"; }
green()  { printf "\033[32m%s\033[0m" "$1"; }
bold()   { printf "\033[1m%s\033[0m" "$1"; }
log()    { echo "  $*"; }

assert_eq() {
    local actual="$1" expected="$2" label="$3"
    if [[ "$actual" == "$expected" ]]; then
        echo "  $(green "PASS") $label"
        PASSED=$((PASSED + 1))
    else
        echo "  $(red "FAIL") $label (expected '$expected', got '$actual')"
        FAILED=$((FAILED + 1))
    fi
}

assert_gt() {
    local actual="$1" min="$2" label="$3"
    if [[ "$actual" -gt "$min" ]] 2>/dev/null; then
        echo "  $(green "PASS") $label"
        PASSED=$((PASSED + 1))
    else
        echo "  $(red "FAIL") $label (expected > $min, got '$actual')"
        FAILED=$((FAILED + 1))
    fi
}

assert_not_empty() {
    local actual="$1" label="$2"
    if [[ -n "$actual" && "$actual" != "null" ]]; then
        echo "  $(green "PASS") $label"
        PASSED=$((PASSED + 1))
    else
        echo "  $(red "FAIL") $label (empty response)"
        FAILED=$((FAILED + 1))
    fi
}

wait_healthy() {
    local container="$1" max_wait="${2:-60}"
    local elapsed=0
    while [[ $elapsed -lt $max_wait ]]; do
        local status
        status=$(docker inspect --format='{{.State.Health.Status}}' "$container" 2>/dev/null || echo "missing")
        if [[ "$status" == "healthy" ]]; then
            return 0
        fi
        sleep 2
        elapsed=$((elapsed + 2))
    done
    return 1
}

get_block_count() {
    local url="$1"
    $CURL "$url/api/v1/stats" 2>/dev/null | jq -r '.data.blockchain.block_count // 0'
}

get_latest_hash() {
    local url="$1"
    $CURL "$url/api/v1/stats" 2>/dev/null | jq -r '.data.blockchain.latest_block_hash // empty'
}

# ── Pre-flight ───────────────────────────────────────────────────────────────
echo ""
bold "═══ rust-bc Recovery Test Suite ═══"
echo ""

bold "0. Pre-flight checks"
for name_port in "node1:8080" "node2:8082" "node3:8084" "orderer1:8086" "orderer2:8088" "orderer3:8090"; do
    name="${name_port%%:*}"
    port="${name_port##*:}"
    status=$($CURL "https://127.0.0.1:$port/api/v1/health" 2>/dev/null | jq -r '.data.status // empty')
    assert_eq "$status" "healthy" "$name is healthy"
done

# Create wallet and mine some blocks to have chain state
WALLET=$($CURL -X POST "$NODE1/api/v1/wallets/create" -H 'Content-Type: application/json' -d '{}' 2>/dev/null | jq -r '.data.address')
log "Test wallet: $WALLET"
for i in $(seq 1 5); do
    $CURL -X POST "$NODE1/api/v1/mine" -H 'Content-Type: application/json' \
        -d "{\"miner_address\":\"$WALLET\"}" > /dev/null 2>&1
done
sleep 3  # Wait for gossip propagation

BASELINE_COUNT=$(get_block_count "$NODE1")
BASELINE_HASH=$(get_latest_hash "$NODE1")
log "Baseline: $BASELINE_COUNT blocks, hash ${BASELINE_HASH:0:16}..."

# Verify all nodes synced
n2_hash=$(get_latest_hash "$NODE2")
n3_hash=$(get_latest_hash "$NODE3")
assert_eq "$n2_hash" "$BASELINE_HASH" "node2 synced to baseline"
assert_eq "$n3_hash" "$BASELINE_HASH" "node3 synced to baseline"

# ═══════════════════════════════════════════════════════════════════════════════
echo ""
bold "1. Peer crash & resync"
echo ""

log "Stopping node2..."
docker compose stop node2 2>&1 | grep -v "level=warning"

# Verify node2 is down
n2_status=$($CURL "$NODE2/api/v1/health" 2>/dev/null | jq -r '.data.status // empty')
assert_eq "$n2_status" "" "node2 is down (no response)"

# Mine more blocks while node2 is down
log "Mining 3 blocks while node2 is down..."
for i in 1 2 3; do
    $CURL -X POST "$NODE1/api/v1/mine" -H 'Content-Type: application/json' \
        -d "{\"miner_address\":\"$WALLET\"}" > /dev/null 2>&1
done
sleep 2

NEW_COUNT=$(get_block_count "$NODE1")
NEW_HASH=$(get_latest_hash "$NODE1")
assert_gt "$NEW_COUNT" "$BASELINE_COUNT" "node1 advanced to $NEW_COUNT blocks (was $BASELINE_COUNT)"

# Verify node3 still works (not affected by node2 crash)
n3_count=$(get_block_count "$NODE3")
n3_hash=$(get_latest_hash "$NODE3")
assert_eq "$n3_hash" "$NEW_HASH" "node3 still synced (not affected by node2 crash)"

# Restart node2
log "Restarting node2..."
docker compose start node2 2>&1 | grep -v "level=warning"

if wait_healthy "rust-bc-node2" 60; then
    log "node2 healthy after restart"
else
    log "WARNING: node2 not healthy after 60s, continuing anyway"
fi
sleep 10  # Extra time for state sync

# Verify node2 resynced
n2_count=$(get_block_count "$NODE2")
n2_hash=$(get_latest_hash "$NODE2")
assert_eq "$n2_hash" "$NEW_HASH" "node2 resynced to latest hash after restart"
assert_eq "$n2_count" "$NEW_COUNT" "node2 has correct block count ($n2_count)"

# ═══════════════════════════════════════════════════════════════════════════════
echo ""
bold "2. Raft orderer crash (1-of-3 fault tolerance)"
echo ""

# Check all orderers healthy
o1_health=$($CURL "$ORDERER1/api/v1/health" 2>/dev/null | jq -r '.data.status // empty')
o2_health=$($CURL "$ORDERER2/api/v1/health" 2>/dev/null | jq -r '.data.status // empty')
o3_health=$($CURL "$ORDERER3/api/v1/health" 2>/dev/null | jq -r '.data.status // empty')
assert_eq "$o1_health" "healthy" "orderer1 healthy before crash"
assert_eq "$o2_health" "healthy" "orderer2 healthy before crash"
assert_eq "$o3_health" "healthy" "orderer3 healthy before crash"

# Kill orderer3
log "Stopping orderer3..."
docker compose stop orderer3 2>&1 | grep -v "level=warning"

# Verify orderer3 is down
o3_down=$($CURL "$ORDERER3/api/v1/health" 2>/dev/null | jq -r '.data.status // empty')
assert_eq "$o3_down" "" "orderer3 is down"

# Verify remaining orderers still respond
o1_after=$($CURL "$ORDERER1/api/v1/health" 2>/dev/null | jq -r '.data.status // empty')
o2_after=$($CURL "$ORDERER2/api/v1/health" 2>/dev/null | jq -r '.data.status // empty')
assert_eq "$o1_after" "healthy" "orderer1 still healthy (cluster survives)"
assert_eq "$o2_after" "healthy" "orderer2 still healthy (cluster survives)"

# Mine blocks — ordering should still work with 2/3 orderers
log "Mining with 1 orderer down..."
PRE_RAFT_COUNT=$(get_block_count "$NODE1")
$CURL -X POST "$NODE1/api/v1/mine" -H 'Content-Type: application/json' \
    -d "{\"miner_address\":\"$WALLET\"}" > /dev/null 2>&1
sleep 2

POST_RAFT_COUNT=$(get_block_count "$NODE1")
assert_gt "$POST_RAFT_COUNT" "$PRE_RAFT_COUNT" "Mining works with orderer down ($PRE_RAFT_COUNT -> $POST_RAFT_COUNT)"

# Restart orderer3
log "Restarting orderer3..."
docker compose start orderer3 2>&1 | grep -v "level=warning"

if wait_healthy "rust-bc-orderer3" 60; then
    log "orderer3 healthy after restart"
else
    log "WARNING: orderer3 not healthy after 60s"
fi
sleep 5

o3_recovered=$($CURL "$ORDERER3/api/v1/health" 2>/dev/null | jq -r '.data.status // empty')
assert_eq "$o3_recovered" "healthy" "orderer3 recovered after restart"

# ═══════════════════════════════════════════════════════════════════════════════
echo ""
bold "3. State persistence across restart (RocksDB)"
echo ""

# Write unique data before crash
MARKER_TX_ID="recovery-test-$(date +%s)"
log "Writing marker transaction: $MARKER_TX_ID"
$CURL -X POST "$NODE1/api/v1/store/transactions" -H 'Content-Type: application/json' \
    -d "{
        \"id\": \"$MARKER_TX_ID\",
        \"block_height\": 999,
        \"timestamp\": $(date +%s),
        \"input_did\": \"did:bc:recovery-test\",
        \"output_recipient\": \"did:bc:persistence\",
        \"amount\": 777,
        \"state\": \"committed\"
    }" > /dev/null 2>&1

# Verify marker is readable
marker_read=$($CURL "$NODE1/api/v1/store/transactions/$MARKER_TX_ID" 2>/dev/null \
    | jq -r '.data.amount // empty')
assert_eq "$marker_read" "777" "Marker transaction written (amount=777)"

# Record chain state before crash
PRE_CRASH_COUNT=$(get_block_count "$NODE1")
PRE_CRASH_HASH=$(get_latest_hash "$NODE1")
log "Pre-crash state: $PRE_CRASH_COUNT blocks, hash ${PRE_CRASH_HASH:0:16}..."

# Hard kill node1 (simulates crash — no graceful shutdown)
log "Hard-killing node1 (docker kill)..."
docker kill rust-bc-node1 > /dev/null 2>&1

sleep 2
n1_down=$($CURL "$NODE1/api/v1/health" 2>/dev/null | jq -r '.data.status // empty')
assert_eq "$n1_down" "" "node1 is down after kill"

# Restart node1
log "Restarting node1..."
docker compose start node1 2>&1 | grep -v "level=warning"

if wait_healthy "rust-bc-node1" 90; then
    log "node1 healthy after hard restart"
else
    log "WARNING: node1 not healthy after 90s"
fi
sleep 5

# Verify chain state persisted
POST_CRASH_COUNT=$(get_block_count "$NODE1")
POST_CRASH_HASH=$(get_latest_hash "$NODE1")
assert_eq "$POST_CRASH_HASH" "$PRE_CRASH_HASH" "Chain hash survived crash"
assert_eq "$POST_CRASH_COUNT" "$PRE_CRASH_COUNT" "Block count survived crash ($POST_CRASH_COUNT)"

# Verify marker transaction survived
marker_after=$($CURL "$NODE1/api/v1/store/transactions/$MARKER_TX_ID" 2>/dev/null \
    | jq -r '.data.amount // empty')
assert_eq "$marker_after" "777" "Marker transaction survived crash (RocksDB persisted)"

# Verify node1 can still mine after recovery
log "Mining after recovery..."
$CURL -X POST "$NODE1/api/v1/mine" -H 'Content-Type: application/json' \
    -d "{\"miner_address\":\"$WALLET\"}" > /dev/null 2>&1
sleep 3

FINAL_COUNT=$(get_block_count "$NODE1")
assert_gt "$FINAL_COUNT" "$POST_CRASH_COUNT" "node1 can mine after recovery ($POST_CRASH_COUNT -> $FINAL_COUNT)"

# Verify other nodes synced the new block
sleep 3
final_n2=$(get_latest_hash "$NODE2")
final_n1=$(get_latest_hash "$NODE1")
assert_eq "$final_n2" "$final_n1" "node2 synced with recovered node1"

# ═══════════════════════════════════════════════════════════════════════════════
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  $(green "PASSED"): $PASSED"
echo "  $(red "FAILED"): $FAILED"
echo "  Total:   $((PASSED + FAILED))"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

if [[ $FAILED -gt 0 ]]; then
    exit 1
fi
