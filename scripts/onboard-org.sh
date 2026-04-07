#!/usr/bin/env bash
# onboard-org.sh — Add a new organization to a running rust-bc network.
#
# Usage:
#   ./scripts/onboard-org.sh <org_id> <node_name> <api_port> <p2p_port> [network_url]
#
# Example:
#   ./scripts/onboard-org.sh org3 node4 8088 8089
#   ./scripts/onboard-org.sh org3 node4 8088 8089 https://localhost:8080
#
# What it does:
#   1. Generate TLS certificate for the new node (signed by existing CA)
#   2. Register the organization in the network
#   3. Generate docker-compose override for the new node
#   4. Start the new node
#   5. Register peer in discovery service
#   6. Verify the node is healthy and synced

set -euo pipefail

# ── Args ──────────────────────────────────────────────────────────────────────

ORG_ID="${1:?Usage: $0 <org_id> <node_name> <api_port> <p2p_port> [network_url]}"
NODE_NAME="${2:?Usage: $0 <org_id> <node_name> <api_port> <p2p_port> [network_url]}"
API_PORT="${3:?Usage: $0 <org_id> <node_name> <api_port> <p2p_port> [network_url]}"
P2P_PORT="${4:?Usage: $0 <org_id> <node_name> <api_port> <p2p_port> [network_url]}"
NETWORK_URL="${5:-https://localhost:8080}"

CURL="curl -sk --max-time 10"
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TLS_DIR="$REPO_ROOT/deploy/tls"
COMPOSE_OVERRIDE="$REPO_ROOT/docker-compose.${NODE_NAME}.yml"

echo "══════════════════════════════════════════════════════"
echo "  Onboarding: $ORG_ID / $NODE_NAME"
echo "  API: $API_PORT  P2P: $P2P_PORT"
echo "  Network: $NETWORK_URL"
echo "══════════════════════════════════════════════════════"
echo ""

# ── Step 1: Generate TLS certificate ─────────────────────────────────────────

echo "▶ Step 1: Generating TLS certificate for $NODE_NAME..."

if [ ! -f "$TLS_DIR/ca-key.pem" ]; then
    echo "  ERROR: CA key not found at $TLS_DIR/ca-key.pem"
    echo "  Run 'cd deploy && bash generate-tls.sh' first."
    exit 1
fi

if [ -f "$TLS_DIR/$NODE_NAME-cert.pem" ]; then
    echo "  Certificate already exists, skipping."
else
    # CSR
    openssl req -newkey ec -pkeyopt ec_paramgen_curve:prime256v1 \
        -nodes \
        -keyout "$TLS_DIR/$NODE_NAME-key.pem" \
        -out "$TLS_DIR/$NODE_NAME.csr" \
        -subj "/CN=$NODE_NAME/O=$ORG_ID" 2>/dev/null

    # SAN config
    cat > "$TLS_DIR/$NODE_NAME-ext.cnf" <<EOF
[v3_req]
subjectAltName = DNS:$NODE_NAME,DNS:localhost,IP:127.0.0.1
EOF

    # Sign with CA
    openssl x509 -req \
        -in "$TLS_DIR/$NODE_NAME.csr" \
        -CA "$TLS_DIR/ca-cert.pem" \
        -CAkey "$TLS_DIR/ca-key.pem" \
        -CAcreateserial \
        -days 365 \
        -extfile "$TLS_DIR/$NODE_NAME-ext.cnf" \
        -extensions v3_req \
        -out "$TLS_DIR/$NODE_NAME-cert.pem" 2>/dev/null

    rm -f "$TLS_DIR/$NODE_NAME.csr" "$TLS_DIR/$NODE_NAME-ext.cnf"
    chmod 644 "$TLS_DIR/$NODE_NAME-key.pem" "$TLS_DIR/$NODE_NAME-cert.pem"

    echo "  ✓ Certificate generated: $TLS_DIR/$NODE_NAME-cert.pem"
fi

# ── Step 2: Register organization ────────────────────────────────────────────

echo ""
echo "▶ Step 2: Registering organization $ORG_ID..."

resp=$($CURL "$NETWORK_URL/api/v1/store/organizations" -X POST \
    -H 'Content-Type: application/json' \
    -d "{\"org_id\": \"$ORG_ID\", \"name\": \"$ORG_ID\", \"msp_id\": \"${ORG_ID}MSP\"}" 2>&1)

status=$(echo "$resp" | jq -r '.status_code // .status // empty' 2>/dev/null)
if [[ "$status" == "200" ]] || [[ "$status" == "201" ]] || echo "$resp" | grep -q "already exists"; then
    echo "  ✓ Organization registered (or already exists)"
else
    echo "  ⚠ Registration response: $(echo "$resp" | head -c 200)"
fi

# ── Step 3: Generate docker-compose override ─────────────────────────────────

