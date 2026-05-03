# Cerulean Ledger DLT — Demo

Post-quantum blockchain DLT with ML-DSA-65 signing, WASM smart contracts, and multi-node consensus.

## Requirements

- Docker Desktop (4GB+ RAM allocated)
- curl, python3 (for test scripts)

## Quick Start

```bash
# Build and run 3-node network (first build: ~5 min)
docker compose -f docker-compose.demo.yml build
docker compose -f docker-compose.demo.yml up -d

# Wait for healthy
docker compose -f docker-compose.demo.yml ps
```

Nodes:
- node1: http://localhost:9600
- node2: http://localhost:9602
- node3: http://localhost:9604

## Test: Submit Transaction

```bash
curl -s -X POST http://localhost:9600/api/v1/gateway/submit \
  -H "Content-Type: application/json" \
  -d '{"chaincode_id":"notarize","transaction":{"id":"my-tx-001","input_did":"did:cerulean:alice","output_recipient":"did:cerulean:bob","amount":0}}'
```

Expected:
```json
{"status":"Success","data":{"tx_id":"my-tx-001","block_height":1,"valid":true}}
```

## Test: Query Transaction

```bash
curl -s http://localhost:9600/api/v1/tx/my-tx-001
```

Expected:
```json
{"status":"Success","data":{"id":"my-tx-001","block_height":1,"timestamp":...,"state":"committed"}}
```

## Test: Smart Contract

```bash
# Install kv_store contract
curl -s -X POST "http://localhost:9600/api/v1/chaincode/install?chaincode_id=kv_store&version=1.0" \
  --data-binary @contracts/kv_store.wat

# Write: set("demo", "hello")
curl -s -X POST "http://localhost:9600/api/v1/chaincode/kv_store/invoke?version=1.0" \
  -H "Content-Type: application/json" \
  -d '{"function":"set"}'

# Read: get("demo")
curl -s -X POST "http://localhost:9600/api/v1/chaincode/kv_store/invoke?version=1.0" \
  -H "Content-Type: application/json" \
  -d '{"function":"get"}'
```

Expected get result: `"result":"68656c6c6f"` (hex for "hello")

## Test: Block Explorer

```bash
# List blocks
curl -s http://localhost:9600/api/v1/blocks

# Get block by height
curl -s http://localhost:9600/api/v1/blocks/1

# Health
curl -s http://localhost:9600/api/v1/health
```

## Verify PQC Signing

```bash
curl -s http://localhost:9600/api/v1/blocks/1 | python3 -c "
import sys, json
d = json.load(sys.stdin)['data']
print(f'algorithm: {d[\"signature_algorithm\"]}')
print(f'sig_bytes: {len(d[\"signature\"])//2}')
"
```

Expected:
```
algorithm: MlDsa65
sig_bytes: 3309
```

## Multi-Node Consistency

```bash
./scripts/demo-consistency.sh
```

## Persistence

```bash
./scripts/demo-persistence.sh
```

## Local Development (without Docker)

```bash
ACL_MODE=permissive SIGNING_ALGORITHM=ml-dsa-65 cargo run --bin rust-bc -- 9600 9601
```

## Notes

- PQC ML-DSA-65 enabled (FIPS 204 compliant signing)
- No TLS in demo mode (ACL_MODE=permissive)
- RocksDB persistent storage with named Docker volumes
- Smart contracts execute via Wasmtime
