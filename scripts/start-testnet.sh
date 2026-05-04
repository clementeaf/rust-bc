#!/bin/bash
# Start a 3-node testnet on localhost.
# Usage: ./scripts/start-testnet.sh
# Stop:  pkill -f "testnet_node node"

set -e

BIN="./target/release/testnet_node"

if [ ! -f "$BIN" ]; then
    echo "Binary not found. Building..."
    cargo build --release --bin testnet_node
fi

echo "Starting node A (:3000)..."
$BIN node --port 3000 --peers 127.0.0.1:3001,127.0.0.1:3002 &
PID_A=$!
sleep 1

echo "Starting node B (:3001)..."
$BIN node --port 3001 --peers 127.0.0.1:3000 &
PID_B=$!
sleep 1

echo "Starting node C (:3002)..."
$BIN node --port 3002 --peers 127.0.0.1:3000,127.0.0.1:3001 &
PID_C=$!
sleep 1

echo ""
echo "Testnet running:"
echo "  Node A: 127.0.0.1:3000 (PID $PID_A)"
echo "  Node B: 127.0.0.1:3001 (PID $PID_B)"
echo "  Node C: 127.0.0.1:3002 (PID $PID_C)"
echo ""
echo "Stop with: pkill -f 'testnet_node node'"
echo ""

wait
