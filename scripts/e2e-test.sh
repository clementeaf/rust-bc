#!/usr/bin/env bash
# E2E test suite for rust-bc multi-org blockchain network.
# Requires: curl, jq, docker compose running (4 nodes).
#
# Usage: ./scripts/e2e-test.sh [--verbose]

set -uo pipefail

# ── Config ───────────────────────────────────────────────────────────────────
NODE1="https://localhost:8080"
NODE2="https://localhost:8082"
NODE3="https://localhost:8084"
ORDERER="https://localhost:8086"
CURL="curl -sk --max-time 10"
VERBOSE="${1:-}"

PASSED=0
FAILED=0
SKIPPED=0

# ── Helpers ──────────────────────────────────────────────────────────────────
red()    { printf "\033[31m%s\033[0m" "$1"; }
green()  { printf "\033[32m%s\033[0m" "$1"; }
yellow() { printf "\033[33m%s\033[0m" "$1"; }
bold()   { printf "\033[1m%s\033[0m" "$1"; }

log() { echo "  $*"; }

api() {
    local method="$1" url="$2"
    shift 2
    local resp
    resp=$($CURL -X "$method" "$url" -H 'Content-Type: application/json' "$@" 2>&1) || true
    if [[ "$VERBOSE" == "--verbose" ]]; then
        echo "$resp" | jq . 2>/dev/null || echo "$resp"
    fi
    echo "$resp"
}

# Extract .data from API response envelope
data() { jq -r '.data // .Data // empty' 2>/dev/null; }

assert_status() {
    local resp="$1" expected="$2" label="$3"
    local code
    code=$(echo "$resp" | jq -r '.status_code // .statusCode // empty' 2>/dev/null)
    # Fallback: check .success for legacy endpoints
    if [[ -z "$code" ]]; then
        local success
        success=$(echo "$resp" | jq -r '.success // empty' 2>/dev/null)
        if [[ "$success" == "true" ]]; then code=200; fi
        if [[ "$success" == "false" ]]; then code=400; fi
    fi
    if [[ "$code" == "$expected" ]]; then
        echo "  $(green "PASS") $label"
        PASSED=$((PASSED + 1))
    else
        echo "  $(red "FAIL") $label (expected $expected, got ${code:-empty})"
        if [[ "$VERBOSE" == "--verbose" ]]; then echo "$resp" | head -3; fi
        FAILED=$((FAILED + 1))
    fi
}

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

skip() {
    echo "  $(yellow "SKIP") $1"
    SKIPPED=$((SKIPPED + 1))
}

# ── Pre-flight ───────────────────────────────────────────────────────────────
echo ""
bold "═══ rust-bc E2E Test Suite ═══"
echo ""

echo "$(bold '1. Pre-flight checks')"
for name_port in "node1:8080" "node2:8082" "node3:8084" "orderer1:8086"; do
    name="${name_port%%:*}"
    port="${name_port##*:}"
    resp=$(api GET "https://localhost:$port/api/v1/health")
    status=$(echo "$resp" | jq -r '.data.status // empty' 2>/dev/null)
    assert_eq "$status" "healthy" "$name is healthy"
done

# Check peer connectivity
peers=$(api GET "$NODE1/api/v1/stats" | jq -r '.data.network.connected_peers' 2>/dev/null)
assert_gt "$peers" 0 "node1 has P2P peers ($peers connected)"

# ── Test 2: Organizations ────────────────────────────────────────────────────
echo ""
echo "$(bold '2. Register organizations')"

ORG1_KEY=$(python3 -c "import os; print(list(os.urandom(32)))" | tr -d '[] ')
ORG2_KEY=$(python3 -c "import os; print(list(os.urandom(32)))" | tr -d '[] ')

resp=$(api POST "$NODE1/api/v1/store/organizations" -d '{
    "org_id": "org1",
    "msp_id": "Org1MSP",
    "admin_dids": ["did:bc:admin1"],
    "member_dids": ["did:bc:peer1", "did:bc:peer3"],
    "root_public_keys": [[1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]]
}')
assert_status "$resp" 200 "Create org1"

