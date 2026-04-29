#!/usr/bin/env bash
set -euo pipefail

# Cerulean Ledger — Cryptocurrency Testnet launcher
# Usage: ./scripts/testnet.sh [up|down|status|faucet|balance|transfer]

COMPOSE="docker compose -f deploy/testnet/docker-compose.testnet.yml"
NODE1="http://localhost:8080/api/v1"
NODE2="http://localhost:8082/api/v1"
NODE3="http://localhost:8084/api/v1"

case "${1:-help}" in
  up)
    echo "Starting 3-node testnet..."
    $COMPOSE up -d --build
    echo "Waiting for nodes..."
    sleep 5
    for url in "$NODE1/health" "$NODE2/health" "$NODE3/health"; do
      if curl -sf "$url" > /dev/null 2>&1; then
        echo "  ✓ $url"
      else
        echo "  ✗ $url (not ready yet)"
      fi
    done
    echo ""
    echo "Testnet running:"
    echo "  Node 1: $NODE1"
    echo "  Node 2: $NODE2"
    echo "  Node 3: $NODE3"
    echo ""
    echo "Get tokens:  ./scripts/testnet.sh faucet <address>"
    echo "Check balance: ./scripts/testnet.sh balance <address>"
    ;;

  down)
    echo "Stopping testnet..."
    $COMPOSE down -v
    ;;

  status)
    for i in 1 2 3; do
      port=$((8078 + i * 2))
      url="http://localhost:$port/api/v1"
      echo -n "Node $i ($url): "
      if curl -sf "$url/health" > /dev/null 2>&1; then
        echo "✓ healthy"
      else
        echo "✗ unreachable"
      fi
    done
    ;;

  faucet)
    addr="${2:?Usage: testnet.sh faucet <address>}"
    echo "Requesting tokens for $addr..."
    curl -s -X POST "$NODE1/faucet/drip" \
      -H "Content-Type: application/json" \
      -d "{\"address\": \"$addr\"}" | python3 -m json.tool 2>/dev/null || true
    ;;

  balance)
    addr="${2:?Usage: testnet.sh balance <address>}"
    echo "Balance on all nodes:"
    for i in 1 2 3; do
      port=$((8078 + i * 2))
      echo -n "  Node $i: "
      curl -s "http://localhost:$port/api/v1/accounts/$addr" | python3 -c "
import sys, json
d = json.load(sys.stdin)
if 'data' in d and d['data']:
    print(f\"balance={d['data']['balance']} nonce={d['data']['nonce']}\")
else:
    print('no data')
" 2>/dev/null || echo "unreachable"
    done
    ;;

  transfer)
    from="${2:?Usage: testnet.sh transfer <from> <to> <amount>}"
    to="${3:?Usage: testnet.sh transfer <from> <to> <amount>}"
    amount="${4:?Usage: testnet.sh transfer <from> <to> <amount>}"
    # Get current nonce
    nonce=$(curl -s "$NODE1/accounts/$from" | python3 -c "
import sys, json
d = json.load(sys.stdin)
print(d.get('data', {}).get('nonce', 0))
" 2>/dev/null || echo 0)
    echo "Sending $amount from $from to $to (nonce=$nonce, fee=5)..."
    curl -s -X POST "$NODE1/transfer" \
      -H "Content-Type: application/json" \
      -d "{\"from\": \"$from\", \"to\": \"$to\", \"amount\": $amount, \"nonce\": $nonce, \"fee\": 5}" \
      | python3 -m json.tool 2>/dev/null || true
    ;;

  mempool)
    echo "Mempool stats:"
    for i in 1 2 3; do
      port=$((8078 + i * 2))
      echo -n "  Node $i: "
      curl -s "http://localhost:$port/api/v1/mempool/stats" | python3 -c "
import sys, json
d = json.load(sys.stdin)
if 'data' in d and d['data']:
    print(f\"pending={d['data']['pending']} base_fee={d['data']['base_fee']}\")
else:
    print('no data')
" 2>/dev/null || echo "unreachable"
    done
    ;;

  *)
    echo "Cerulean Ledger — Cryptocurrency Testnet"
    echo ""
    echo "Usage: ./scripts/testnet.sh <command>"
    echo ""
    echo "Commands:"
    echo "  up                          Start 3-node testnet"
    echo "  down                        Stop and remove testnet"
    echo "  status                      Check node health"
    echo "  faucet <address>            Request test tokens"
    echo "  balance <address>           Check balance on all nodes"
    echo "  transfer <from> <to> <amt>  Send tokens"
    echo "  mempool                     Show mempool stats"
    ;;
esac
