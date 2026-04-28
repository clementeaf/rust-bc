# Operations

## CLI (bcctl)

Native Rust CLI that replaces `scripts/bcctl.sh`. Build with `cargo build --bin bcctl`.

```bash
# Network status
bcctl status
bcctl status --format json

# Target a specific node
bcctl --node node2 blocks
bcctl --node orderer1 peers

# Operations
bcctl mine                          # Mine a block
bcctl wallet-create                 # Create wallet
bcctl orgs                          # List organizations
bcctl channels                      # List channels
bcctl channel-create mychannel      # Create channel
bcctl verify                        # Verify chain integrity
bcctl consistency                   # Compare chain across peers
bcctl metrics                       # Prometheus metrics
bcctl env                           # Show network config

# Docker operations
bcctl logs node1 100                # Tail logs
bcctl restart node2                 # Restart node
bcctl restart all                   # Restart all nodes
```

## Docker deployment

### Start network

```bash
# Build images
docker compose build

# Start (3 peers + 1 orderer + Prometheus + Grafana)
docker compose up -d

# Check health
docker compose ps
bcctl status
```

### TLS certificates

```bash
cd deploy && ./generate-tls.sh
```

Generates per-node certs signed by a local CA. Nodes use mTLS for P2P and HTTPS for the API.

### Services

| Service | Host Port | Description |
|---------|-----------|-------------|
| node1 | 8080/8081 | peer + orderer (org1) |
| node2 | 8082/8083 | peer (org2) |
| node3 | 8084/8085 | peer (org1) |
| orderer1 | 8086/8087 | orderer |
| prometheus | 9090 | Metrics collection |
| grafana | 3000 | Dashboards (admin/admin) |

## Monitoring

### Prometheus

Available at `http://localhost:9090`. Scrapes `/metrics` from all nodes.

Key metrics:
- `blocks_total` — total blocks mined
- `transactions_total` — total transactions processed
- `peers_connected` — P2P peer count
- `endorsement_duration_seconds` — endorsement latency

### Grafana

Available at `http://localhost:3000` (admin/admin).

Import dashboards from `deploy/grafana/` or create custom ones from Prometheus data.

## E2E testing

```bash
# Run full test suite (requires Docker network running)
./scripts/e2e-test.sh

# Verbose mode (shows API responses)
./scripts/e2e-test.sh --verbose
```

56 assertions across 20 test categories:
1. Health checks
2. Organizations
3. Endorsement policies
4. Channels
5. Block mining + propagation
6. Transaction lifecycle
7. Private data collections
8. Discovery service
9. Gateway (endorse + order + commit)
10. Chain integrity
11. Observability
12. Store CRUD
13. Chaincode lifecycle
14. Channel config governance
15. Event polling
16. ACL enforcement
17. Channel membership
18. MVCC validity
19. MSP role enforcement
20. Crash recovery

## Troubleshooting

### Node won't start

```bash
# Check logs
docker compose logs node1 | tail -50

# Verify TLS certs exist
ls deploy/tls/

# Check port availability
lsof -i :8080
```

### Peers not connecting

```bash
# Verify network IDs match
bcctl --node node1 env
bcctl --node node2 env

# Check P2P connectivity
bcctl peers

# Verify bootstrap nodes
docker compose exec node2 env | grep BOOTSTRAP
```

### Chain inconsistency

```bash
# Compare all peers
bcctl consistency

# If inconsistent, the lagging peer should catch up via pull-sync
# (automatic, every 10 seconds)
```

### CouchDB world state

```bash
# Verify CouchDB is running
curl http://localhost:5984/

# Check world state database
curl http://localhost:5984/world_state/_all_docs?limit=5

# If connection fails, node falls back to MemoryWorldState (check logs)
```

## Node.js SDK

Located in `sdk/node/`. Install and use:

```bash
cd sdk/node
npm install
npm run build
```

```typescript
import { Gateway } from '@rust-bc/sdk';

const gw = new Gateway({
  url: 'https://localhost:8080',
  orgId: 'org1',
  insecure: true,
});

// Submit transaction
const result = await gw.submitTransaction('mycc', {
  id: 'tx-001',
  input_did: 'did:bc:alice',
  output_recipient: 'did:bc:bob',
  amount: 100,
});

// Subscribe to block events
const unsubscribe = gw.subscribeBlocks((event) => {
  console.log('New block:', event);
});
```

## Block Explorer

Located in `explorer/`. Start with:

```bash
cd explorer
npm install
npm run dev
```

Opens at `http://localhost:3001`. Features:
- Block list with real-time updates (WebSocket)
- Block detail with transaction list
- Organization list
- Node health status bar
