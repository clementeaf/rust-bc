# Getting Started

rust-bc is a Hyperledger Fabric-compatible blockchain node written in Rust. This guide walks you through running your first network and submitting a transaction.

## Prerequisites

- Rust nightly toolchain (`rustup default nightly`)
- Docker and Docker Compose (for multi-node network)
- `curl` and `jq` (for API interaction)

## Option A: Single node (development)

```bash
# Build
cargo build

# Run with defaults (API on 8080, P2P on 8081)
cargo run

# Verify
curl -sk https://localhost:8080/api/v1/health | jq
```

## Option B: Multi-node Docker network

```bash
# Generate TLS certificates
cd deploy && ./generate-tls.sh && cd ..

# Build and start the network (3 peers + 1 orderer)
docker compose build
docker compose up -d

# Verify all nodes are healthy
./scripts/bcctl.sh status
```

| Node | API Port | P2P Port | Role |
|------|----------|----------|------|
| node1 | 8080 | 8081 | peer + orderer (org1) |
| node2 | 8082 | 8083 | peer (org2) |
| node3 | 8084 | 8085 | peer (org1) |
| orderer1 | 8086 | 8087 | orderer |

## Your first transaction

### 1. Register organizations

```bash
curl -sk -X POST https://localhost:8080/api/v1/store/organizations \
  -H 'Content-Type: application/json' \
  -d '{
    "org_id": "org1",
    "msp_id": "Org1MSP",
    "admin_dids": ["did:bc:admin1"],
    "member_dids": ["did:bc:peer1"],
    "root_public_keys": [[1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]]
  }'
```

### 2. Submit a transaction through the gateway

```bash
curl -sk -X POST https://localhost:8080/api/v1/gateway/submit \
  -H 'Content-Type: application/json' \
  -H 'X-Org-Id: org1' \
  -d '{
    "chaincode_id": "mycc",
    "transaction": {
      "id": "tx-001",
      "input_did": "did:bc:alice",
      "output_recipient": "did:bc:bob",
      "amount": 100
    }
  }'
```

Response:

```json
{
  "status": "success",
  "data": {
    "tx_id": "tx-001",
    "block_height": 1,
    "valid": true
  }
}
```

### 3. Verify the block was committed

```bash
curl -sk https://localhost:8080/api/v1/store/blocks/1 | jq '.data'
```

## Install and run chaincode

```bash
# Install a Wasm chaincode module
curl -sk -X POST 'https://localhost:8080/api/v1/chaincode/install?chaincode_id=basic&version=1.0' \
  -H 'Content-Type: application/octet-stream' \
  --data-binary @my-chaincode.wasm

# Approve as org1
curl -sk -X POST 'https://localhost:8080/api/v1/chaincode/basic/approve?version=1.0' \
  -H 'X-Org-Id: org1'

# Commit
curl -sk -X POST 'https://localhost:8080/api/v1/chaincode/basic/commit?version=1.0'

# Simulate (read-only execution)
curl -sk -X POST 'https://localhost:8080/api/v1/chaincode/basic/simulate?version=1.0' \
  -H 'Content-Type: application/json' \
  -d '{"function": "run"}'
```

## Create a channel

```bash
# Create channel
curl -sk -X POST https://localhost:8080/api/v1/channels \
  -H 'Content-Type: application/json' \
  -d '{"channel_id": "mychannel"}'

# Submit to specific channel
curl -sk -X POST https://localhost:8080/api/v1/gateway/submit \
  -H 'X-Org-Id: org1' \
  -H 'X-Channel-Id: mychannel' \
  -H 'Content-Type: application/json' \
  -d '{
    "chaincode_id": "mycc",
    "channel_id": "mychannel",
    "transaction": { "id": "tx-ch-1", "input_did": "did:bc:alice", "output_recipient": "did:bc:bob", "amount": 50 }
  }'
```

## What's next

- [Architecture](architecture.md) — how the transaction flow works
- [API Reference](api-reference.md) — all endpoints
- [Configuration](configuration.md) — environment variables
- [Operations](operations.md) — monitoring, CLI, troubleshooting
