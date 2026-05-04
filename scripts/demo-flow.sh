#!/bin/bash
# Demo flow: send tx, mine block, check balances on all nodes.
# Prerequisites: ./scripts/start-testnet.sh running in another terminal.

set -e

BIN="./target/release/testnet_node"
BOB=bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb
NODE_A=127.0.0.1:3000
NODE_B=127.0.0.1:3001
NODE_C=127.0.0.1:3002

echo "=== Genesis balance ==="
$BIN show-balance --addr genesis --node $NODE_A

echo ""
echo "=== Sending 5000 NOTA to Bob ==="
$BIN send-tx --to $BOB --amount 5000 --fee 1 --nonce 0 --node $NODE_A
sleep 1

echo ""
echo "=== Mining block ==="
$BIN mine-block --node $NODE_A
sleep 1

echo ""
echo "=== Balances after block 1 ==="
echo "--- Node A ---"
$BIN show-balance --addr genesis --node $NODE_A
$BIN show-balance --addr $BOB --node $NODE_A

echo "--- Node B ---"
$BIN show-balance --addr genesis --node $NODE_B
$BIN show-balance --addr $BOB --node $NODE_B

echo "--- Node C ---"
$BIN show-balance --addr genesis --node $NODE_C
$BIN show-balance --addr $BOB --node $NODE_C

echo ""
echo "=== Sending 3000 NOTA to Bob (nonce=1) ==="
$BIN send-tx --to $BOB --amount 3000 --fee 1 --nonce 1 --node $NODE_A
sleep 1

echo ""
echo "=== Mining block 2 ==="
$BIN mine-block --node $NODE_A
sleep 1

echo ""
echo "=== Final balances ==="
echo "--- Node A ---"
$BIN show-balance --addr genesis --node $NODE_A
$BIN show-balance --addr $BOB --node $NODE_A

echo "--- Node B ---"
$BIN show-balance --addr genesis --node $NODE_B
$BIN show-balance --addr $BOB --node $NODE_B

echo "--- Node C ---"
$BIN show-balance --addr genesis --node $NODE_C
$BIN show-balance --addr $BOB --node $NODE_C

echo ""
echo "=== Expected: genesis=991998, bob=8000, nonce=2 on all nodes ==="
