# rust-bc

[![CI](https://github.com/clementeaf/rust-bc/actions/workflows/ci.yml/badge.svg)](https://github.com/clementeaf/rust-bc/actions/workflows/ci.yml)

A permissioned blockchain platform built in Rust with post-quantum cryptography.

Follows the Hyperledger Fabric architecture (execute-order-validate) with channels, private data collections, endorsement policies, Raft consensus, and WebAssembly smart contracts.

## Quick start

```bash
cargo build
cargo test

# Start a single node
cargo run

# Start a 6-node network (3 peers + 3 Raft orderers + Prometheus + Grafana)
cd deploy && ./generate-tls.sh && cd ..
docker compose up -d
```

## Features

- **Channels** — Isolated ledgers per business network
- **Private data** — Confidential data shared only between authorized organizations
- **Endorsement policies** — AnyOf, AllOf, NOutOf with per-key overrides
- **Raft consensus** — 3-node crash-fault-tolerant ordering with persistent log
- **WebAssembly chaincode** — Sandboxed smart contracts with state CRUD, events, cross-chaincode calls
- **Post-quantum crypto** — ML-DSA-65 (FIPS 204, NIST Level 3) alongside Ed25519
- **Mutual TLS** — X.509 MSP with certificate-based identity and role inference
- **REST API** — 68 endpoints, OpenAPI 3.0, Swagger UI at `/swagger`
- **JS/TS SDK** — `npm install rust-bc-sdk`
- **Block explorer** — Next.js web UI

## Architecture

```
Client --> Gateway --> Endorsing Peers --> Ordering Service (Raft) --> Commit
                                              |
                                         3 orderers
                                      (persistent log)
```

| Component | Description |
|---|---|
| Peer nodes | Execute chaincode, maintain ledger, endorse proposals |
| Ordering service | 3-node Raft cluster, block creation, crash fault tolerance |
| Gateway | Endorse-order-commit pipeline |
| Channels | Isolated ledgers shared between subsets of organizations |
| Chaincode | Smart contracts in Rust, compiled to WebAssembly |
| World state | Key-value state backed by RocksDB or CouchDB |

## Docker network

| Service | Ports | Role |
|---|---|---|
| node1 | 8080, 8081 | Peer (org1) |
| node2 | 8082, 8083 | Peer (org2) |
| node3 | 8084, 8085 | Peer (org1) |
| orderer1 | 8086, 8087 | Orderer (Raft ID 1) |
| orderer2 | 8088, 8089 | Orderer (Raft ID 2) |
| orderer3 | 8090, 8091 | Orderer (Raft ID 3) |
| prometheus | 9090 | Metrics |
| grafana | 3000 | Dashboards |

## Post-quantum cryptography

```bash
SIGNING_ALGORITHM=ml-dsa-65 cargo run
```

ML-DSA-65 (FIPS 204) and Ed25519 coexist in the same network. Nodes auto-detect signature type. See [PQC-ENTERPRISE.md](docs/PQC-ENTERPRISE.md).

## Performance

Measured with Criterion on Apple M-series:

| Operation | Throughput |
|---|---|
| Ordering (submit + cut) | 23M tx/s |
| Endorsement validation (Ed25519) | 45K/s |
| RocksDB block writes | 104K blocks/s |
| Estimated E2E (3-node Raft LAN) | 5K-15K tx/s |

See [BENCHMARKS-FULL.md](docs/BENCHMARKS-FULL.md).

## Documentation

| Document | Description |
|---|---|
| [Quick Start](docs/QUICK-START.md) | Zero to first transaction in < 5 min |
| [API Reference](docs/API-REFERENCE.md) | 68 endpoints with curl examples |
| [Deployment](docs/DEPLOYMENT.md) | Production config and security checklist |
| [Enterprise Overview](docs/ENTERPRISE.md) | Platform overview for enterprise evaluation |
| [Chaincode SDK](chaincode-sdk/README.md) | Write smart contracts in Rust |
| [Certification Roadmap](docs/CERTIFICATION-ROADMAP.md) | FIPS 140-3, SOC 2, ISO 27001 readiness |
| [Compliance Framework](docs/COMPLIANCE-FRAMEWORK.md) | SOC 2 / ISO 27001 control mapping |

## Tests

```bash
cargo test              # 992 unit + integration tests
./scripts/e2e-test.sh   # 42 E2E assertions on Docker network
```

## License

[MIT](LICENSE)
