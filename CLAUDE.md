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

### Legacy system (`src/blockchain.rs`, `src/block_storage.rs`, `src/models.rs`) — DEPRECATED
The original in-memory `Blockchain` struct plus a file-backed `BlockStorage`. Loaded at startup and kept in `AppState`. **Not integrated with the new storage layer.** Still used by `api_legacy.rs` for mining, balance calculation, and transaction processing (17 production references). The new `BlockStore` trait handles identity, credentials, channels, and block persistence. Migration path: gradually move mining/balance logic to services backed by `BlockStore`, then remove `Blockchain` struct. Not urgent — both systems operate independently without conflicts.

### New storage layer (`src/storage/`)
Clean trait-based persistence introduced in Fases I–VI:
- `traits.rs` — `BlockStore` trait + data types (`Block`, `Transaction`, `IdentityRecord`, `Credential`). A blanket `impl<T: BlockStore> BlockStore for Arc<T>` lets `Arc<MemoryStore>` be used as `Box<dyn BlockStore>`.
- `memory.rs` — `MemoryStore`: HashMap-backed, used as default and in tests.
- `adapters.rs` — `RocksDbBlockStore`: Column Families (`blocks`, `transactions`, `identities`, `credentials`, `meta`, `tx_by_block`, `audit_log`, `sandbox_reports`, `oracle_records`). On open, static names are merged with `RocksDB::list_cf` so extra families on disk (e.g. `private_*`) are opened. Secondary index `tx_by_block` uses key `{012-padded-height}:{tx_id}` for prefix scans. `flush_wal()` for graceful shutdown.
- `migrations.rs` — Schema migration system: version tracked in `meta` CF, migration registry runs pending steps on startup. `LATEST_VERSION = 2`. Add new migrations by incrementing version + registering a function.
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
- `src/network/mod.rs` — P2P node, peer discovery, BFT message types (`BftProposal`, `BftVote`, `BftQuorumCertificate`, `BftViewChange`). `MembershipTable` with `MAX_PEERS` cap (default 500) for Sybil protection.
- `src/transaction/parallel.rs` — conflict detector (RAW/WAW/WAR) + wave scheduler; groups non-conflicting txs for concurrent execution
- `src/transaction/executor.rs` — wave-parallel block executor: MVCC validate per wave, apply writes in deterministic order, `to_legacy_results()` adapter
- `src/tokenomics/economics.rs` — NOTA supply cap (100M), halving rewards, capped issuance, 80/20 fee burn/proposer split, EIP-1559 dynamic base fee, epoch-based `process_block()` state machine
- `src/tokenomics/storage_deposit.rs` — `DepositLedger`: lock tokens proportional to data size on state writes, refund on delete, delta on update
- `src/bridge/` — cross-chain bridge: chain registry, message envelope, escrow vault (lock/release outbound, mint/burn inbound), Merkle inclusion proof verifier, `BridgeEngine` with replay protection, `Relayer` with job queue, batch processing, and retry logic
- `src/governance/` — on-chain governance: typed `ParamRegistry` with protocol defaults, `ProposalStore` (submit→vote→pass→timelock→execute lifecycle), stake-weighted `VoteStore` (Yes/No/Abstain, quorum + threshold checks). Vote handler: optional Ed25519 signature verification, blind voter ID for vote secrecy (`sha256(proposal_id || voter_did)`), DID-to-pubkey binding. HTTP API: 7 endpoints under `/api/v1/governance/` (params, proposals CRUD, vote, tally, JSON-LD export). AppState fields: `proposal_store`, `vote_store`, `param_registry`.
- `src/api/handlers/interop.rs` — W3C interoperability: DID Resolution (`GET /did/{did}` → DID Document with Ed25519VerificationKey2020), Verifiable Credentials (`GET /credentials/{id}/vc` → VC Data Model 2.0 with Ed25519 proof), JSON-LD export (`GET /governance/proposals/{id}/export` → schema.org VoteAction). Content types: `application/did+ld+json`, `application/vc+ld+json`, `application/ld+json`.
- `src/light_client/` — compact `BlockHeader` chain with BFT QC verification, `LightClient` for state proof verification via Merkle proofs against synced headers. Enables IoT/mobile participation without full node.
- `src/transaction/executor.rs` — `execute_block_concurrent()` async tokio executor for true intra-wave parallelism
- `src/testnet/` — `GenesisConfig` (testnet/devnet/mainnet presets with validation), `Faucet` (rate-limited token drip with cooldown and depletion)
- `src/evm_compat/` — Full EVM execution via revm (`executor.rs`), Solidity ABI encoding/decoding (`abi.rs`), precompile interface with gas metering (`precompile.rs`), DID-to-address derivation. HTTP endpoints: `POST /evm/deploy`, `POST /evm/call`, `POST /evm/static-call`, `GET /evm/contracts`
- `src/channel/store.rs` — `ChannelStore`: per-channel isolated world state and block ledger (Fabric-compatible channel isolation)
- `src/channel/config.rs` — `ChannelConfig` with `RetentionPolicy` (block retention count, private data TTL, transaction retention), configurable per channel via `ConfigUpdateType::SetRetention`
- `src/events/webhook.rs` — `WebhookNotifier`: subscribes to `EventBus`, filters security events (`AclDenied`, `EquivocationDetected`, `RateLimitExceeded`, `InvalidSignature`, `ValidatorSlashed`), POSTs to `CSIRT_WEBHOOK_URL` with exponential backoff
- `src/chaincode/upgrade.rs` — `UpgradeManager`: multi-org approval lifecycle for chaincode version upgrades (propose→approve→commit)
- `src/audit.rs` — `AuditStore` trait + `MemoryAuditStore`. Two audit levels: request-level (every HTTP request via `AuditMiddleware`) and action-level (semantic domain events like `BlockMined`, `DidRegistered`, `ChaincodeInstalled`). `emit_if_present()` helper for fire-and-forget audit from handlers. Endpoints: `GET /audit/requests?action=...&org_id=...&from=...&to=...`, `GET /audit/export` (CSV).
- `src/chaincode/sandbox.rs` — Static Wasm validation gate: well-formedness (wasmparser), import whitelist (6 allowed host functions), memory limits (max 16 pages). Runs on `install_chaincode` — rejects invalid Wasm before storage. `SandboxReportStore` persists reports. Endpoint: `GET /chaincode/{id}/sandbox-report?version=...`.
- `src/legal_oracle/` — Off-chain legal data oracle. `LegalOracle` service queries configurable sources with TTL cache, stores `OracleRecord` (response_hash on-chain, full data off-chain). Endpoints: `POST /oracle/legal/query`, `GET /oracle/legal/records`, `GET /oracle/legal/records/{id}`.
- `src/identity/zkp.rs` — Commitment-based attribute verification (NOT zero-knowledge — verifier sees claim value). SHA-256 + blinding commitment with 3 predicates: RangeProof, SetMembership, CredentialValidity. Endpoints: `POST /identity/zkp/prove`, `POST /identity/zkp/verify`.
- `src/pin/` — Numeric PIN generation (CSPRNG, 4-6 digits), Argon2id hashing/verification, `PinStore` trait with `MemoryPinStore` for DID-to-PIN association. HTTP: `POST /pin/generate`, `POST /pin/verify`
- `src/crypto/hasher.rs` — Configurable hash algorithm (`SHA-256` / `SHA3-256`), `hash()`, `hash_with()`, KAT self-tests. `HASH_ALGORITHM` env var.
- `src/identity/pqc_policy.rs` — PQC enforcement: `enforce_pqc()` rejects classical sigs when `REQUIRE_PQC_SIGNATURES=true`, `validate_signature_consistency()` catches tag forgery (size vs algorithm mismatch).
- `src/identity/dual_signing.rs` — Crypto migration: `dual_sign()` with two providers, `verify_dual()` with `Either`/`Both` modes, `DUAL_SIGN_VERIFY_MODE` env var.
- `src/consensus/equivocation.rs` — Byzantine equivocation detection: `EquivocationDetector` tracks `(height, slot, proposer)` proposals, constructs `EquivocationProof` on conflict, gossip dedup via `receive_proof()`, proposer quarantine. Serde persistence via `to_bytes()`/`from_bytes()`.
- `src/consensus/slashing.rs` — Validator penalty economics: `PenaltyManager` with `PenaltyRecord`, `PenaltyPolicy` (configurable duration/permanent/escalation), deterministic expiration at `start_height + duration`, anti-double-slash, reputation tracking. Serde persistence.
- `crates/pqc_crypto_module/` — FIPS-oriented standalone crate: approved-mode lifecycle (`Uninitialized→Approved→Error`), ML-DSA-65 (FIPS 204), SHA3-256 (FIPS 202), ML-KEM-768 (FIPS 203 via `pqcrypto-mlkem`), KAT self-tests with roundtrip verification, `ZeroizeOnDrop` + `mlock` on private keys, exhaustive FSM tests (16/16 transition pairs), no classical fallback. ACVP dry-run harness (`tools/acvp_dry_run/`) validates all three algorithms. See `crates/pqc_crypto_module/README.md`.