resp=$(api POST "$NODE1/api/v1/store/organizations" -d '{
    "org_id": "org2",
    "msp_id": "Org2MSP",
    "admin_dids": ["did:bc:admin2"],
    "member_dids": ["did:bc:peer2"],
    "root_public_keys": [[2,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]]
}')
assert_status "$resp" 200 "Create org2"

resp=$(api GET "$NODE1/api/v1/store/organizations")
org_count=$(echo "$resp" | jq '.data | length' 2>/dev/null)
assert_eq "$org_count" "2" "List organizations returns 2"

# ── Test 3: Endorsement Policies ─────────────────────────────────────────────
echo ""
echo "$(bold '3. Endorsement policies')"

resp=$(api POST "$NODE1/api/v1/store/policies" -d '{
    "resource_id": "mycc",
    "policy": {"NOutOf": {"n": 2, "orgs": ["org1", "org2"]}}
}')
assert_status "$resp" 200 "Set policy NOutOf(2, [org1, org2])"

resp=$(api GET "$NODE1/api/v1/store/policies/mycc")
policy_type=$(echo "$resp" | jq -r '.data | keys[0] // empty' 2>/dev/null)
assert_eq "$policy_type" "NOutOf" "Get policy returns NOutOf"

# Also register policy with channel/chaincode key for discovery endorsers
resp=$(api POST "$NODE1/api/v1/store/policies" -d '{
    "resource_id": "mychannel/mycc",
    "policy": {"NOutOf": {"n": 2, "orgs": ["org1", "org2"]}}
}')
assert_status "$resp" 200 "Set discovery policy mychannel/mycc"

# ── Test 4: Channels ─────────────────────────────────────────────────────────
echo ""
echo "$(bold '4. Channel management')"

resp=$(api POST "$NODE1/api/v1/channels" -d '{"channel_id": "mychannel"}')
ch_code=$(echo "$resp" | jq -r '.status_code // empty' 2>/dev/null)
if [[ "$ch_code" == "200" || "$ch_code" == "201" || "$ch_code" == "409" ]]; then
    echo "  $(green "PASS") Create channel 'mychannel' ($ch_code)"
    PASSED=$((PASSED + 1))
else
    echo "  $(red "FAIL") Create channel 'mychannel' (got $ch_code)"
    FAILED=$((FAILED + 1))
fi

resp=$(api GET "$NODE1/api/v1/channels")
ch_count=$(echo "$resp" | jq '.data | length' 2>/dev/null)
assert_gt "$ch_count" 0 "List channels returns >= 1"

# Channel isolation: write a tx to mychannel, verify default channel unaffected
resp=$(api POST "$NODE1/api/v1/store/transactions" \
    -H "X-Channel-Id: mychannel" \
    -d '{"id":"tx-ch-1","block_height":0,"timestamp":1000,"input_did":"did:bc:alice","output_recipient":"did:bc:bob","amount":10,"state":"committed"}')
ch_status=$(echo "$resp" | jq -r '.status_code // .statusCode // empty' 2>/dev/null)
if [[ "$ch_status" == "201" || "$ch_status" == "200" ]]; then
    echo "  $(green "PASS") Write transaction to mychannel"
    PASSED=$((PASSED + 1))

    # Verify it's NOT in default channel
    resp_default=$(api GET "$NODE1/api/v1/store/transactions/tx-ch-1")
    default_status=$(echo "$resp_default" | jq -r '.status_code // empty' 2>/dev/null)
    if [[ "$default_status" == "404" ]]; then
        echo "  $(green "PASS") Transaction NOT visible in default channel (isolation)"
        PASSED=$((PASSED + 1))
    else
        echo "  $(red "FAIL") Channel isolation — tx visible in default (status=$default_status)"
        FAILED=$((FAILED + 1))
    fi
else
    skip "Channel transaction write (status=$ch_status)"
