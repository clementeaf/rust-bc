# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## MANDATORY: Pre-commit quality gate

**All three must pass before every commit. No exceptions.**

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test --lib
```

If any integration test file was modified, also run `cargo test --test <test_name>`.

## Commands

```bash
cargo build                          # Build
cargo test                           # All tests
cargo test --lib storage             # Module tests
cargo test --test bft_e2e            # BFT E2E tests
cargo test -- --nocapture            # With stdout
cargo run --bin rust-bc              # Start server (API :8080, P2P :8081)
cargo clippy -- -D warnings          # Lint
cargo fmt                            # Format
./scripts/try-it.sh                  # Interactive demo
```

## Architecture overview

Blockchain node (Rust/Actix-Web 4) with HTTP API. Key layers:

- **Storage** (`src/storage/`): `BlockStore` trait with `MemoryStore` and `RocksDbBlockStore` implementations. Selected via `STORAGE_BACKEND` env var.
- **Consensus** (`src/consensus/`): DAG + HotStuff BFT + DPoS. `ConsensusBackend` trait for Raft/BFT selection.
- **Identity** (`src/identity/`): DID + pluggable signing (`Ed25519`, `ML-DSA-65`). PQC enforcement, dual-signing for migration.
- **API** (`src/api/` + `src/api_legacy.rs`): `/api/v1` scope. Response envelope: `ApiResponse<T>`. Security: mTLS + ACL + rate limiting.
- **Crypto** (`crates/pqc_crypto_module/`): FIPS-oriented crate. ALL production crypto goes through this module — direct imports of `sha2`, `ed25519_dalek`, etc. in `src/` are forbidden.

Other subsystems: bridge, governance, EVM (revm), chaincode, channels, oracles, compliance, tokenomics, intelligence, light client, audit. See `docs/architecture/` for details.

### Legacy storage — DEPRECATED

`src/blockchain.rs`, `src/block_storage.rs`, `src/models.rs` — original in-memory system. Still used by `api_legacy.rs` for mining/balance (17 refs). Migration path: move logic to `BlockStore`-backed services, then remove.

### Frontends and SDKs (separate repos)

- [cerulean-explorer](https://github.com/clementeaf/cerulean-explorer) — Block explorer (Vite + React + Tailwind)
- [cerulean-voto](https://github.com/clementeaf/cerulean-voto) — Electronic voting platform
- [cerulean-sdks](https://github.com/clementeaf/cerulean-sdks) — TypeScript and Python clients

Not required to run the node.

## Key conventions

- Requires `nightly` toolchain (`rust-toolchain.toml`).
- RocksDB keys: zero-padded 12 digits. Secondary index: `{:012}:{id}`.
- Signatures: `Vec<u8>` (not `[u8; 64]`) to support Ed25519 (64B) and ML-DSA-65 (3309B). Hex-serialized via `vec_hex` serde helpers.
- Every signed struct carries `signature_algorithm: SigningAlgorithm` with `#[serde(default)]`.
- Crypto boundary enforced by `cargo test --test crypto_boundary`.
- `tempfile::TempDir` for RocksDB test fixtures.

## Configuration

Environment variables control all runtime behavior. See [`docs/api/configuration-guide.md`](docs/api/configuration-guide.md) for the full reference.

Essential vars: `STORAGE_BACKEND`, `ACL_MODE`, `SIGNING_ALGORITHM`, `NETWORK_ID`, `API_PORT`, `P2P_PORT`.

## Deployment

See [`docs/api/DEPLOYMENT.md`](docs/api/DEPLOYMENT.md) for Docker Compose, sandbox, and production (AWS) setup.

Quick reference:
```bash
docker compose up -d                              # Multi-node network
./scripts/sandbox.sh                              # Sandbox with tunnels
./scripts/bcctl.sh status                         # Operator CLI
./scripts/e2e-test.sh                             # 71 E2E assertions
```

## Documentation

| Directory | Contents |
|---|---|
| `docs/api/` | API reference, configuration, deployment, quick-start |
| `docs/architecture/` | Core architecture, benchmarks, security audits, roadmaps |
| `docs/compliance/` | FIPS 140, certification roadmap, PQC enterprise |
| `docs/camara/` | Blockchain Chamber of Chile presentations (md + pdf) |
| `docs/commercial/` | Enterprise docs, impact studies |
| `docs/dev/` | Developer onboarding, branching strategy |