- `src/intelligence/` — Anomaly detection (z-score), risk scoring (6-rule AML), pattern recognition (velocity, structuring, round-trip, dormant activation). HTTP: `POST /intelligence/anomaly`, `POST /intelligence/risk`, `POST /intelligence/patterns`.
- `src/oracle_demo.rs` — Simulated market feeds (BTC/ETH/CLP) for sandbox. `ORACLE_DEMO=true` env var.
- `src/oracle_connector.rs` — External HTTP price feeds with multi-source aggregation, spread detection, JSON path extraction. Activated by `ORACLE_SOURCES` env var. Wired in `main.rs` via `spawn_oracle_poller`.
- `src/oracle_system.rs` — Oracle registry: reputation, outlier filtering, median aggregation, HMAC signatures, staleness detection. HTTP: `GET /oracle/feeds`, `GET /oracle/feeds/{symbol}`, `GET /oracle/nodes`, `GET /oracle/status`.
- `src/oracle_collateral.rs` — Bonding/collateral/dispute/slashing for oracle operators.
- `src/regulatory/` — 21 compliance checks (Ley 21.663, ISO 20022, ERC-3643, retention, intelligence, forensic). Report generation with SHA-256 hash. HTTP: `GET /regulatory/checks`, `GET /regulatory/report`.
- `src/stress.rs` — Per-module stress tests (10 modules: storage, crypto, anomaly, risk, compliance, governance, identity, credential, forensic, patterns). HTTP: `GET /stress/report?ops=N`.
- `src/compliance/` — ISO 20022 (7 message types), ISO 3166 (193 countries), ISO 4217 (64 currencies), ISO 8601 (dates/durations), ERC-3643 (security tokens with `checked_add` overflow protection). HTTP: `POST /compliance/validate/*`, `GET /compliance/countries`, `GET /compliance/currencies`.
- `src/forensic_pentest.rs` — 40 adversarial attack scenarios across 8 categories (integrity, cryptography, access control, consensus/BFT, network P2P, EVM, economic, identity/governance). Covers: tampering, forgery, replay, double-spend, equivocation, ACL bypass, channel crossing, identity spoofing, overflow, rollback, path traversal, oracle manipulation, governance abuse, credential forgery, oversized payload, Sybil flooding, malformed P2P, eclipse attack, front-running, reentrancy, storage collision, nothing-at-stake, long-range reorg, validator grinding, fee suppression, delegatecall abuse, gas bomb, proposer-MEV, oversized proposal description, empty voter, quorum-zero param attack, credential without issuer, vote spam, delegation cycle, signature bypass. All exercise real code paths. 0 critical vulnerabilities. HTTP: `GET /pentest/report`.

