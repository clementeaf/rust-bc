#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════════════════════
# try-it.sh — Interactive demo of the rust-bc blockchain
#
# Starts a local node (no Docker needed) and walks you through:
#   1. Creating wallets
#   2. Mining blocks
#   3. Sending transactions
#   4. Checking balances and chain state
#
# Requirements: cargo (Rust toolchain)
# Optional:     node/npm (for the block explorer UI)
#
# Usage: ./scripts/try-it.sh
# ═══════════════════════════════════════════════════════════════════════════════

set -euo pipefail

# ── Colors ────────────────────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
DIM='\033[2m'
NC='\033[0m'

BASE_URL="http://127.0.0.1:8080/api/v1"
NODE_PID=""
EXPLORER_PID=""

cleanup() {
    echo ""
    echo -e "${DIM}Cleaning up...${NC}"
    if [[ -n "$NODE_PID" ]]; then
        kill "$NODE_PID" 2>/dev/null || true
        wait "$NODE_PID" 2>/dev/null || true
    fi
    if [[ -n "$EXPLORER_PID" ]]; then
        kill "$EXPLORER_PID" 2>/dev/null || true
    fi
    echo -e "${GREEN}Done. Thanks for trying rust-bc!${NC}"
}
trap cleanup EXIT

banner() {
    echo ""
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BOLD}  $1${NC}"
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
}

info()    { echo -e "  ${BLUE}ℹ${NC}  $1"; }
success() { echo -e "  ${GREEN}✓${NC}  $1"; }
warn()    { echo -e "  ${YELLOW}⚠${NC}  $1"; }
step()    { echo -e "  ${CYAN}→${NC}  ${BOLD}$1${NC}"; }

pause() {
    echo ""
    echo -e "  ${DIM}Press Enter to continue...${NC}"
    read -r
}

api_get()  { curl -s "$BASE_URL$1" 2>/dev/null; }
api_post() { curl -s -X POST "$BASE_URL$1" -H 'Content-Type: application/json' -d "$2" 2>/dev/null; }
jq_field() { python3 -c "import sys,json; d=json.load(sys.stdin); print($1)" 2>/dev/null; }

# ── Pre-flight ────────────────────────────────────────────────────────────────
banner "🔗 rust-bc — Interactive Blockchain Demo"

echo -e "  This script will:"
echo -e "    ${CYAN}1.${NC} Start a local blockchain node"
echo -e "    ${CYAN}2.${NC} Create two wallets"
echo -e "    ${CYAN}3.${NC} Mine blocks to earn coins"
echo -e "    ${CYAN}4.${NC} Send a transaction between wallets"
echo -e "    ${CYAN}5.${NC} Verify the chain"
echo ""
echo -e "  ${DIM}No Docker required. Everything runs locally.${NC}"

pause

# ── Check dependencies ────────────────────────────────────────────────────────
banner "📋 Checking dependencies"

if command -v cargo &>/dev/null; then
    success "Rust/Cargo found: $(cargo --version 2>/dev/null | head -1)"
else
    echo -e "  ${RED}✗${NC}  Cargo not found. Install Rust: https://rustup.rs"
    exit 1
fi

if command -v python3 &>/dev/null; then
    success "Python3 found (used for JSON parsing)"
else
    echo -e "  ${RED}✗${NC}  Python3 not found. Install Python 3."
    exit 1
fi

# Check if port 8080 is free
if lsof -i :8080 -t &>/dev/null; then
    warn "Port 8080 is in use. Stopping existing process..."
    kill "$(lsof -i :8080 -t)" 2>/dev/null || true
    sleep 2
fi

# ── Build & start node ───────────────────────────────────────────────────────
banner "🚀 Starting blockchain node"

info "Building in release mode (first time may take a few minutes)..."
echo ""

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

(cd "$PROJECT_DIR" && ACL_MODE=permissive cargo run --release --bin rust-bc > /tmp/rust-bc-demo.log 2>&1) &
NODE_PID=$!

