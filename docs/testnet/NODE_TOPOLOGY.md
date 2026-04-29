# Cerulean Ledger — Testnet Node Topology

## Architecture

```
                    ┌──────────────────────┐
                    │   Public Internet     │
                    └──────┬───────┬────────┘
                           │       │
                    ┌──────▼──┐ ┌──▼──────┐
                    │ Sentry 1│ │ Sentry 2│   ← P2P gateway (exposed)
                    └────┬────┘ └────┬────┘
                         │           │
              ┌──────────┼───────────┼──────────┐
              │          │           │           │
         ┌────▼───┐ ┌───▼────┐ ┌───▼────┐      │
         │Valid. 1 │ │Valid. 2│ │Valid. 3│      │  ← BFT consensus (private)
         └────────┘ └────────┘ └────────┘      │
                                                │
              ┌─────────────┬───────────────────┘
              │             │
         ┌────▼───┐    ┌───▼─────┐
         │API Node│    │Explorer │  ← public HTTP (rate-limited)
         └────────┘    └─────────┘
              │
         ┌────▼──────┐
         │Monitoring │  ← private (Prometheus + Grafana)
         └───────────┘
```

## Node Roles

| Node | Role | Exposed | Port (API) | Port (P2P) |
|---|---|---|---|---|
| validator-1 | BFT validator + orderer | No | 8080 (internal) | 8081 (internal) |
| validator-2 | BFT validator | No | 8082 (internal) | 8083 (internal) |
| validator-3 | BFT validator | No | 8084 (internal) | 8085 (internal) |
| sentry-1 | P2P relay | P2P only | — | 30301 (public) |
| sentry-2 | P2P relay | P2P only | — | 30302 (public) |
| api-1 | Public API + faucet | HTTPS | 443 (public) | — |
| explorer-1 | Block explorer UI | HTTPS | 443 (public) | — |
| monitoring | Prometheus + Grafana | No | 9090/3000 (internal) | — |

## Firewall Rules

### Validators (private subnet)

```
ALLOW TCP 8081-8085 FROM sentry-1, sentry-2        # P2P from sentries
ALLOW TCP 8080-8085 FROM api-1, monitoring          # API + metrics scraping
DENY ALL FROM 0.0.0.0/0                             # No public access
```

### Sentries (DMZ)

```
ALLOW TCP 30301-30302 FROM 0.0.0.0/0               # Public P2P
ALLOW TCP 8081-8085 TO validator-1,2,3              # Forward to validators
DENY TCP 8080-8085 FROM 0.0.0.0/0                  # No direct API
```

### API Node

```
ALLOW TCP 443 FROM 0.0.0.0/0                       # Public HTTPS
RATE LIMIT 100 req/min per IP                       # nginx rate limiting
ALLOW TCP 8080 TO validator-1                       # Internal API forwarding
```

### Explorer

```
ALLOW TCP 443 FROM 0.0.0.0/0                       # Public HTTPS
ALLOW TCP 8080 TO api-1                             # API proxy
```

## Environment Variables

### Validators

```env
NETWORK_ID=cerulean-testnet
CONSENSUS_MODE=bft
NODE_ROLE=PeerAndOrderer
STORAGE_BACKEND=rocksdb
STORAGE_PATH=/data/rocksdb
ACL_MODE=permissive
BIND_ADDR=0.0.0.0
P2P_EXTERNAL_ADDRESS=validator-N:808X
BOOTSTRAP_NODES=sentry-1:30301,sentry-2:30302
SIGNING_ALGORITHM=ed25519
HASH_ALGORITHM=sha256
```

### Sentries

```env
NETWORK_ID=cerulean-testnet
NODE_ROLE=Peer
STORAGE_BACKEND=rocksdb
STORAGE_PATH=/data/rocksdb
BIND_ADDR=0.0.0.0
P2P_PORT=30301
BOOTSTRAP_NODES=validator-1:8081,validator-2:8083,validator-3:8085
```

### API Node

```env
NETWORK_ID=cerulean-testnet
NODE_ROLE=Peer
ACL_MODE=permissive
FAUCET_ENABLED=true
FAUCET_AMOUNT=1000
```

## Disk Requirements

| Node | Storage | Expected growth |
|---|---|---|
| Validator | 50 GB SSD | ~1 GB/month at 15s blocks |
| Sentry | 20 GB SSD | Minimal (relay only) |
| API | 50 GB SSD | Same as validator (full sync) |
| Monitoring | 20 GB SSD | Prometheus TSDB retention |
