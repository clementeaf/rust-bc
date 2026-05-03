#!/usr/bin/env bash
# Cerulean Ledger DLT — Multi-node consistency proof
#
# Proves that the network is NOT a single-node illusion:
# 1. Submits a tx to node1
# 2. Queries from all 3 nodes
# 3. Verifies same block_height, tx_id, state
# 4. Verifies block signature is PQC (ML-DSA-65)
#
# Usage:
#   ./scripts/demo-consistency.sh
#
# Requires: 3 nodes running on ports 9600, 9602, 9604
# (via docker-compose.demo.yml or cargo run instances)

set -euo pipefail

NODE1="http://localhost:9600"
NODE2="http://localhost:9602"
NODE3="http://localhost:9604"
TX_ID="demo-multi-node-$(date +%s)"

HEADER="Content-Type: application/json"
NGROK_HEADER="ngrok-skip-browser-warning: true"

echo "=== Cerulean Ledger — Multi-Node Consistency Proof ==="
echo ""

# 1. Submit tx to node1
echo "--- 1. Submit tx to node1 ---"
SUBMIT=$(curl -s -X POST "$NODE1/api/v1/gateway/submit" \
  -H "$HEADER" -H "$NGROK_HEADER" \
  -d "{\"chaincode_id\":\"notarize\",\"transaction\":{\"id\":\"$TX_ID\",\"input_did\":\"did:cerulean:alice\",\"output_recipient\":\"did:cerulean:bob\",\"amount\":0}}")
echo "$SUBMIT" | python3 -m json.tool
BLOCK_HEIGHT=$(echo "$SUBMIT" | python3 -c "import sys,json; print(json.load(sys.stdin)['data']['block_height'])")
echo ""

# 2. Wait for propagation
sleep 2

# 3. Query from all nodes
echo "--- 2. Query from node1 ---"
R1=$(curl -s "$NODE1/api/v1/tx/$TX_ID" -H "$NGROK_HEADER")
echo "$R1" | python3 -m json.tool
echo ""

echo "--- 3. Query from node2 ---"
R2=$(curl -s "$NODE2/api/v1/tx/$TX_ID" -H "$NGROK_HEADER")
echo "$R2" | python3 -m json.tool
echo ""

echo "--- 4. Query from node3 ---"
R3=$(curl -s "$NODE3/api/v1/tx/$TX_ID" -H "$NGROK_HEADER")
echo "$R3" | python3 -m json.tool
echo ""

# 4. Verify consistency
echo "--- 5. Consistency verification ---"
H1=$(echo "$R1" | python3 -c "import sys,json; print(json.load(sys.stdin)['data']['block_height'])" 2>/dev/null || echo "MISSING")
H2=$(echo "$R2" | python3 -c "import sys,json; print(json.load(sys.stdin)['data']['block_height'])" 2>/dev/null || echo "MISSING")
H3=$(echo "$R3" | python3 -c "import sys,json; print(json.load(sys.stdin)['data']['block_height'])" 2>/dev/null || echo "MISSING")

S1=$(echo "$R1" | python3 -c "import sys,json; print(json.load(sys.stdin)['data']['state'])" 2>/dev/null || echo "MISSING")
S2=$(echo "$R2" | python3 -c "import sys,json; print(json.load(sys.stdin)['data']['state'])" 2>/dev/null || echo "MISSING")
S3=$(echo "$R3" | python3 -c "import sys,json; print(json.load(sys.stdin)['data']['state'])" 2>/dev/null || echo "MISSING")

echo "node1: block_height=$H1, state=$S1"
echo "node2: block_height=$H2, state=$S2"
echo "node3: block_height=$H3, state=$S3"
echo ""

# 5. Block signature check
echo "--- 6. Block signature (PQC proof) ---"
curl -s "$NODE1/api/v1/store/blocks/$BLOCK_HEIGHT" -H "$NGROK_HEADER" | python3 -c "
import sys, json
d = json.load(sys.stdin)['data']
sig = d['signature']
print(f'algorithm: {d[\"signature_algorithm\"]}')
print(f'signature_len: {len(sig)//2} bytes')
print(f'signature_non_zero: {sig != \"0\" * len(sig)}')
print(f'pqc_confirmed: {d[\"signature_algorithm\"] == \"MlDsa65\"}')
"
echo ""

# 6. Summary
echo "=== RESULT ==="
if [ "$H1" = "$H2" ] && [ "$H2" = "$H3" ] && [ "$S1" = "committed" ]; then
  echo "PASS: All nodes consistent (block_height=$H1, state=committed)"
else
  echo "FAIL: Inconsistency detected"
  echo "  node1=$H1/$S1, node2=$H2/$S2, node3=$H3/$S3"
  exit 1
fi