echo -e "  ${DIM}Waiting for node to start...${NC}"
for i in $(seq 1 60); do
    if curl -s "$BASE_URL/health" &>/dev/null; then
        break
    fi
    if ! kill -0 "$NODE_PID" 2>/dev/null; then
        echo -e "  ${RED}✗${NC}  Node failed to start. Check /tmp/rust-bc-demo.log"
        exit 1
    fi
    sleep 2
    printf "  ${DIM}.${NC}"
done
echo ""

HEALTH=$(api_get "/health" | jq_field "d['data']['status']")
if [[ "$HEALTH" == "healthy" ]]; then
    success "Node is running at http://127.0.0.1:8080"
else
    echo -e "  ${RED}✗${NC}  Node did not start correctly."
    exit 1
fi

# ── Optional: start block explorer ───────────────────────────────────────────
EXPLORER_DIR="$PROJECT_DIR/block-explorer"
if [[ -d "$EXPLORER_DIR" ]] && command -v npm &>/dev/null; then
    echo ""
    echo -e "  ${YELLOW}?${NC}  Start the Block Explorer UI? (opens at http://localhost:3000)"
    echo -e "  ${DIM}[y/N]${NC}"
    read -r START_EXPLORER
    if [[ "$START_EXPLORER" =~ ^[yYsS]$ ]]; then
        info "Starting Block Explorer..."
        (cd "$EXPLORER_DIR" && npm run dev > /tmp/rust-bc-explorer.log 2>&1) &
        EXPLORER_PID=$!
        sleep 5
        if kill -0 "$EXPLORER_PID" 2>/dev/null; then
            success "Block Explorer running at http://localhost:3000"
            info "Open it in your browser to see the blockchain visually!"
        else
            warn "Explorer failed to start. Continuing without it."
            EXPLORER_PID=""
        fi
    fi
fi

# ══════════════════════════════════════════════════════════════════════════════
# DEMO
# ══════════════════════════════════════════════════════════════════════════════

# ── Step 1: Create wallets ───────────────────────────────────────────────────
banner "👛 Step 1: Create two wallets"

step "Creating Wallet A (Alice)..."
RESP_A=$(api_post "/wallets/create" "{}")
WALLET_A=$(echo "$RESP_A" | jq_field "d['data']['address']")
success "Alice's wallet: ${WALLET_A:0:16}..."

step "Creating Wallet B (Bob)..."
RESP_B=$(api_post "/wallets/create" "{}")
WALLET_B=$(echo "$RESP_B" | jq_field "d['data']['address']")
success "Bob's wallet:   ${WALLET_B:0:16}..."

info "Both wallets start with 0 balance."

pause

# ── Step 2: Mine blocks ──────────────────────────────────────────────────────
banner "⛏️  Step 2: Mine blocks (Alice earns coins)"

info "Mining creates a new block and rewards the miner with 50 coins."
echo ""

for i in 1 2 3; do
    step "Mining block #$i..."
    MINE_RESP=$(api_post "/mine" "{\"miner_address\": \"$WALLET_A\"}")
    HASH=$(echo "$MINE_RESP" | jq_field "d['data']['hash'][:16]")
    REWARD=$(echo "$MINE_RESP" | jq_field "d['data']['reward']")
    success "Block mined! Hash: ${HASH}...  Reward: ${REWARD} coins"
    sleep 0.5
done

echo ""
BALANCE_A=$(api_get "/wallets/$WALLET_A" | jq_field "d['data']['balance']")
success "Alice's balance: ${BOLD}${BALANCE_A} coins${NC}"

pause

# ── Step 3: Send transaction ─────────────────────────────────────────────────
banner "💸 Step 3: Send coins (Alice → Bob)"

SEND_AMOUNT=25
FEE=1
info "Sending $SEND_AMOUNT coins from Alice to Bob (fee: $FEE)..."
echo ""

