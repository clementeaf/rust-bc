# Configuration

All configuration is via environment variables. No config files required.

## Core

| Variable | Default | Description |
|----------|---------|-------------|
| `API_PORT` | `8080` | HTTP API port |
| `P2P_PORT` | `8081` | P2P gossip port |
| `BIND_ADDR` | `127.0.0.1` | Listen address (`0.0.0.0` for Docker) |
| `NETWORK_ID` | `mainnet` | Network identifier |
| `DIFFICULTY` | `1` | Mining difficulty |

## Storage

| Variable | Default | Description |
|----------|---------|-------------|
| `STORAGE_BACKEND` | `memory` | `memory` or `rocksdb` |
| `STORAGE_PATH` | `./data/rocksdb` | RocksDB data directory |

## Consensus

| Variable | Default | Description |
|----------|---------|-------------|
| `CONSENSUS_MODE` | `raft` | `raft` (CFT) or `bft` (Byzantine) |
| `NODE_ROLE` | `peerandorderer` | `peer`, `orderer`, or `peerandorderer` |

## Security

| Variable | Default | Description |
|----------|---------|-------------|
| `TLS_CERT_PATH` | — | Node TLS certificate |
| `TLS_KEY_PATH` | — | Node TLS private key |
| `TLS_CA_CERT_PATH` | — | CA certificate for peer verification |
| `ACL_MODE` | `strict` | `strict` or `permissive` |
| `SIGNING_ALGORITHM` | `ed25519` | `ed25519` or `ml-dsa-65` (post-quantum) |

## P2P

| Variable | Default | Description |
|----------|---------|-------------|
| `BOOTSTRAP_NODES` | — | Comma-separated `host:port` list |
| `SEED_NODES` | — | Always-tried peer list |
| `P2P_EXTERNAL_ADDRESS` | — | Announce address for NAT traversal |
