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
- `pin.rs` — PIN generation and verification endpoints

Response envelope: `ApiResponse<T>` in `errors.rs` — always `{ status, status_code, message, data?, error?, timestamp, trace_id }`.

**Security layers:**
- All mutation endpoints (legacy + scaffold) call `enforce_acl()` from `api/errors.rs`. Strict mode (default) denies requests without TLS identity or `X-Org-Id`/`X-Msp-Role` headers.
- `mine_block` additionally verifies `miner_address` belongs to a registered wallet.
- `RateLimitMiddleware` (sliding window) wraps all routes except `/health`.
- Chaincode install computes SHA-256 of Wasm bytes; optional `expected_hash` query param for supply-chain verification.
- `jwt_secret` is loaded but reserved for future use — mTLS + ACL is the active auth mechanism.
- See `docs/architecture/security/SECURITY-AUDIT.md` for the full audit and remediation status.

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
- `src/network/testnet/` — Minimal SegWit testnet: TCP transport (no TLS), `NodeHandle` (mempool + chain + AccountStore), block production, compact block propagation, CLI control messages. Binary: `testnet_node`.
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
- `src/pin/` — Numeric PIN generation (CSPRNG, 4-6 digits), Argon2id hashing/verification, `PinStore` trait with `MemoryPinStore` for DID-to-PIN association. HTTP: `POST /pin/generate`, `POST /pin/verify`
- `src/crypto/hasher.rs` — Configurable hash algorithm (`SHA-256` / `SHA3-256`), `hash()`, `hash_with()`, KAT self-tests. `HASH_ALGORITHM` env var.
- `src/identity/pqc_policy.rs` — PQC enforcement: `enforce_pqc()` rejects classical sigs when `REQUIRE_PQC_SIGNATURES=true`, `validate_signature_consistency()` catches tag forgery (size vs algorithm mismatch).
- `src/identity/dual_signing.rs` — Crypto migration: `dual_sign()` with two providers, `verify_dual()` with `Either`/`Both` modes, `DUAL_SIGN_VERIFY_MODE` env var.
- `src/consensus/equivocation.rs` — Byzantine equivocation detection: `EquivocationDetector` tracks `(height, slot, proposer)` proposals, constructs `EquivocationProof` on conflict, gossip dedup via `receive_proof()`, proposer quarantine. Serde persistence via `to_bytes()`/`from_bytes()`.
- `src/consensus/slashing.rs` — Validator penalty economics: `PenaltyManager` with `PenaltyRecord`, `PenaltyPolicy` (configurable duration/permanent/escalation), deterministic expiration at `start_height + duration`, anti-double-slash, reputation tracking. Serde persistence.
- `crates/pqc_crypto_module/` — FIPS-oriented standalone crate: approved-mode lifecycle (`Uninitialized→Approved→Error`), ML-DSA-65 (FIPS 204), SHA3-256 (FIPS 202), ML-KEM-768 (FIPS 203 via `pqcrypto-mlkem`), KAT self-tests with roundtrip verification, `ZeroizeOnDrop` + `mlock` on private keys, exhaustive FSM tests (16/16 transition pairs), no classical fallback. ACVP dry-run harness (`tools/acvp_dry_run/`) validates all three algorithms. See `crates/pqc_crypto_module/README.md`.
- `src/transaction/segwit.rs` — SegWit model: `TxCore` (executable data) + `TxWitness` (signature + pubkey + scheme), dual Merkle roots, `validate_segwit_block()`, `NativeTransaction::to_segwit()` conversion.
- `src/transaction/verification_cache.rs` — `VerificationCache` (FIFO, key=`SHA-256(core||witness)`), `validate_segwit_block_with_cache()`, `validate_segwit_block_parallel()` (rayon).
- `src/transaction/compact_block.rs` — `CompactBlock` with `ShortId` (8-byte SHA3-256), `SegWitMempool`, `reconstruct_compact_block()`, `apply_missing_response()`.
- `src/transaction/witness_pruning.rs` — `PrunedSegWitBlock`, `prune_witnesses(block, height, depth)`, `validate_pruned_block()`.
- `src/transaction/weight_fee.rs` — Weight-based fees: `core_size×4 + witness_size×1`, `validate_fee()`, `validate_segwit_block_with_fees()`.
- `src/transaction/pqc_validation.rs` — Unified pipeline: `validate_pqc_block(block, cache, config)` with `PqcValidationConfig` (enforce_fees, use_cache, parallel_verify).
- `src/transaction/block_version.rs` — `BlockVersion` (Legacy=0, SegWitPqcV1=1), `AnyBlock`, `ChainConfig`, `validate_block_versioned()` with activation height.
- `src/transaction/replay_protection.rs` — `signing_payload_for_version()` with domain separator `RUST_BC_SEGWIT_PQC_V1_TX`, `verify_witness_versioned()`.
- `src/transaction/canonical.rs` — `CanonicalEncode` trait: deterministic binary serialization for consensus types (replaces `serde_json` in roots, cache keys, short IDs, SegWitPqcV1 signing payload). LE integers, length-prefixed strings/bytes, u8 enum discriminants.

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

