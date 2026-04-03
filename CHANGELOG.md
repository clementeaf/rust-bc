# Changelog

Format: [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) · Versioning: [SemVer](https://semver.org)

---

## [Unreleased]

### 2026-04-03 (Fase 7 — Private Data Collections)

**7.2.1 — Access control en handlers de private data**
- `CollectionRegistry` trait + `MemoryCollectionRegistry` en `src/private_data/mod.rs`
- `ApiError::Forbidden` → HTTP 403
- `PUT/GET /api/v1/private-data/{collection}/{key}` en `src/api/handlers/private_data.rs`
- Header `X-Org-Id` obligatorio; `check_membership` verifica org en `member_org_ids` de la collection
- `AppState`: campos `private_data_store` y `collection_registry`
- 6 tests nuevos (member → 200, non-member → 403, sin header → 400, clave ausente → 404)

**7.1.2 — RocksDB private data store**
- `PrivateDataStore` trait + `MemoryPrivateDataStore`; impl para `RocksDbBlockStore` con CF `private_{name}` dinámica
- Helper `sha256` para hash on-chain; DB migrada a `DBWithThreadMode<MultiThreaded>`

**7.1.1 — PrivateDataCollection struct**
- `PrivateDataCollection { name, member_org_ids, required_peer_count, blocks_to_live }` + `is_member()`
- `PrivateDataError`: `InvalidCollection`, `AccessDenied`
- 634 lib + 535 integration tests al cierre de 7.2.1

---

### 2026-04-03 (Fase 3 — Transaction Lifecycle)

**Transaction — Fase 3.1: Read-Write Sets**
- `src/transaction/mod.rs` + `rwset.rs`: `KVRead { key, version }`, `KVWrite { key, value }`, `ReadWriteSet { reads, writes }` con `is_empty()`
- Serde derive en los tres tipos; módulo declarado en `lib.rs` y `main.rs`
- 6 tests nuevos; 531 tests en total

---

### 2026-04-03 (Fase 1–2 — Endorsement + Ordering)

**Endorsement (Fase 1) — completa**
- `src/endorsement/`: `Organization`, `OrgRegistry` trait + `MemoryOrgRegistry`, CF `organizations` en RocksDB
- `EndorsementPolicy` (AnyOf / AllOf / NOutOf / And / Or) + `evaluate()`
- `PolicyStore` trait + `MemoryPolicyStore`
- `Endorsement` struct + `verify_endorsement` + `validate_endorsements`
- `Block.endorsements: Vec<Endorsement>` (serde default)
- `ConsensusEngine::with_policy_store()`: valida endorsements antes de insertar en DAG
- REST: `POST/GET /api/v1/store/organizations`, `GET /api/v1/store/organizations/{id}`, `POST/GET /api/v1/store/policies/{resource_id}`
- `AppState`: `org_registry`, `policy_store`

**Ordering (Fase 2) — completa**
- `src/ordering/`: `NodeRole` enum (Peer / Orderer / PeerAndOrderer) + `FromStr` desde `NODE_ROLE` env
- `OrderingService`: cola `VecDeque<Transaction>`, `submit_tx`, `cut_block` con batch drain
- `run_batch_loop`: tokio task lanzada en `main.rs` si el nodo ordena
- `Node.role: NodeRole`; `Message::SubmitTransaction` y `Message::OrderedBlock`
- `process_message`: orderer ingesta TXs; peer persiste `OrderedBlock` directamente en store
- 525 tests al cierre de Fase 2

---

### 2026-04-03 (Storage)

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
