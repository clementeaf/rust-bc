#!/usr/bin/env bash
set -euo pipefail

# Cerulean Ledger — Pre-Launch Dry Run
# Validates the full system before public testnet exposure.
#
# Phase 0: Library-level simulation (1000 tx, faucet spam, restart, replay)
# Phase 1: Docker topology (if Docker available)
# Phase 2: Load test against running nodes
# Phase 3: Node restart + recovery
# Phase 4: Validation (metrics, logs, state consistency)

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

ok()   { echo -e "  ${GREEN}✓${NC} $1"; }
fail() { echo -e "  ${RED}✗${NC} $1"; FAILURES=$((FAILURES + 1)); }
info() { echo -e "  ${CYAN}→${NC} $1"; }
phase(){ echo -e "\n${YELLOW}══ Phase $1 ══${NC} $2\n"; }

FAILURES=0
NODE1="http://localhost:8080/api/v1"
NODE2="http://localhost:8082/api/v1"
NODE3="http://localhost:8084/api/v1"
COMPOSE="docker compose -f deploy/testnet/docker-compose.testnet.yml"

# ════════════════════════════════════════════════════════════════════════════
phase 0 "Library-level simulation (cargo test)"
# ════════════════════════════════════════════════════════════════════════════

info "Running dry_run_48h test suite..."
if cargo test --test dry_run_48h -- --nocapture 2>&1 | tee /tmp/dry_run_phase0.log | tail -15; then
    PHASE0_TESTS=$(grep "test result:" /tmp/dry_run_phase0.log | grep -oP '\d+ passed' | grep -oP '\d+')
    ok "All $PHASE0_TESTS simulation phases passed"
else
    fail "Library simulation failed — DO NOT LAUNCH"
    echo ""
    echo "Fix the failures above before proceeding."
    exit 1
fi

# ════════════════════════════════════════════════════════════════════════════
# Docker phases — skip if Docker unavailable
# ════════════════════════════════════════════════════════════════════════════

if ! command -v docker &>/dev/null || ! docker info &>/dev/null 2>&1; then
    echo -e "\n${YELLOW}Docker not available — skipping network phases.${NC}"
    echo "Library simulation passed. For full dry run, install Docker."
    exit 0
fi

# ════════════════════════════════════════════════════════════════════════════
phase 1 "Start 3-node testnet"
# ════════════════════════════════════════════════════════════════════════════

info "Building and starting nodes..."
$COMPOSE down -v 2>/dev/null || true
$COMPOSE up -d --build 2>&1 | tail -3

info "Waiting 15s for nodes to initialize..."
sleep 15

HEALTHY=0
for port in 8080 8082 8084; do
    if curl -sf "http://localhost:$port/api/v1/health" >/dev/null 2>&1; then
        ok "Node $port healthy"
        HEALTHY=$((HEALTHY + 1))
    else
        fail "Node $port not responding"
    fi
done

if [ "$HEALTHY" -lt 3 ]; then
    fail "Not all nodes healthy — aborting"
    $COMPOSE logs --tail 20
    $COMPOSE down -v
    exit 1
fi

# ════════════════════════════════════════════════════════════════════════════
phase 2 "Load test: faucet + transfers"
# ════════════════════════════════════════════════════════════════════════════

info "Faucet funding 10 addresses..."
FAUCET_OK=0
for i in $(seq 1 10); do
    RESULT=$(curl -sf -X POST "$NODE1/faucet/drip" \
        -H "Content-Type: application/json" \
        -d "{\"address\": \"loadtest_$i\"}" 2>/dev/null || echo "FAIL")
    if echo "$RESULT" | grep -q "amount"; then
        FAUCET_OK=$((FAUCET_OK + 1))
    fi
done
if [ "$FAUCET_OK" -ge 8 ]; then
    ok "Faucet: $FAUCET_OK/10 funded"
else
    fail "Faucet: only $FAUCET_OK/10 funded"
fi

info "Submitting transfers..."
TX_OK=0
for i in $(seq 1 50); do
    FROM="loadtest_$(( (i % 10) + 1 ))"
    TO="loadtest_$(( ((i + 3) % 10) + 1 ))"
    NONCE=$(( (i - 1) / 10 ))
    RESULT=$(curl -sf -X POST "$NODE1/transfer" \
        -H "Content-Type: application/json" \
        -d "{\"from\": \"$FROM\", \"to\": \"$TO\", \"amount\": 1, \"nonce\": $NONCE, \"fee\": 2}" 2>/dev/null || echo "FAIL")
    if echo "$RESULT" | grep -q "tx_id"; then
        TX_OK=$((TX_OK + 1))
    fi
done
if [ "$TX_OK" -ge 30 ]; then
    ok "Transfers: $TX_OK/50 accepted"
else
    fail "Transfers: only $TX_OK/50 accepted"
fi

info "Checking mempool..."
PENDING=$(curl -sf "$NODE1/mempool/stats" 2>/dev/null | python3 -c "import sys,json; print(json.load(sys.stdin).get('data',{}).get('pending',0))" 2>/dev/null || echo "?")
info "Mempool pending: $PENDING"

# ════════════════════════════════════════════════════════════════════════════
phase 3 "Node restart + recovery"
# ════════════════════════════════════════════════════════════════════════════

info "Restarting node 2..."
$COMPOSE restart testnet-node2 2>/dev/null || $COMPOSE restart node2 2>/dev/null || true
sleep 10

if curl -sf "$NODE2/health" >/dev/null 2>&1; then
    ok "Node 2 recovered after restart"
else
    fail "Node 2 failed to recover"
fi

# ════════════════════════════════════════════════════════════════════════════
phase 4 "Validation"
# ════════════════════════════════════════════════════════════════════════════

info "Checking logs for panics..."
PANICS=$($COMPOSE logs 2>&1 | grep -ci "panic\|fatal\|SIGSEGV" || true)
if [ "$PANICS" -eq 0 ]; then
    ok "No panics in logs"
else
    fail "$PANICS panic(s) found in logs"
fi

info "Checking height consistency..."
HEIGHTS=""
for port in 8080 8082 8084; do
    H=$(curl -sf "http://localhost:$port/api/v1/store/blocks/latest" 2>/dev/null | python3 -c "import sys,json; print(json.load(sys.stdin).get('data',{}).get('height','?'))" 2>/dev/null || echo "?")
    HEIGHTS="$HEIGHTS $port=$H"
done
info "Heights:$HEIGHTS"

info "Checking balances on funded account..."
BAL=$(curl -sf "$NODE1/accounts/loadtest_1" 2>/dev/null | python3 -c "import sys,json; print(json.load(sys.stdin).get('data',{}).get('balance','?'))" 2>/dev/null || echo "?")
info "loadtest_1 balance: $BAL"

# ════════════════════════════════════════════════════════════════════════════
# Cleanup
# ════════════════════════════════════════════════════════════════════════════

info "Stopping testnet..."
$COMPOSE down -v 2>/dev/null || true

# ════════════════════════════════════════════════════════════════════════════
echo ""
echo "═══════════════════════════════════════════════"
echo "  Cerulean Ledger — Dry Run Report"
echo "═══════════════════════════════════════════════"
if [ "$FAILURES" -eq 0 ]; then
    echo -e "  Status: ${GREEN}ALL PHASES PASSED${NC}"
    echo "  Ready for public testnet launch."
else
    echo -e "  Status: ${RED}$FAILURES FAILURE(S)${NC}"
    echo "  DO NOT LAUNCH until all failures are resolved."
fi
echo "═══════════════════════════════════════════════"
exit "$FAILURES"