### SDKs (`sdks/`)

| Path | Language | Notes |
|---|---|---|
| `sdks/js/` | TypeScript | v1.0, axios-based client, tests, examples |
| `sdks/python/` | Python | Client, types, exceptions, tests |

### Tools (`tools/`)

| Path | Purpose |
|---|---|
| `tools/acvp_dry_run/` | ACVP test vector harness (SHA3, ML-DSA, ML-KEM) |
| `tools/caliper/` | Hyperledger Caliper benchmark configs |

### Documentation (`docs/`)

Organized into subdirectories:

| Directory | Contents |
|---|---|
| `docs/api/` | API reference, quick-start, deployment, configuration guides |
| `docs/architecture/` | Core architecture, network membership, storage schema, use cases |
| `docs/architecture/benchmarks/` | Performance benchmarks and results |
| `docs/architecture/comparisons/` | Competitive analyses (Bitcoin, Fabric, IOTA) |
| `docs/architecture/roadmaps/` | Roadmaps, progress tracking, production status |
| `docs/architecture/security/` | Security audits and audit packages |
| `docs/camara/` | Blockchain Chamber of Chile: presentations, dossiers, quotations (md + pdf) |
| `docs/commercial/` | Enterprise docs, impact studies, sales materials |
| `docs/compliance/` | FIPS 140, certification roadmap, compliance framework, PQC enterprise |
| `docs/compliance/fips_submission/` | CMVP submission package (lab selection, gap analysis, timeline) |
| `docs/compliance/pre_lab_audit/` | Pre-lab mock audit (findings, traceability, ACVP plan) |
| `docs/prompts/` | Test generation prompts (POE, PROMPT1–10) |
| `docs/analysis/` | Phase analysis, architecture notes, decision matrices |
| `docs/dev/` | Developer onboarding, branching strategy, setup |
| `docs/es/` | Spanish translations |
| `docs/archive/` | Legacy documentation |
| `docs/book/` | mdBook documentation site |

All `.md` presentation docs in `docs/camara/` have corresponding `.pdf` via pandoc + weasyprint.

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
| `REQUIRE_PQC_SIGNATURES` | *(false)* | Set to `true` to reject classical (Ed25519) signatures in consensus and endorsement |
| `TLS_PQC_KEM` | *(false)* | Set to `true` to enable X25519+ML-KEM-768 hybrid TLS key exchange |
| `DUAL_SIGN_VERIFY_MODE` | *(either)* | `either` (transition) or `both` (strict) — dual-signature verification mode |
| `HASH_ALGORITHM` | *(sha256)* | `sha256` or `sha3-256` — configurable block hash algorithm |

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
- Every struct carrying a signature also carries `signature_algorithm: SigningAlgorithm` with `#[serde(default)]` (defaults to Ed25519 for backwards compat). `validate_signature_consistency()` enforces size matches tag.
- `Block` carries `hash_algorithm: HashAlgorithm` with `#[serde(default)]` (defaults to Sha256). Old blocks without the field deserialize correctly.
- `Block` and `DagBlock` carry optional `secondary_signature` + `secondary_signature_algorithm` for dual-signing during PQC migration.
- **Crypto boundary policy**: ALL production code uses `pqc_crypto_module` for cryptographic operations. Direct imports of `sha2`, `sha3`, `ed25519_dalek`, `pqcrypto_mldsa`, `rand` in `src/` are forbidden (0 violations, 100% compliant). Legacy algorithms available via `pqc_crypto_module::legacy::*` (explicitly non-approved). Enforced by `cargo test --test crypto_boundary`.