### Block explorer — Cerulean Ledger UI

| Path | Stack | Notes |
|---|---|---|
| `block-explorer-vite/` | Vite + React + Tailwind | `npm install` / `npm run dev`; proxies `/api` to the node. |

Branded as **Cerulean Ledger**. Full Spanish UI. DID prefix: `did:cerulean:`.

Routes:
- `/` — Landing page (hero with live network pulse, thesis, verticals selector, numbers, audit quote, CTA)
- `/integridad` — Institutional integrity dashboard (flagship): 8 horizontal service cards with detail drawers, integrity report table, security events timeline, vertical control cards, stress performance grid. Auto-refresh 30s. Print-friendly.
- `/dashboard` — Network stats, blocks, hub cards
- `/identity` — Digital identity module: identity list + signed documents panel + detail drawer with cryptographic proof
- `/demo` — 5-step RRHH credential verification demo (compact single-card layout)
- `/compliance` — Audit trail with action/org filters, summary indicators, auto-refresh
- `/chaincode-health` — Sandbox report viewer per chaincode version
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

Standalone voting frontend with real Ed25519 wallet integration. Consumes governance, identity, and interop APIs.

Routes (grouped in sidebar: Votacion / Organizacion / Administracion):
- `/` — Landing page (hero + 3 pillars, standalone)
- `/dashboard` — Active/closed election stats with internal scroll panels
- `/elections` — History table + slide-over drawer for creation
- `/vote` — Wallet-based voting: select registered voter (dropdown), enter passphrase, Ed25519-signed vote with animated receipt
- `/results` — Compact tally cards with percentage bars, quorum/threshold stats
- `/voters` — Wallet registration (Ed25519 via WASM), padron table with DID, address, algorithm
- `/assemblies` — Assembly CRUD (ordinaria/extraordinaria), convocatoria validation (Ley 19.418 Art. 16), folio correlativo
- `/sessions?assembly=ID` — Sessions per assembly: citation (1a/2a), quorum check, agenda linked to proposals. Auto-generates acta + blockchain anchoring on close
- `/actas` — Libro de Actas: permanent records (ISO 15489), SHA-256 hash, blockchain anchor (`did:cerulean:acta:{folio}`), legal format with signatures, print-friendly
- `/admin` — Org settings (name, RUT, president, secretary), quorum config (1a/2a citation), normativa reference, export/import JSON