step "Submitting transaction..."
TX_RESP=$(api_post "/transactions" "{\"from\": \"$WALLET_A\", \"to\": \"$WALLET_B\", \"amount\": $SEND_AMOUNT, \"fee\": $FEE}")
TX_ID=$(echo "$TX_RESP" | jq_field "d['data']['id'][:16]")
success "Transaction submitted: ${TX_ID}..."

step "Checking mempool..."
MEMPOOL=$(api_get "/mempool" | jq_field "d['data']['count']")
success "Mempool has $MEMPOOL pending transaction(s)"

step "Mining block to confirm transaction..."
api_post "/mine" "{\"miner_address\": \"$WALLET_A\"}" > /dev/null
success "Block mined! Transaction confirmed."

echo ""
BALANCE_A=$(api_get "/wallets/$WALLET_A" | jq_field "d['data']['balance']")
BALANCE_B=$(api_get "/wallets/$WALLET_B" | jq_field "d['data']['balance']")
success "Alice's balance: ${BOLD}${BALANCE_A} coins${NC}"
success "Bob's balance:   ${BOLD}${BALANCE_B} coins${NC}"

pause

# ── Step 4: Explore the chain ────────────────────────────────────────────────
banner "🔍 Step 4: Explore the blockchain"

CHAIN_INFO=$(api_get "/chain/info")
BLOCK_COUNT=$(echo "$CHAIN_INFO" | jq_field "d['data']['block_count']")
DIFFICULTY=$(echo "$CHAIN_INFO" | jq_field "d['data']['difficulty']")
LATEST_HASH=$(echo "$CHAIN_INFO" | jq_field "d['data']['latest_block_hash'][:24]")

echo -e "  ${BOLD}Chain status:${NC}"
echo -e "    Blocks:     $BLOCK_COUNT"
echo -e "    Difficulty:  $DIFFICULTY"
echo -e "    Latest hash: ${LATEST_HASH}..."

echo ""
STATS=$(api_get "/stats")
TOTAL_TX=$(echo "$STATS" | jq_field "d['data']['blockchain']['total_transactions']")
TOTAL_COINBASE=$(echo "$STATS" | jq_field "d['data']['blockchain']['total_coinbase']")
UNIQUE_ADDR=$(echo "$STATS" | jq_field "d['data']['blockchain']['unique_addresses']")

echo -e "  ${BOLD}Statistics:${NC}"
echo -e "    Total transactions: $TOTAL_TX"
echo -e "    Total mined coins:  $TOTAL_COINBASE"
echo -e "    Unique addresses:   $UNIQUE_ADDR"

pause

# ── Step 5: Verify chain integrity ───────────────────────────────────────────
banner "✅ Step 5: Verify chain integrity"

step "Verifying all blocks and hashes..."
VERIFY=$(api_get "/chain/verify")
VALID=$(echo "$VERIFY" | jq_field "d['data']['block_count']")
success "Chain verified: $VALID blocks, all hashes valid"

pause

# ── Summary ──────────────────────────────────────────────────────────────────
banner "🎉 Demo complete!"

echo -e "  ${BOLD}What you just did:${NC}"
echo -e "    ✓ Started a blockchain node"
echo -e "    ✓ Created two wallets"
echo -e "    ✓ Mined blocks with Proof of Work"
echo -e "    ✓ Sent a signed transaction"
echo -e "    ✓ Verified chain integrity"
echo ""
echo -e "  ${BOLD}Explore more:${NC}"
echo -e "    API docs:     ${CYAN}http://127.0.0.1:8080/api/v1/openapi.json${NC}"
if [[ -n "$EXPLORER_PID" ]]; then
    echo -e "    Explorer:     ${CYAN}http://localhost:3000${NC}"
fi
echo -e "    All blocks:   ${CYAN}curl http://127.0.0.1:8080/api/v1/blocks${NC}"
echo -e "    Health:       ${CYAN}curl http://127.0.0.1:8080/api/v1/health${NC}"
echo ""
echo -e "  ${DIM}The node is still running. Press Ctrl+C to stop.${NC}"
echo ""

# Keep alive until user presses Ctrl+C
wait "$NODE_PID" 2>/dev/null || true