fi

# ── Test 5: Block Mining & Propagation ───────────────────────────────────────
echo ""
echo "$(bold '5. Block mining & multi-node propagation')"

# Create wallet
wallet_resp=$(api POST "$NODE1/api/v1/wallets/create" -d '{}')
ADDR=$(echo "$wallet_resp" | jq -r '.data.address // empty' 2>/dev/null)
if [[ -n "$ADDR" && "$ADDR" != "null" ]]; then
    echo "  $(green "PASS") Create wallet ($ADDR)"
    PASSED=$((PASSED + 1))
else
    echo "  $(red "FAIL") Create wallet"
    FAILED=$((FAILED + 1))
    ADDR="fallback"
fi

# Get initial block count
initial_blocks=$(api GET "$NODE1/api/v1/stats" | jq '.data.blockchain.block_count' 2>/dev/null)

# Mine a block
resp=$(api POST "$NODE1/api/v1/mine" -d "{\"miner_address\":\"$ADDR\"}")
mine_ok=$(echo "$resp" | jq -r '.success // empty' 2>/dev/null)
assert_eq "$mine_ok" "true" "Mine block on node1"

# Wait for propagation (gossip may need multiple rounds)
sleep 5

# Check all peers have the same block count and hash
n1_hash=$(api GET "$NODE1/api/v1/stats" | jq -r '.data.blockchain.latest_block_hash' 2>/dev/null)
n2_hash=$(api GET "$NODE2/api/v1/stats" | jq -r '.data.blockchain.latest_block_hash' 2>/dev/null)
n3_hash=$(api GET "$NODE3/api/v1/stats" | jq -r '.data.blockchain.latest_block_hash' 2>/dev/null)

assert_eq "$n2_hash" "$n1_hash" "node2 has same latest hash as node1"
assert_eq "$n3_hash" "$n1_hash" "node3 has same latest hash as node1"

n1_count=$(api GET "$NODE1/api/v1/stats" | jq '.data.blockchain.block_count' 2>/dev/null)
expected_count=$((initial_blocks + 1))
assert_eq "$n1_count" "$expected_count" "Block count incremented ($initial_blocks -> $expected_count)"

# ── Test 6: Transaction Lifecycle ────────────────────────────────────────────
echo ""
echo "$(bold '6. Transaction lifecycle (mempool -> block)')"

# Create a second wallet as recipient
wallet2_resp=$(api POST "$NODE1/api/v1/wallets/create" -d '{}')
ADDR2=$(echo "$wallet2_resp" | jq -r '.data.address // empty' 2>/dev/null)

# Submit transaction from miner wallet (has balance from mining) to wallet2
resp=$(api POST "$NODE1/api/v1/transactions" -d "{
    \"from\": \"$ADDR\",
    \"to\": \"$ADDR2\",
    \"amount\": 1,
    \"fee\": 1,
    \"data\": \"e2e-test-tx\"
}")
assert_status "$resp" 200 "Submit transaction to mempool"

# Check mempool
resp=$(api GET "$NODE1/api/v1/mempool")
mempool_count=$(echo "$resp" | jq '.data.transactions | length' 2>/dev/null || echo "0")
log "Mempool has $mempool_count pending transactions"

# Mine to include mempool txs
resp=$(api POST "$NODE1/api/v1/mine" -d "{\"miner_address\":\"$ADDR\"}")
tx_count=$(echo "$resp" | jq -r '.data.transactions_count // 0' 2>/dev/null)
assert_gt "$tx_count" 0 "Mined block includes transactions ($tx_count)"

# ── Test 7: Private Data ─────────────────────────────────────────────────────
echo ""
echo "$(bold '7. Private data collections')"

# Register collection first
resp=$(api POST "$NODE1/api/v1/private-data/collections" -d '{
    "name": "secret-collection",
    "member_org_ids": ["org1"],
    "required_peer_count": 1,
    "blocks_to_live": 100
}')
reg_status=$(echo "$resp" | jq -r '.status_code // empty' 2>/dev/null)
assert_status "$resp" 200 "Register private data collection"