**Wallet integration:** Uses cerulean-wallet WASM module (`src/wasm/`). Ed25519 keygen + Argon2id + AES-256-GCM encryption. Wallets cross-compatible with cerulean-wallet CLI. DID derived from `sha256(public_key)[0..20]`.

**Vote security:**
- Ed25519 signature over canonical payload: `vote:{proposal_id}:{option}:{public_key}`
- Backend verifies signature + DID-to-pubkey binding before accepting
- Blind voter ID: `sha256(proposal_id || voter_did)` — real identity never stored with vote
- Deduplication: `AlreadyVoted` error on same (proposal, blind_id)

**Compliance:** Ley 19.418 Art. 16 (convocatoria deadlines, quorum), Art. 17 (actas content, signatures), ISO 15489 (permanent records, integrity hash), ISO 8601 (dates).

UI patterns: `h-screen` fixed layout, no page-level scroll, `border-neutral-100` borders, no shadows, compact padding.

Key structure:
- `src/lib/api.ts` — governance + identity + acta anchoring API client
- `src/lib/wallet.ts` — WASM wallet integration (createWallet, signVote, didFromPublicKey, localStorage persistence)
- `src/lib/store.ts` — localStorage CRUD for assemblies, sessions, actas, org settings (correlative counters, SHA-256 integrity, schema migration merge)
- `src/lib/routes.ts` — 9 lazy-loaded routes, 3 sidebar groups
- `src/lib/format.ts` — shared formatters (`timeAgo`, `pct`, `fmtDateTime`)
- `src/wasm/` — cerulean-wallet WASM module (Ed25519 keygen, signing, HD derivation)
- `src/components/Layout.tsx` — header + grouped sidebar + minimal footer, `h-screen overflow-hidden`
- `Dockerfile` + `nginx.conf` — containerized with API proxy to node

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
| `CSIRT_WEBHOOK_URL` | — | POST security events to this URL (enables CSIRT/SIEM integration) |
| `CSIRT_WEBHOOK_SECRET` | — | Shared secret sent as `X-Webhook-Secret` header |
| `CSIRT_WEBHOOK_TIMEOUT_SECS` | 10 | HTTP timeout for webhook requests |
| `ORACLE_DEMO` | *(false)* | Set to `true` to enable simulated price feeds (BTC/ETH/CLP) |
| `ORACLE_SOURCES` | — | Comma-separated HTTP URLs for real external price feeds |
| `ORACLE_CONNECTOR_SYMBOL` | `BTC/USD` | Symbol for the external oracle connector feed |
| `ORACLE_POLL_INTERVAL_SECS` | 60 | Polling interval for external oracle sources |
| `ORACLE_MIN_SOURCES` | 1 | Minimum agreeing sources for valid oracle reading |

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