echo ""
echo "▶ Step 3: Generating $COMPOSE_OVERRIDE..."

# Find existing bootstrap nodes from the network
BOOTSTRAP="node1:8081,node2:8081,node3:8081"

cat > "$COMPOSE_OVERRIDE" <<EOF
version: '3.8'

services:
  $NODE_NAME:
    build: .
    container_name: rust-bc-$NODE_NAME
    restart: unless-stopped
    networks:
      - bc-net
    ports:
      - "$API_PORT:$API_PORT"
      - "$P2P_PORT:$P2P_PORT"
    environment:
      BIND_ADDR: "0.0.0.0"
      P2P_EXTERNAL_ADDRESS: "$NODE_NAME:$P2P_PORT"
      API_PORT: "$API_PORT"
      P2P_PORT: "$P2P_PORT"
      DIFFICULTY: "1"
      RUST_LOG: "info"
      NETWORK_ID: "local-test"
      ORG_ID: "$ORG_ID"
      NODE_ROLE: "peer"
      STORAGE_BACKEND: "rocksdb"
      STORAGE_PATH: "/app/data/rocksdb"
      TLS_CERT_PATH: "/tls/$NODE_NAME-cert.pem"
      TLS_KEY_PATH: "/tls/$NODE_NAME-key.pem"
      TLS_CA_CERT_PATH: "/tls/ca-cert.pem"
      BOOTSTRAP_NODES: "$BOOTSTRAP"
      ACL_MODE: "\${ACL_MODE:-permissive}"
    volumes:
      - ${NODE_NAME}-data:/app/data
      - ./deploy/tls:/tls:ro
    healthcheck:
      test: ["CMD", "curl", "-fk", "https://localhost:$API_PORT/api/v1/health"]
      interval: 15s
      timeout: 5s
      retries: 3
      start_period: 10s

volumes:
  ${NODE_NAME}-data:

networks:
  bc-net:
    external: true
    name: rust-bc_bc-net
EOF

echo "  ✓ Compose file generated"

# ── Step 4: Start the new node ───────────────────────────────────────────────

echo ""
echo "▶ Step 4: Starting $NODE_NAME..."

docker compose -f "$REPO_ROOT/docker-compose.yml" -f "$COMPOSE_OVERRIDE" up -d "$NODE_NAME"

echo "  Waiting for node to start..."
sleep 10

# ── Step 5: Register peer in discovery ───────────────────────────────────────

echo ""
echo "▶ Step 5: Registering peer in discovery service..."

resp=$($CURL "$NETWORK_URL/api/v1/discovery/register" -X POST \
    -H 'Content-Type: application/json' \
    -d "{
        \"peer_address\": \"$NODE_NAME:$P2P_PORT\",
        \"org_id\": \"$ORG_ID\",
        \"role\": \"Peer\",
        \"chaincodes\": [],
        \"channels\": [\"default\"]
    }" 2>&1)

echo "  ✓ Peer registered in discovery"

# ── Step 6: Verify ───────────────────────────────────────────────────────────

echo ""
echo "▶ Step 6: Verifying node health..."

ok=0
for i in $(seq 1 15); do
    resp=$($CURL "https://localhost:$API_PORT/api/v1/health" 2>&1)
    status=$(echo "$resp" | jq -r '.data.status // empty' 2>/dev/null)
    if [[ "$status" == "healthy" ]] || [[ "$status" == "degraded" ]]; then
        echo "  ✓ Node is $status (attempt $i)"
        ok=1
        break
    fi
    sleep 2
done

if [[ "$ok" -ne 1 ]]; then
    echo "  ⚠ Node did not become healthy within 30 seconds"
    echo "  Check logs: docker compose -f docker-compose.yml -f $COMPOSE_OVERRIDE logs $NODE_NAME"
    exit 1
fi

# Check sync status
resp=$($CURL "https://localhost:$API_PORT/api/v1/chain/info" 2>&1)
blocks=$(echo "$resp" | jq -r '.data.block_count // "?"' 2>/dev/null)
echo "  ✓ Block count: $blocks"

echo ""
echo "══════════════════════════════════════════════════════"
echo "  ✓ $ORG_ID / $NODE_NAME onboarded successfully"
echo ""
echo "  API:  https://localhost:$API_PORT/api/v1"
echo "  P2P:  $NODE_NAME:$P2P_PORT"
echo ""
echo "  To stop:    docker compose -f docker-compose.yml -f $COMPOSE_OVERRIDE stop $NODE_NAME"
echo "  To remove:  docker compose -f docker-compose.yml -f $COMPOSE_OVERRIDE down $NODE_NAME"
echo "  To logs:    docker compose -f docker-compose.yml -f $COMPOSE_OVERRIDE logs $NODE_NAME"
echo "══════════════════════════════════════════════════════"
