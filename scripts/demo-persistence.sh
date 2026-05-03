#!/usr/bin/env bash
# Cerulean Ledger DLT — Restart persistence proof
#
# Proves data survives container restarts:
# 1. Submits a tx
# 2. Stops containers
# 3. Restarts containers
# 4. Queries the tx — must still exist
#
# Usage:
#   ./scripts/demo-persistence.sh
#
# Requires: docker-compose.demo.yml

set -euo pipefail

COMPOSE="docker compose -f docker-compose.demo.yml"
NODE1="http://localhost:9600"
TX_ID="demo-persistence-$(date +%s)"
HEADER="Content-Type: application/json"

echo "=== Cerulean Ledger — Persistence Proof ==="
echo ""

# 1. Ensure nodes are running
echo "--- 1. Checking nodes are up ---"
$COMPOSE up -d
sleep 15
curl -sf "$NODE1/api/v1/health" > /dev/null || { echo "FAIL: node1 not healthy"; exit 1; }
echo "node1 healthy"
echo ""

# 2. Submit tx
echo "--- 2. Submit tx: $TX_ID ---"
curl -s -X POST "$NODE1/api/v1/gateway/submit" \
  -H "$HEADER" \
  -d "{\"chaincode_id\":\"notarize\",\"transaction\":{\"id\":\"$TX_ID\",\"input_did\":\"did:cerulean:alice\",\"output_recipient\":\"did:cerulean:bob\",\"amount\":0}}" | python3 -m json.tool
echo ""

# 3. Verify tx exists before restart
echo "--- 3. Query before restart ---"
curl -s "$NODE1/api/v1/tx/$TX_ID" | python3 -c "
import sys, json
d = json.load(sys.stdin)['data']
print(f'tx_id: {d[\"id\"]}')
print(f'state: {d[\"state\"]}')
print(f'block_height: {d[\"block_height\"]}')
"
echo ""

# 4. Stop containers
echo "--- 4. Stopping containers ---"
$COMPOSE down
echo "Containers stopped"
sleep 3
echo ""

# 5. Restart containers
echo "--- 5. Restarting containers ---"
$COMPOSE up -d
echo "Waiting for node1 to be healthy..."
for i in $(seq 1 30); do
  if curl -sf "$NODE1/api/v1/health" > /dev/null 2>&1; then
    echo "node1 healthy after ${i}s"
    break
  fi
  sleep 1
done
echo ""

# 6. Query tx after restart
echo "--- 6. Query after restart ---"
RESULT=$(curl -s "$NODE1/api/v1/tx/$TX_ID")
echo "$RESULT" | python3 -m json.tool
echo ""

# 7. Verify
STATE=$(echo "$RESULT" | python3 -c "import sys,json; print(json.load(sys.stdin)['data']['state'])" 2>/dev/null || echo "MISSING")
echo "=== RESULT ==="
if [ "$STATE" = "committed" ]; then
  echo "PASS: Transaction survived restart (state=$STATE)"
else
  echo "FAIL: Transaction lost after restart (state=$STATE)"
  exit 1
fi