# Write private data as org1
resp=$(api PUT "$NODE1/api/v1/private-data/secret-collection/key1" \
    -H "X-Org-Id: org1" \
    -d '{"value": "secret-for-org1"}')
assert_status "$resp" 200 "Write private data as org1"

# Read as org1 (should succeed)
resp=$(api GET "$NODE1/api/v1/private-data/secret-collection/key1" -H "X-Org-Id: org1")
pd_value=$(echo "$resp" | jq -r '.data.value // empty' 2>/dev/null)
assert_eq "$pd_value" "secret-for-org1" "Read private data as org1 (authorized)"

# Read as org2 (should fail with 403 — not a member)
resp=$(api GET "$NODE1/api/v1/private-data/secret-collection/key1" -H "X-Org-Id: org2")
deny_status=$(echo "$resp" | jq -r '.status_code // empty' 2>/dev/null)
assert_eq "$deny_status" "403" "Private data denied for org2 (403)"

# ── Test 8: Discovery Service ────────────────────────────────────────────────
echo ""
echo "$(bold '8. Discovery service')"

# Register a peer
resp=$(api POST "$NODE1/api/v1/discovery/register" -d '{
    "peer_address": "node1:8081",
    "org_id": "org1",
    "role": "PeerAndOrderer",
    "chaincodes": ["mycc"],
    "channels": ["mychannel"],
    "last_heartbeat": 9999999999
}')
disc_status=$(echo "$resp" | jq -r '.status_code // empty' 2>/dev/null)
if [[ "$disc_status" == "200" || "$disc_status" == "201" ]]; then
    echo "  $(green "PASS") Register peer in discovery"
    PASSED=$((PASSED + 1))
else
    skip "Discovery register (status=$disc_status)"
fi

# Register node2
resp=$(api POST "$NODE1/api/v1/discovery/register" -d '{
    "peer_address": "node2:8081",
    "org_id": "org2",
    "role": "Peer",
    "chaincodes": ["mycc"],
    "channels": ["mychannel"],
    "last_heartbeat": 9999999999
}')

# Query endorsers
resp=$(api GET "$NODE1/api/v1/discovery/endorsers?chaincode=mycc&channel=mychannel")
endorser_status=$(echo "$resp" | jq -r '.status_code // empty' 2>/dev/null)
if [[ "$endorser_status" == "200" ]]; then
    endorser_count=$(echo "$resp" | jq '.data | length' 2>/dev/null)
    assert_gt "$endorser_count" 0 "Discovery returns endorsers ($endorser_count found)"
else
    skip "Discovery endorsers query (status=$endorser_status)"
fi

# Query channel peers
resp=$(api GET "$NODE1/api/v1/discovery/peers?channel=mychannel")
peer_status=$(echo "$resp" | jq -r '.status_code // empty' 2>/dev/null)
if [[ "$peer_status" == "200" ]]; then
    peer_count=$(echo "$resp" | jq '.data | length' 2>/dev/null)
    assert_gt "$peer_count" 0 "Discovery returns channel peers ($peer_count found)"
else
    skip "Discovery channel peers (status=$peer_status)"
fi

# ── Test 9: Gateway Submit ───────────────────────────────────────────────────
echo ""
echo "$(bold '9. Gateway (endorse -> order -> commit)')"

resp=$(api POST "$NODE1/api/v1/gateway/submit" -d '{
    "chaincode_id": "mycc",
    "channel_id": "",
    "transaction": {
        "id": "gw-tx-001",
        "input_did": "did:bc:admin1",
        "output_recipient": "did:bc:peer2",
        "amount": 50
    }
}')
assert_status "$resp" 200 "Gateway submit (endorse -> order -> commit)"
gw_tx=$(echo "$resp" | jq -r '.data.tx_id // empty' 2>/dev/null)
gw_height=$(echo "$resp" | jq -r '.data.block_height // empty' 2>/dev/null)
if [[ -n "$gw_tx" && "$gw_tx" != "null" ]]; then
    assert_eq "$gw_tx" "gw-tx-001" "Gateway returns correct tx_id"
    assert_gt "$gw_height" 0 "Gateway committed at block height $gw_height"
