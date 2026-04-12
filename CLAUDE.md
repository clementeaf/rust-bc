# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
# Build
cargo build

# Run all tests
cargo test

# Run tests for a specific module
cargo test --lib storage
cargo test storage::adapters

# Run a single test by name
cargo test write_and_read_block_roundtrip

# Run with stdout (useful for debugging)
cargo test -- --nocapture

# Start the server (default: API 8080, P2P 8081)
cargo run
cargo run -- 8080 8081

# Lint
cargo clippy -- -D warnings
cargo fmt
```

## Architecture

The project is a blockchain node with an HTTP API. Two parallel storage systems coexist:

### Legacy system (`src/blockchain.rs`, `src/block_storage.rs`, `src/models.rs`)
The original in-memory `Blockchain` struct plus a file-backed `BlockStorage`. Loaded at startup and kept in `AppState`. Not integrated with the new storage layer.

### New storage layer (`src/storage/`)
Clean trait-based persistence introduced in Fases I–VI:
- `traits.rs` — `BlockStore` trait + data types (`Block`, `Transaction`, `IdentityRecord`, `Credential`). A blanket `impl<T: BlockStore> BlockStore for Arc<T>` lets `Arc<MemoryStore>` be used as `Box<dyn BlockStore>`.
- `memory.rs` — `MemoryStore`: HashMap-backed, used as default and in tests.
- `adapters.rs` — `RocksDbBlockStore`: Column Families (`blocks`, `transactions`, `identities`, `credentials`, `meta`, `tx_by_block`). Secondary index `tx_by_block` uses key `{012-padded-height}:{tx_id}` for prefix scans.
- `errors.rs` — `StorageError` enum.
- `comprehensive_tests.rs` — cross-store integration tests.

Storage backend is selected at runtime via `STORAGE_BACKEND=rocksdb` (path from `STORAGE_PATH`) or defaults to `MemoryStore`. Lives in `AppState.store: Option<Arc<dyn BlockStore>>`.

### HTTP API (`src/api/` + `src/api_legacy.rs`)
Actix-Web 4. A single `/api/v1` scope is built in `api_legacy.rs::config_routes` and extended by `ApiRoutes::register()` from `api/routes.rs`.

**Routing architecture:**
- `api_legacy.rs` creates the `/api/v1` scope with legacy `.route()` handlers (wallets, contracts, staking, airdrop, etc.) and flat utility routes (`/health`, `/version`, `/openapi.json`).
- `ApiRoutes::register()` appends sub-scoped scaffold services (store, channels, chaincode, events, etc.) into the same scope.
- **Important:** `web::scope("")` (empty sub-scopes) are invisible to Actix when the parent scope uses `.route()`. All scaffold handlers are registered directly with `.service()` in `register()`. Only sub-scopes with a real path prefix (e.g. `/store/blocks`, `/chain`) work as nested scopes. Flat routes like `/health` use `.route()` in the legacy scope.

Handlers split by domain in `handlers/`:
- `blocks.rs` — legacy chain blocks + store-backed block endpoints
- `transactions.rs` — mempool endpoints + store-backed transaction endpoints
- `identity.rs`, `credentials.rs` — store-backed DID/credential endpoints

Response envelope: `ApiResponse<T>` in `errors.rs` — always `{ status, status_code, message, data?, error?, timestamp, trace_id }`.

### AppState (`src/app_state.rs`)
Central shared state. Legacy `blockchain: Arc<Mutex<Blockchain>>` and new `store` coexist independently.

Services initialized at startup (all use in-memory backends by default):
- `org_registry`, `policy_store` — endorsement infrastructure
- `discovery_service` — peer registration and endorsement plans
- `gateway` — endorse → order → commit pipeline
- `private_data_store`, `collection_registry` — private data collections
- `chaincode_package_store`, `chaincode_definition_store` — chaincode lifecycle

### Other subsystems
- `src/consensus/` — DAG, fork choice, validator scheduling
- `src/identity/` — DID + key management + pluggable signing (`SigningProvider` trait with Ed25519 and ML-DSA-65 implementations)
- `src/tls.rs`, `src/pki.rs` — mutual TLS, certificate provisioning
- `src/network.rs` — P2P node, peer discovery

## Environment variables

| Variable | Default | Description |
|---|---|---|
| `API_PORT` | 8080 | HTTP API port |
| `P2P_PORT` | 8081 | P2P gossip port |
| `BIND_ADDR` | `127.0.0.1` | HTTP listen address (`0.0.0.0` in Docker) |
| `P2P_EXTERNAL_ADDRESS` | — | Announce address for P2P (e.g. `node1:8081`) |
| `DIFFICULTY` | 1 | Mining difficulty |
| `STORAGE_BACKEND` | *(memory)* | Set to `rocksdb` to enable RocksDB |
| `STORAGE_PATH` | `./data/rocksdb` | RocksDB data directory |
| `NETWORK_ID` | `mainnet` | Network identifier |
| `TLS_CERT_PATH` | — | Node TLS certificate (enables HTTPS + P2P TLS) |
| `TLS_KEY_PATH` | — | Node TLS private key |
| `TLS_CA_CERT_PATH` | — | CA certificate for peer verification |
| `BOOTSTRAP_NODES` | — | Comma-separated `host:port` list |
| `SEED_NODES` | — | Always-tried peer list |
| `ACL_MODE` | *(strict)* | Set to `permissive` to allow all requests without identity |
| `JWT_SECRET` | `change-me-in-production` | Secret for JWT token signing |
| `P2P_RESPONSE_BUFFER_BYTES` | 262144 | Buffer size for `send_and_wait` responses (256 KB) |
| `P2P_HANDLER_BUFFER_BYTES` | 65536 | Buffer size for per-connection message handler (64 KB) |
| `P2P_SYNC_BUFFER_BYTES` | 4194304 | Buffer size for pull-based state sync (4 MB) |
| `SIGNING_ALGORITHM` | *(ed25519)* | `ed25519` or `ml-dsa-65` — selects the node's signing provider |

## Global Claude configuration (`~/.claude/`)

### Active rules for this project

Rules are layered: `~/.claude/rules/common/` sets universal defaults, `~/.claude/rules/rust/` overrides them for Rust-specific idioms. Both are always active.

Key rules that affect day-to-day work here:

| Rule file | What it enforces |
|---|---|
| `rust/coding-style.md` | `rustfmt` + `clippy -D warnings`; borrow over clone; `&str` over `String` in params; `thiserror` for library errors |
| `rust/testing.md` | `#[cfg(test)]` unit tests in same file; `tempfile` for RocksDB fixtures; 80%+ coverage via `cargo-llvm-cov` |
| `rust/patterns.md` | Repository pattern behind traits; newtype for type safety; builder for complex structs |
| `rust/security.md` | `// SAFETY:` comment mandatory on every `unsafe` block; parameterized queries; no secrets in source |
| `common/development-workflow.md` | Plan → TDD → code-review pipeline; `gh search` before writing new code |

