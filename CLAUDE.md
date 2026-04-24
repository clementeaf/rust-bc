# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## MANDATORY: Pre-commit quality gate

**A commit CANNOT be made unless ALL THREE pass locally. No exceptions. No "fix in next commit". Workflow failure is unacceptable.**

```bash
# Run ALL THREE before EVERY commit. All must exit 0.
cargo fmt --check
cargo clippy -- -D warnings
cargo test --lib
```

If any integration test file was modified, also run it explicitly:
```bash
cargo test --test <test_name>
```

If any gate fails, fix the issue FIRST, re-run ALL THREE, and only then commit. This is not optional.

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

# Run BFT E2E tests (adversarial scenarios)
cargo test --test bft_e2e

# Run with stdout (useful for debugging)
cargo test -- --nocapture

# Start the server (default: API 8080, P2P 8081)
cargo run --bin rust-bc
cargo run --bin rust-bc -- 8080 8081

# Interactive demo (no Docker needed)
./scripts/try-it.sh

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
- `adapters.rs` — `RocksDbBlockStore`: Column Families (`blocks`, `transactions`, `identities`, `credentials`, `meta`, `tx_by_block`). On open, static names are merged with `RocksDB::list_cf` so extra families on disk (e.g. `private_*`) are opened. Secondary index `tx_by_block` uses key `{012-padded-height}:{tx_id}` for prefix scans.
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

**Security layers:**
- All mutation endpoints (legacy + scaffold) call `enforce_acl()` from `api/errors.rs`. Strict mode (default) denies requests without TLS identity or `X-Org-Id`/`X-Msp-Role` headers.
- `mine_block` additionally verifies `miner_address` belongs to a registered wallet.
- `RateLimitMiddleware` (sliding window) wraps all routes except `/health`.
- Chaincode install computes SHA-256 of Wasm bytes; optional `expected_hash` query param for supply-chain verification.
- `jwt_secret` is loaded but reserved for future use — mTLS + ACL is the active auth mechanism.
- See `docs/SECURITY-AUDIT.md` for the full audit and remediation status.

### AppState (`src/app_state.rs`)
Central shared state. Legacy `blockchain: Arc<Mutex<Blockchain>>` and new `store` coexist independently.

Services initialized at startup (all use in-memory backends by default):
- `org_registry`, `policy_store` — endorsement infrastructure
- `discovery_service` — peer registration and endorsement plans
- `gateway` — endorse → order → commit pipeline; `commit_block_parallel()` for wave-parallel batch execution with MVCC
- `private_data_store`, `collection_registry` — private data collections
- `chaincode_package_store`, `chaincode_definition_store` — chaincode lifecycle

### Other subsystems
- `src/consensus/` — DAG, fork choice, validator scheduling, HotStuff-inspired BFT layer (`bft/`), `ConsensusBackend` trait for Raft/BFT selection (`CONSENSUS_MODE` env var), DPoS validator selection (`dpos.rs`: stake-weighted committee, proportional leader rotation). `ConsensusEngine` supports BFT mode via `with_bft()`. `DagBlock` carries optional `commit_qc`.
- `src/identity/` — DID + key management + pluggable signing (`SigningProvider` trait with Ed25519 and ML-DSA-65 implementations)
- `src/tls.rs`, `src/pki.rs` — mutual TLS, certificate provisioning
- `src/network/mod.rs` — P2P node, peer discovery, BFT message types (`BftProposal`, `BftVote`, `BftQuorumCertificate`, `BftViewChange`)
- `src/transaction/parallel.rs` — conflict detector (RAW/WAW/WAR) + wave scheduler; groups non-conflicting txs for concurrent execution
- `src/transaction/executor.rs` — wave-parallel block executor: MVCC validate per wave, apply writes in deterministic order, `to_legacy_results()` adapter
- `src/tokenomics/economics.rs` — NOTA supply cap (100M), halving rewards, capped issuance, 80/20 fee burn/proposer split, EIP-1559 dynamic base fee, epoch-based `process_block()` state machine
- `src/tokenomics/storage_deposit.rs` — `DepositLedger`: lock tokens proportional to data size on state writes, refund on delete, delta on update
- `src/bridge/` — cross-chain bridge: chain registry, message envelope, escrow vault (lock/release outbound, mint/burn inbound), Merkle inclusion proof verifier, `BridgeEngine` with replay protection, `Relayer` with job queue, batch processing, and retry logic
- `src/governance/` — on-chain governance: typed `ParamRegistry` with protocol defaults, `ProposalStore` (submit→vote→pass→timelock→execute lifecycle), stake-weighted `VoteStore` (Yes/No/Abstain, quorum + threshold checks). HTTP API: 7 endpoints under `/api/v1/governance/` (params, proposals CRUD, vote, tally). AppState fields: `proposal_store`, `vote_store`, `param_registry`.
- `src/light_client/` — compact `BlockHeader` chain with BFT QC verification, `LightClient` for state proof verification via Merkle proofs against synced headers. Enables IoT/mobile participation without full node.
- `src/transaction/executor.rs` — `execute_block_concurrent()` async tokio executor for true intra-wave parallelism
- `src/testnet/` — `GenesisConfig` (testnet/devnet/mainnet presets with validation), `Faucet` (rate-limited token drip with cooldown and depletion)
- `src/evm_compat/` — Full EVM execution via revm (`executor.rs`), Solidity ABI encoding/decoding (`abi.rs`), precompile interface with gas metering (`precompile.rs`), DID-to-address derivation. HTTP endpoints: `POST /evm/deploy`, `POST /evm/call`, `POST /evm/static-call`, `GET /evm/contracts`
- `src/channel/store.rs` — `ChannelStore`: per-channel isolated world state and block ledger (Fabric-compatible channel isolation)
- `src/chaincode/upgrade.rs` — `UpgradeManager`: multi-org approval lifecycle for chaincode version upgrades (propose→approve→commit)