## Sandbox (public demo)

```bash
# One-command launcher: node + explorer + voto + observability + Cloudflare tunnels
./scripts/sandbox.sh          # Start (gives public URLs)
./scripts/sandbox.sh stop     # Stop everything
./scripts/sandbox.sh reset    # Wipe data + re-seed fresh
./scripts/sandbox-backup.sh   # Snapshot RocksDB volume
./scripts/sandbox-backup.sh restore <tarball>  # Restore from backup
```

Uses `docker-compose.sandbox.yml` (single node, PQC, permissive ACL, RocksDB, Prometheus, Grafana). See `SANDBOX.md` for custom domain setup.

| Service | Port | Notes |
|---|---|---|
| Node API | :9600 | PQC, RocksDB, 512M memory cap |
| Explorer | :5173 | Branded 502 fallback, SSE proxy |
| Voto | :5174 | Branded 502 fallback |
| Prometheus | :9090 | Scrapes node /metrics every 10s |
| Grafana | :3000 | Pre-provisioned dashboard (admin/admin) |

Seed data (`scripts/seed-sandbox.sh`): 2 orgs, 2 channels, 7 wallets, 8 blocks, transfers, 7 DIDs, 5 credentials, governance proposals with votes.

## Production deployment (AWS)

Architecture: frontends on S3/CloudFront, backend on EC2.

| Component | Service | URL |
|---|---|---|
| Explorer | S3 + CloudFront | https://ceruleanledger.com |
| Voto | S3 + CloudFront | https://voto.ceruleanledger.com |
| Node API | EC2 (t3.medium) | proxied via CloudFront `/api/*` |
| Grafana | EC2 | http://<ec2-ip>:3000 |

Frontend deploy:
```bash
# Build and upload Explorer
cd block-explorer-vite && npm run build
aws s3 sync dist/ s3://ceruleanledger-explorer/ --delete

# Build and upload Voto
cd cerulean-voto && npm run build
aws s3 sync dist/ s3://ceruleanledger-voto/ --delete

# Invalidate CloudFront cache after deploy
aws cloudfront create-invalidation --distribution-id E9QQPJR6KVMFH --paths "/*"
aws cloudfront create-invalidation --distribution-id E2QW638B59JZ89 --paths "/*"
```

Backend deploy (EC2):
```bash
# Uses docker-compose.sandbox.yml (node + prometheus + grafana only)
# Seed with: API_URL=http://localhost:9600 ./scripts/seed-sandbox.sh
```

Alternative deploy with TLS via Caddy: `docker-compose.deploy.yml` + `.env` (see `.env.deploy.example`).

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