### Agents to use in this project

| Agent | When to invoke |
|---|---|
| `rust-reviewer` | After any Rust code change |
| `build-error-resolver` | When `cargo build` fails |
| `tdd-guide` | Before implementing a new feature or fix |
| `code-reviewer` | General review pass |
| `security-reviewer` | Before committing anything touching crypto, TLS, or auth |

### Relevant skills

- `rust-patterns` — ownership, traits, error handling, concurrency idioms
- `rust-testing` — property-based tests (proptest), mockall, Criterion benchmarks
- `rust-review` — comprehensive review checklist
- `rust-build` — incremental build error resolution

### Project memory (`~/.claude/projects/…/memory/`)

Persisted context that carries across sessions:

- `project_storage_progress.md` — Fases I–VI del storage layer completadas; patrón de índices secundarios (CF propia + prefix scan)
- `project_tls_roadmap.md` — Fases A–C de TLS completadas; próximo paso sería rotación en caliente o QTSP
- `feedback_microtasks.md` — **Trabajar en microtareas: una tarea por iteración, sin agentes de planning, sin refactoring colateral**

## Project Claude configuration (`.claude/`)

`.claude/settings.local.json` pre-approves the following Bash commands so they run without prompting:

- `cargo build:*`, `cargo test:*`, `cargo fetch:*`, `cargo search:*`, `cargo tree:*`, `cargo metadata:*`
- `openssl req:*`
- `ls` on `*.md` files at the repo root and `docs/`
- `wait`

Any other shell command (e.g. `rm`, `git push`, `cargo publish`) will still prompt for approval.

## Docker

```bash
# Build all node images
docker compose build

# Start the network (3 peers + 1 orderer + Prometheus + Grafana)
docker compose up -d

# Check health
docker compose ps

# Test from host
curl -sk https://localhost:8080/api/v1/health

# Regenerate TLS certificates
cd deploy && ./generate-tls.sh
```

| Service | Host port | Role |
|---|---|---|
| node1 | 8080 (API), 8081 (P2P) | peer + orderer (org1) |
| node2 | 8082 (API), 8083 (P2P) | peer (org2) |
| node3 | 8084 (API), 8085 (P2P) | peer (org1) |
| orderer1 | 8086 (API), 8087 (P2P) | orderer (Raft ID 1) |
| orderer2 | 8088 (API), 8089 (P2P) | orderer (Raft ID 2) |
| orderer3 | 8090 (API), 8091 (P2P) | orderer (Raft ID 3) |
| prometheus | 9090 | Metrics |
| grafana | 3000 | Dashboards (admin/admin) |

## Operator tooling

```bash
# Operator CLI
./scripts/bcctl.sh status          # Health, blocks, peers for all nodes
./scripts/bcctl.sh consistency     # Compare chain tips across peers
./scripts/bcctl.sh mine            # Create wallet + mine a block
./scripts/bcctl.sh orgs            # List registered organizations
./scripts/bcctl.sh logs node1 100  # Tail container logs

# E2E test suite (requires running Docker network)
./scripts/e2e-test.sh              # 71 assertions across 20 categories
./scripts/e2e-test.sh --verbose    # Show full API responses
```

## Key conventions

- The `nightly` feature `#![feature(unsigned_is_multiple_of)]` is required; use `cargo +nightly` if the toolchain doesn't default to nightly.
- Block keys in RocksDB are zero-padded to 12 digits so lexicographic order matches numeric order.
- Secondary index keys use the same zero-padded prefix: `{:012}:{id}`, enabling cheap prefix range scans without a full table scan.
- `tempfile::TempDir` is the standard test helper for RocksDB tests — the directory is cleaned up on drop.
- Signature fields across all structs are `Vec<u8>` (not `[u8; 64]`) to support both Ed25519 (64 bytes) and ML-DSA-65 (3309 bytes). Serialized as hex strings via `vec_hex` serde helpers.
