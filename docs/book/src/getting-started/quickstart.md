# Quick Start

## Prerequisites

- Rust nightly toolchain (`rustup default nightly`)
- Git

## Build

```bash
git clone https://github.com/clementeaf/rust-bc.git
cd rust-bc
cargo build --release
```

## Start a Node

```bash
# Start with in-memory storage (for development)
cargo run --release

# Start with RocksDB (for persistence)
STORAGE_BACKEND=rocksdb STORAGE_PATH=./data cargo run --release
```

The node exposes:
- **HTTP API**: `http://127.0.0.1:8080/api/v1`
- **P2P**: `127.0.0.1:8081`

## Verify It Works

```bash
# Health check
curl http://localhost:8080/api/v1/health

# Create a wallet
curl -X POST http://localhost:8080/api/v1/wallets

# Mine a block
curl -X POST http://localhost:8080/api/v1/mine \
  -H "Content-Type: application/json" \
  -d '{"miner_address": "<your-wallet-address>"}'
```

## Interactive Demo

```bash
./scripts/try-it.sh
```

## Docker Network (3 peers + orderers)

```bash
docker compose up -d
docker compose ps
curl -sk https://localhost:8080/api/v1/health
```

## Next Steps

- [Configuration](./configuration.md) — environment variables and tuning
- [Your First dApp](./first-dapp.md) — deploy a Wasm smart contract
