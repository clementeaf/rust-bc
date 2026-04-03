# Changelog

Format: [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) · Versioning: [SemVer](https://semver.org)

---

## [Unreleased]

### 2026-04-03

**Storage — secondary index endpoint**
- `GET /api/v1/store/blocks/{height}/transactions` — queries `transactions_by_block_height` via prefix scan on `tx_by_block` CF

**Storage — secondary index `tx_by_block`**
- New `tx_by_block` CF in RocksDB; key schema `{012-padded-height}:{tx_id}` → empty value
- `write_transaction` and `write_batch` write index entry atomically in the same `WriteBatch`
- `BlockStore::transactions_by_block_height(height)` added to trait; delegated in `Arc<T>` blanket impl
- `MemoryStore`: equivalent linear scan over the HashMap
- 9 new tests (key format, empty result, filtering, no height bleed-over, batch indexing); 463 tests total

**Storage — Fase VI: `MemoryStore` + `Arc<T>` blanket impl**
- `Arc<T: BlockStore>` implements `BlockStore` — lets `Arc<MemoryStore>` be used as `Box<dyn BlockStore>`
- `ConsensusEngine::with_store()` persists accepted blocks into the store

**Storage — Fase V: store-backed REST endpoints**
- `POST/GET /api/v1/store/transactions/{tx_id}`
- `POST/GET /api/v1/store/identities/{did}`
- `POST/GET /api/v1/store/credentials/{cred_id}`
- All handlers return 404 when store is not configured

**Storage — Fase IV: RocksDB Column Families**
- 5 CFs: `blocks`, `transactions`, `identities`, `credentials`, `meta`
- `create_missing_column_families(true)` — compatible with new and existing DBs
- Block keys: zero-padded 12-digit decimal for lexicographic = numeric ordering
- 17 tests: per-type roundtrip, CF isolation, reopen with persisted data

**Storage — Fase III: switcheable backend**
- `STORAGE_BACKEND=rocksdb` → `RocksDbBlockStore` at `STORAGE_PATH`; default → `MemoryStore`
- Fallback to `MemoryStore` if RocksDB fails to open

**Storage — Fase II: RocksDB**
- `RocksDbBlockStore`: JSON serialization, atomic `WriteBatch`, `META:latest_height` tracking
- `rocksdb = "0.22"` added to `Cargo.toml`
- 13 unit tests with `tempfile::TempDir`

**Storage — Fase I: MemoryStore + API**
- `MemoryStore`: `BlockStore` backed by `HashMap` + `Mutex`
- `AppState.store: Option<Arc<dyn BlockStore>>`
- `GET /api/v1/store/blocks/{height}` and `/store/blocks/latest`

**Consensus — Fase H: ConsensusEngine**
- `ConsensusEngine`: wraps `Dag`, `ForkChoice`, and `SlotScheduler`
- `accept_block()` validates and inserts; `canonical_tip()` / `canonical_chain()` query state
- `ConsensusError` typed errors via `thiserror`
- 11 tests

**Consensus — Fase G: Fork Resolution**
- `Dag::subtree_weight()`, `canonical_chain()`, `resolve_fork()`
- `ForkChoiceRule`: `HeaviestSubtree` (default) and `LongestChain`
- 33 tests (22 dag, 11 fork_choice)

**TLS — Fase C: Certificate Pinning**
- `CertPinConfig`: SHA-256 fingerprint allowlist; disabled when empty
- `PinningServerCertVerifier` / `PinningClientCertVerifier`: verify CA first, then fingerprint
- `TLS_PINNED_CERTS` env var (comma-separated); absent = pinning off
- `docs/NETWORK_MEMBERSHIP.md`: pinning section with rotation guide
- 32 TLS tests total

**TLS — Fase B: mTLS**
- `build_server_config_mtls` / `build_client_config_mtls`
- `TLS_MUTUAL=true` + `TLS_CA_CERT_PATH`; explicit error if CA missing
- 2 P2P integration tests (valid handshake, server rejects client without cert)

**TLS — Fase A: TLS básico**
- `src/tls.rs`: PEM loading, `ServerConfig`, `ClientConfig`, `PeerVerification` enum
- `TLS_CERT_PATH`, `TLS_KEY_PATH`, `TLS_VERIFY_PEER`, `TLS_CA_CERT_PATH`
- P2P connections wrapped in `TlsAcceptor` / `TlsConnector`
- Dependencies: `rustls 0.23`, `rustls-pemfile 2`, `tokio-rustls 0.26`, `webpki-roots 0.26`

**CI**
- Added `toolchain: stable` to all GitHub Actions workflows (required by `dtolnay/rust-toolchain@master`)

### Changed
- Docs reorganized: `ANALYSIS/` → `docs/analysis/`, `Documents/` → `docs/archive/`
- Stopped tracking local `blockchain_blocks/` sample data

---

## [0.1.0] — target Q3 2026

Planned first release. Tracks when the Unreleased work above is stable and versioned.