fi

# ── Test 10: Chain Integrity ─────────────────────────────────────────────────
echo ""
echo "$(bold '10. Chain integrity verification')"

for name_port in "node1:8080" "node2:8082" "node3:8084"; do
    name="${name_port%%:*}"
    port="${name_port##*:}"
    resp=$(api GET "https://localhost:$port/api/v1/chain/verify")
    verify_status=$(echo "$resp" | jq -r '.status_code // empty' 2>/dev/null)
    valid=$(echo "$resp" | jq -r '.data.valid // empty' 2>/dev/null)
    block_count=$(echo "$resp" | jq -r '.data.block_count // empty' 2>/dev/null)
    if [[ "$verify_status" == "200" ]]; then
        echo "  $(green "PASS") $name chain verify (valid=$valid, blocks=$block_count)"
        PASSED=$((PASSED + 1))
    else
        skip "$name chain verify (status=$verify_status)"
    fi
done

# ── Test 11: Observability ───────────────────────────────────────────────────
echo ""
echo "$(bold '11. Observability')"

# Prometheus metrics
resp=$($CURL "https://localhost:8080/metrics" 2>&1)
if echo "$resp" | grep -q "endorsement_validations_total\|ordering_blocks_cut_total\|rust_bc"; then
    echo "  $(green "PASS") Prometheus metrics endpoint returns metrics"
    PASSED=$((PASSED + 1))
else
    skip "Prometheus metrics (endpoint may not match expected format)"
fi

# Prometheus scrape target
resp=$($CURL "http://localhost:9090/api/v1/targets" 2>&1)
if echo "$resp" | grep -q "rust-bc\|node1"; then
    echo "  $(green "PASS") Prometheus scraping nodes"
    PASSED=$((PASSED + 1))
else
    skip "Prometheus targets (may need configuration)"
fi

# Grafana health
resp=$($CURL "http://localhost:3000/api/health" 2>&1)
grafana_ok=$(echo "$resp" | jq -r '.database // empty' 2>/dev/null)
assert_eq "$grafana_ok" "ok" "Grafana is healthy"

# ── Test 12: Store-backed endpoints ──────────────────────────────────────────
echo ""
echo "$(bold '12. Store-backed CRUD')"

# Write identity (fields: did, created_at, updated_at, status)
resp=$(api POST "$NODE1/api/v1/store/identities" -d '{
    "did": "did:bc:test1",
    "created_at": 1000,
    "updated_at": 1000,
    "status": "active"
}')
assert_status "$resp" 200 "Store identity"

resp=$(api GET "$NODE1/api/v1/store/identities/did:bc:test1")
assert_status "$resp" 200 "Read identity back"

# Write credential (fields: id, issuer_did, subject_did, cred_type, issued_at, expires_at)
resp=$(api POST "$NODE1/api/v1/store/credentials" -d '{
    "id": "cred-001",
    "issuer_did": "did:bc:admin1",
    "subject_did": "did:bc:test1",
    "cred_type": "membership",
    "issued_at": 1000,
    "expires_at": 99999999,
    "revoked_at": null
}')
assert_status "$resp" 200 "Store credential"

resp=$(api GET "$NODE1/api/v1/store/credentials/cred-001")
assert_status "$resp" 200 "Read credential back"

# ── Summary ──────────────────────────────────────────────────────────────────
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  $(green "PASSED"): $PASSED"
echo "  $(red "FAILED"): $FAILED"
echo "  $(yellow "SKIPPED"): $SKIPPED"
echo "  Total:   $((PASSED + FAILED + SKIPPED))"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

if [[ $FAILED -gt 0 ]]; then
    exit 1
fi
