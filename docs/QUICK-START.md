# Quick Start

Get a 4-node blockchain network running in under 5 minutes.

## Prerequisites

- Docker and Docker Compose
- curl (for testing)
- Node.js 18+ (optional, for the JS SDK)

## 1. Clone and generate TLS certificates

```bash
git clone https://github.com/clementeaf/rust-bc.git
cd rust-bc
cd deploy && bash generate-tls.sh && cd ..
```

## 2. Start the network

```bash
docker compose build
docker compose up -d node1 node2 node3 orderer1
```

Wait ~20 seconds for nodes to start and discover each other.

## 3. Verify health

```bash
curl -sk https://localhost:8080/api/v1/health | jq .
```

Expected:

```json
{
  "status": "Success",
  "status_code": 200,
  "data": {
    "status": "healthy",
    "uptime_seconds": 25,
    "blockchain": { "height": 1, "last_block_hash": "...", "validators_count": 0 },
    "checks": { "storage": "ok", "peers": "ok (3 connected)", "ordering": "ok" }
  }
}
```

## 4. Create a wallet and mine a block

```bash
# Create wallet
WALLET=$(curl -sk https://localhost:8080/api/v1/wallets/create -X POST | jq -r '.data.address')
echo "Wallet: $WALLET"

# Mine a block
curl -sk https://localhost:8080/api/v1/mine -X POST \
  -H 'Content-Type: application/json' \
  -d "{\"miner_address\": \"$WALLET\"}" | jq .
```

## 5. Submit a transaction via the gateway

```bash
curl -sk https://localhost:8080/api/v1/gateway/submit -X POST \
  -H 'Content-Type: application/json' \
  -d '{
    "chaincode_id": "mycc",
    "channel_id": "",
    "transaction": {
      "id": "tx-001",
      "input_did": "did:bc:alice",
      "output_recipient": "did:bc:bob",
      "amount": 100
    }
  }' | jq .
```

Response:

```json
{
  "data": { "tx_id": "tx-001", "block_height": 2, "valid": true }
}
```

## 6. Verify multi-node propagation

```bash
# Check all nodes have the same chain height
for port in 8080 8082 8084; do
  echo -n "Port $port: "
  curl -sk "https://localhost:$port/api/v1/chain/info" | jq -r '.data.block_count'
done
```

## 7. Use the JS SDK (optional)

```bash
cd sdk-js && npm install && npm run build && cd ..
```

```typescript
import { BlockchainClient } from '@rust-bc/sdk';

const client = new BlockchainClient({
  baseUrl: 'https://localhost:8080/api/v1',
});

const health = await client.health();
console.log(health.status); // "healthy"

const result = await client.submitTransaction('mycc', '', {
  id: 'tx-002',
  inputDid: 'did:bc:alice',
  outputRecipient: 'did:bc:bob',
  amount: 50,
});
console.log(result.block_height); // 3
```

## 8. Start monitoring (optional)

```bash
docker compose up -d prometheus grafana
```

- Prometheus: http://localhost:9090
- Grafana: http://localhost:3000 (admin/admin)

## Next steps

- [API Reference](API-REFERENCE.md) — all 68 endpoints with examples
- [Deployment Guide](DEPLOYMENT.md) — production configuration
- [Operator CLI](../scripts/bcctl.sh) — `./scripts/bcctl.sh status`