### Block explorer — Cerulean Ledger UI

| Path | Stack | Notes |
|---|---|---|
| `block-explorer-vite/` | Vite + React + Tailwind | `npm install` / `npm run dev`; proxies `/api` to the node. |

Branded as **Cerulean Ledger**. Full Spanish UI. DID prefix: `did:cerulean:`.

Routes:
- `/` — Landing page (full-width, no sidebar)
- `/dashboard` — Network stats, blocks, hub cards
- `/demo` — 5-step credential verification demo (flagship)
- All other pages use sidebar Layout

Key structure:
- `src/pages/Landing.tsx` — hero + pillars + tech specs + CTAs
- `src/lib/format.ts` — shared formatters (`timeAgo`, `shortHash`, `fmtDate`, etc.)
- `src/lib/routes.ts` — route config with lazy loading
- `src/lib/api.ts` — API client and types

Not required to run the node.

### Electronic voting — Cerulean Voto

| Path | Stack | Notes |
|---|---|---|
| `cerulean-voto/` | Vite + React + Tailwind | `npm install` / `npm run dev`; proxies `/api` to the node on port 5174. |

Standalone voting frontend built on the same patterns as `block-explorer-vite/`. Consumes the existing governance and identity APIs.

Routes:
- `/` — Landing page (hero + 3 pillars, standalone)
- `/dashboard` — Active/closed election stats, tally bars
- `/elections` — Create elections, history table
- `/vote` — Emit vote (Yes/No/Abstain) with DID identity
- `/results` — Public audit with percentage bars, quorum indicators
- `/voters` — Register and lookup voters via DID

Key structure:
- `src/lib/api.ts` — governance + identity API client
- `src/lib/routes.ts` — 5 lazy-loaded routes
- `src/lib/format.ts` — shared formatters (`timeAgo`, `pct`, `fmtDateTime`)
- `src/components/Layout.tsx` — header + sidebar + footer (same pattern as block explorer)

Not required to run the node.

### Presentation materials (`docs/`)

Documentation prepared for the Blockchain Chamber of Chile:

| Document | Purpose |
|---|---|
| `PRESENTACION.md` | Full platform overview for the board |
| `FAQ.md` | ~40 questions by audience (board, enterprise, technical, regulators) |
| `PITCH.md` | Talking points, one-liners, objection handling |
| `ONE-PAGER-CAMARA.md` | One-page executive summary |
| `DEMO-SCRIPT.md` | 5-minute live demo script with timing and commands |
| `TESSERACT.md` | Standalone Tesseract explanation (geometric consensus prototype) |
| `PUBLIC-ROADMAP.md` | Public roadmap with concrete dates (Q2 2026 – H2 2027) |
| `PQC-TEST-EVIDENCE.md` | PQC test inventory evidence |
| `COTIZACION-VOTO-ELECTRONICO.md` | E-voting quotation (md + html + pdf) |

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
| `JWT_SECRET` | `change-me-in-production` | Reserved for future JWT middleware (not used for auth yet) |
| `CHECKPOINT_HMAC_SECRET` | `checkpoint-dev-secret` | HMAC key for checkpoint file integrity verification |
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
