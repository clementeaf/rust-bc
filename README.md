# Cerulean Ledger

[![CI](https://github.com/clementeaf/cerulean-ledger/actions/workflows/ci.yml/badge.svg)](https://github.com/clementeaf/cerulean-ledger/actions/workflows/ci.yml)

A permissioned blockchain platform built in Rust with post-quantum cryptography.

Follows the Hyperledger Fabric architecture (execute-order-validate) with channels, private data collections, endorsement policies, BFT/DPoS consensus, and WebAssembly smart contracts.

## Quick start

```bash
cargo build
cargo test

# Start a single node
cargo run

# Interactive demo (no Docker needed)
./scripts/try-it.sh

# Start a 6-node network (3 peers + 3 orderers + Prometheus + Grafana)
cd deploy && ./generate-tls.sh && cd ..
docker compose up -d
```

## Features

- **Channels** — Isolated ledgers per business network
- **Private data** — Confidential data shared only between authorized organizations
- **Endorsement policies** — AnyOf, AllOf, NOutOf with per-key overrides
- **BFT/DPoS consensus** — HotStuff-inspired BFT + Delegated Proof of Stake
- **WebAssembly chaincode** — Sandboxed smart contracts with state CRUD, events, cross-chaincode calls
- **Post-quantum crypto** — ML-DSA-65 (FIPS 204), ML-KEM-768 (FIPS 203), SHA3-256 (FIPS 202)
- **Mutual TLS** — X.509 MSP with certificate-based identity and role inference
- **EVM compatibility** — Deploy and call Solidity contracts via revm
- **On-chain governance** — Proposals, stake-weighted voting, parameter registry
- **REST API** — 66+ endpoints, OpenAPI 3.0
- **DID/VC** — W3C DID Resolution, Verifiable Credentials, JSON-LD interop

## Architecture

```
Client → Gateway → Endorsing Peers → Ordering Service → Commit
```

| Component | Description |
|---|---|
| Peer nodes | Execute chaincode, maintain ledger, endorse proposals |
| Ordering service | Raft or BFT consensus, block creation |
| Gateway | Endorse-order-commit pipeline with wave-parallel execution |
| Channels | Isolated ledgers shared between subsets of organizations |
| Chaincode | Smart contracts in Rust/Wasm with static validation |
| World state | Key-value state backed by RocksDB |

## Post-quantum cryptography

```bash
SIGNING_ALGORITHM=ml-dsa-65 cargo run
```

ML-DSA-65 and Ed25519 coexist. Dual-signing for migration. See [`docs/compliance/`](docs/compliance/).

## Related repositories

- [cerulean-explorer](https://github.com/clementeaf/cerulean-explorer) — Block explorer (Vite + React)
- [cerulean-voto](https://github.com/clementeaf/cerulean-voto) — Electronic voting platform
- [cerulean-sdks](https://github.com/clementeaf/cerulean-sdks) — TypeScript and Python clients

## Tests

```bash
cargo test              # 1698 unit + integration tests
./scripts/e2e-test.sh   # 71 E2E assertions on Docker network
```

## Documentation

See [`docs/`](docs/) for API reference, deployment guides, architecture, compliance, and more.

## License

[MIT](LICENSE)
