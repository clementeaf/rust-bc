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

### HTTP API (`src/api/`)
Actix-Web 4. All routes mounted under `/api/v1` in `routes.rs`. Handlers split by domain in `handlers/`:
- `blocks.rs` — legacy chain blocks + store-backed block endpoints
- `transactions.rs` — mempool endpoints + store-backed transaction endpoints
- `identity.rs`, `credentials.rs` — store-backed DID/credential endpoints

Response envelope: `ApiResponse<T>` in `errors.rs` — always `{ status, status_code, message, data?, error?, timestamp, trace_id }`.

New store endpoints follow this pattern:
```rust
#[get("/store/...")]
pub async fn handler(state: web::Data<AppState>, ...) -> ApiResult<HttpResponse> {
    match &state.store {
        None => Err(ApiError::NotFound { resource: "store".to_string() }),
        Some(store) => { /* call store method */ }
    }
}
```

### AppState (`src/app_state.rs`)
Central shared state. Both the legacy `blockchain: Arc<Mutex<Blockchain>>` and new `store: Option<Arc<dyn BlockStore>>` live here. They are independent — writes to one do not propagate to the other.

### Other subsystems
- `src/consensus/` — DAG, fork choice, validator scheduling
- `src/identity/` — DID + key management
- `src/tls.rs`, `src/pki.rs` — mutual TLS, certificate provisioning
- `src/network.rs` — P2P node, peer discovery

## Environment variables

| Variable | Default | Description |
|---|---|---|
| `API_PORT` | 8080 | HTTP API port |
| `P2P_PORT` | 8081 | P2P gossip port |
| `DIFFICULTY` | 1 | Mining difficulty |
| `STORAGE_BACKEND` | *(memory)* | Set to `rocksdb` to enable RocksDB |
| `STORAGE_PATH` | `./data/rocksdb` | RocksDB data directory |
| `NETWORK_ID` | `mainnet` | Network identifier |
| `BOOTSTRAP_NODES` | — | Comma-separated `host:port` list |
| `SEED_NODES` | — | Always-tried peer list |

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

## Key conventions

- The `nightly` feature `#![feature(unsigned_is_multiple_of)]` is required; use `cargo +nightly` if the toolchain doesn't default to nightly.
- Block keys in RocksDB are zero-padded to 12 digits so lexicographic order matches numeric order.
- Secondary index keys use the same zero-padded prefix: `{:012}:{id}`, enabling cheap prefix range scans without a full table scan.
- `tempfile::TempDir` is the standard test helper for RocksDB tests — the directory is cleaned up on drop.
