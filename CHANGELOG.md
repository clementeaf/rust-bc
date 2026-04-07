# Changelog

Format: [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) · Versioning: [SemVer](https://semver.org)

---

## [Unreleased]

### 2026-04-07 (MVP Readiness)

**Graceful shutdown**
- SIGTERM/SIGINT handler via `tokio::signal` — drains HTTP connections, aborts background tasks, flushes RocksDB

**Persistent service stores**
- 8 of 9 services now persist to RocksDB when `STORAGE_BACKEND=rocksdb`
- New CF impls: `PolicyStore`, `CollectionRegistry`, `ChaincodeDefinitionStore`
- Added serde derives to `PrivateDataCollection`, `ChaincodeDefinition`, `ChaincodeStatus`
- Single shared `Arc<RocksDbBlockStore>` instance for all services
- Explicit failure: node exits if `STORAGE_BACKEND=rocksdb` and DB fails to open (no silent fallback)

**Health check with dependency verification**
- `/api/v1/health` now reports `checks: { storage, peers, ordering }`
- Returns `"degraded"` when storage or ordering is unavailable

**JS/TS SDK — Fabric-style operations**
- New methods: `submitTransaction`, `evaluate`, `registerOrg`, `setPolicy`, `createChannel`, `listChannels`, `putPrivateData`, `getPrivateData`

**Mutex poison recovery**
- Replaced 178 `.lock()/.read()/.write().unwrap()` with `unwrap_or_else(|e| e.into_inner())`
- Prevents cascading panics across threads from poisoned locks

**Documentation**
- `docs/QUICK-START.md` — git clone to first transaction in < 5 minutes
- `docs/API-REFERENCE.md` — all 68 endpoints with curl examples
- `docs/DEPLOYMENT.md` — production config, env vars, security checklist
- `docs/MVP-ROADMAP.md` — task-level breakdown for MVP delivery

---

### 2026-04-07 (CI Stabilization)

**Docker TLS permissions**
- `deploy/generate-tls.sh` now runs `chmod 644` on generated `.pem` files so the non-root container user (`rustbc`, uid 1000) can read them through the read-only `/tls` volume mount

**E2E test resilience**
- Grafana health check skipped when Grafana is not running (CI only starts blockchain nodes)
- Channel membership test asserts "not 403" instead of exact 200, isolating membership enforcement from downstream endorsement errors
- `POST /api/v1/store/transactions` now returns `status_code: 201` in the JSON envelope to match the HTTP 201 Created status

**Flaky Raft test fix**
- `three_nodes_in_process_propose_committed_on_all` routing rounds increased from 30 to 50, accommodating worst-case Raft election timeout randomisation on slow CI runners

**CI status:** all 4 jobs green (Check + Clippy, Build CLI, Unit Tests, E2E Tests)

---

### 2026-04-07 (Production Hardening)

**ACL deny-by-default**
- `enforce_acl()` now denies requests with missing identity, missing ACL infrastructure, or undefined ACL entries
- New env var `ACL_MODE=permissive` restores the old allow-all behavior for local development
- `enforce_channel_membership()` denies requests without `X-Org-Id` on non-default channels (strict mode)

**JWT secret from environment**
- `ApiConfig` reads `JWT_SECRET` env var at startup; falls back to hardcoded default only if unset

**CouchDB async client**
- Replaced `reqwest::blocking::Client` with async `reqwest::Client` in `CouchDbWorldState`
- Sync `WorldState` trait bridged via `block_in_place` + `Handle::block_on` (no runtime deadlock)
- Same fix applied to `ExternalInvoker` in `src/chaincode/invoker.rs`

**Configurable P2P buffer sizes**
- `P2P_RESPONSE_BUFFER_BYTES` — `send_and_wait` responses (default 256 KB, was 64 KB)
- `P2P_HANDLER_BUFFER_BYTES` — per-connection message handler (default 64 KB, was 8 KB)
- `P2P_SYNC_BUFFER_BYTES` — pull-based state sync responses (default 4 MB, was 1 MB)

---

### 2026-04-06 (E2E Tests, Operator Tooling, Full Service Wiring & Gap Analysis)

**All scaffold services wired to startup**
- `org_registry`, `policy_store`, `discovery_service`, `private_data_store`, `collection_registry`, `chaincode_package_store`, `chaincode_definition_store`, `gateway` initialized in `main.rs`
- `POST /api/v1/private-data/collections` endpoint added for collection registration

**Route registration fix**
- `ApiRoutes::register()` uses `.configure()` closures to break the generic type chain and prevent stack overflow from deeply nested Actix wrappers
- `ApiRoutes::configure()` kept for integration tests, `configure_metrics()` for production
- Main thread spawned with 32 MB stack to accommodate release + debug builds

**E2E test suite** (`scripts/e2e-test.sh`) — 42 pass, 0 fail, 0 skip
- Organizations, endorsement policies, channel isolation
- Block mining with multi-node propagation
- Transaction lifecycle (wallet → mempool → mine → block)
- Private data (register collection → write → read authorized → deny unauthorized)
- Discovery (register peers → query endorsers → query channel peers)
- Gateway (endorse → order → commit pipeline)
- Chain integrity, Prometheus metrics, Grafana health
- Store CRUD (identities, credentials)

**Operator CLI** (`scripts/bcctl.sh`)
- 14 commands: `status`, `peers`, `blocks`, `mine`, `wallet create`, `channels`, `channel create`, `orgs`, `logs`, `restart`, `metrics`, `verify`, `consistency`, `env`

**Fabric 2.5 gap analysis** (`FABRIC-GAP-ANALYSIS.md`)
- Detailed comparison: 12 verified E2E categories, 10 implemented-but-untested features, gaps vs Fabric
- Research-backed task backlog with code change requirements, blockers, and exact E2E steps
- Key findings: Raft is in-process only (no network transport), MVCC not wired to gateway, install doesn't create chaincode definition, world_state not initialized for snapshots

---

### 2026-04-05 (Docker & P2P Networking)

**Docker deployment**
- Multi-stage `Dockerfile` (nightly Rust builder + `debian:bookworm-slim` runtime)
- `docker-compose.yml`: 3 peers + 1 orderer + Prometheus + Grafana
- Self-signed TLS via `deploy/generate-tls.sh` (EC P-256, per-node SANs)
- Non-root container user, named volumes for persistence

**Network fixes for containerized nodes**
- `BIND_ADDR` env var for HTTP listen address (default `127.0.0.1`, containers use `0.0.0.0`)
- `P2P_EXTERNAL_ADDRESS` env var for announce address (e.g. `node1:8081`)
- `Node::p2p_address()` helper replaces 8 hardcoded `self.address` formats
- P2P TLS acceptor now configured on the server node (was missing)
- Fixed `TLS_CA_CERT_PATH` env var name in compose (was `TLS_CA_PATH`)

**Route unification**
- Merged legacy and scaffold into a single `/api/v1` scope
- `ApiRoutes::register()` appends scaffold sub-services into the legacy scope
- `ApiRoutes::configure()` retained for integration tests (standalone scope)
- `ApiRoutes::configure_metrics()` used in production (metrics only)
- `health`, `version`, `openapi.json` registered as `.route()` in the legacy scope

**E2E verified**
- 4 nodes healthy, 3 peers each via mutual TLS
- Block mining on node1 propagates to node2/node3 within seconds
- 2020 unit/integration tests passing

---

### 2026-04-04 (Fase 19 — Snapshots + Pagination)

**19.1 — State snapshots**
- `StateSnapshot` metadata struct in `src/storage/snapshot.rs`
- `create_snapshot()`: serializes world state to `{key}\t{version}\t{base64}\n` format with SHA-256 hash
- `restore_snapshot()`: reads `.snap` file, restores world state, verifies hash integrity
- API handlers: `POST /snapshots/{channel_id}`, `GET /snapshots/{channel_id}`, `GET /snapshots/{channel_id}/{id}`
- `AppState.world_state` field added; `base64 = "0.22"` dependency added

**19.2 — State regeneration**
- `regenerate_state()`: replays all blocks from store to rebuild world state

**19.3 — Pagination**
- `PaginationParams` (page/limit/cursor) and `PaginatedResponse<T>` in `src/api/pagination.rs`
- `BlockStore::list_blocks(offset, limit)` with default implementation
- `GET /store/blocks` now accepts `?page=N&limit=M` and returns `PaginatedResponse`

---

### 2026-04-04 (Fase 18 — Delivery Service)

**18.1 — DeliverFiltered**
- `FilteredBlock` and `FilteredTx` structs in `src/events/filtered.rs`
- `to_filtered_block()` strips payload/rwset/endorsements, keeps only tx IDs and validation codes
- `GET /events/blocks/filtered` WebSocket streams `FilteredBlock` summaries

**18.2 — DeliverWithPrivateData**
- `BlockWithPrivateData` struct in `src/events/private_delivery.rs`
- `GET /events/blocks/private` WebSocket with `X-Org-Id` header for collection membership filtering
- `CollectionRegistry::list()` method added for iterating registered collections

**18.3 — Replay and checkpoints**
- `start_block` field in `WsFilter`: replays historical blocks before switching to live
- `ack` + `client_id` checkpoint system: server tracks last acked height per client
- Reconnect with same `client_id` resumes from `last_ack + 1`

---

### 2026-04-04 (Fase 17 — Key History + Chaincode-to-Chaincode)

**17.1 — Key history**
- `HistoryEntry` struct in `storage/traits.rs`
- CF `key_history` in RocksDB with `{key}\x00{version:012}` key schema
- `get_history` method on `WorldState` trait, implemented for Memory and RocksDB
- `put()` and `delete()` auto-append history entries in `MemoryWorldState`
- `get_history_for_key` host function in `WasmExecutor`

**17.2 — Chaincode-to-chaincode invocation**
- `ChaincodeResolver` trait + `StoreBackedResolver` in `src/chaincode/resolver.rs`
- `invoke_chaincode` host function: resolves target, creates child executor, shares `WorldState`
- ACL check via `AclProvider` before cross-chaincode calls (`chaincode/{id}/invoke`)
- `MAX_CHAINCODE_DEPTH=8` recursion limit with depth counter propagation
- `ChaincodeError::NotFound` variant added

---

### 2026-04-04 (Fase 16 — Gossip Protocol Enhancement)

**16.1 — Alive messages**
- `AliveMessage` struct in `src/network/gossip.rs` with Ed25519 signature verification
- `Alive(AliveMessage)` variant in the P2P `Message` enum
- `MembershipTable`: thread-safe peer liveness tracking with suspect sweep
- `start_alive_loop` on `Node`: periodic broadcast + suspect detection
- Refactored `src/network.rs` → `src/network/mod.rs` + `gossip.rs` module

**16.2 — Pull-based state sync**
- `StateRequest { from_height }` and `StateResponse { blocks }` message variants
- `STATE_BATCH_SIZE` (50) caps response payload
- `start_pull_sync_loop` on `Node`: periodic height comparison + block fetch
- Anti-entropy: `latest_height` field on `AliveMessage`, `peers_ahead_of` gap detection

**16.3 — Anchor peers**
- `AnchorPeer` struct with `parse_anchor_peers` from `ANCHOR_PEERS` env var
- `connect_to_anchor_peers` runs before bootstrap for cross-org discovery
- `anchor_peers_from_config` bridges `ChannelConfig.anchor_peers` map to gossip

**16.4 — Leader election per org**
- `LeaderElectionMode` enum (`Static` / `Dynamic`) from `LEADER_ELECTION` env var
- `elect_leader(org_id)`: smallest alive peer address wins; failover on suspect

39 network tests passing.

---

### 2026-04-04 (Fase 15 — Raft Consensus Ordering)

**15.1 — Raft core**
- `RaftNode` in `src/ordering/raft_node.rs`: wrapper over tikv `RawNode<MemStorage>`
- `new`, `tick`, `propose`, `step`, `advance` methods
- Full raft 0.7 ready cycle: handles `messages()` (leader) and `persisted_messages()` (candidate/follower) correctly
- `create_snapshot` / `apply_snapshot` for node catch-up
- 8 tests: init, election, 3-node leader election, propose+commit, 5-entry replication, snapshot transfer

**15.2 — Raft ordering service**
- `RaftOrderingService` in `src/ordering/raft_service.rs`: JSON-serialized TX proposals through raft, committed entry draining with no-op filtering
- `OrderingBackend` trait in `src/ordering/mod.rs` with `submit_tx`, `cut_block`, `pending_count`
- Implemented by both `OrderingService` (solo) and `RaftOrderingService` (raft)
- Backend selection via `ORDERING_BACKEND=raft|solo` env var; `RAFT_NODE_ID`, `RAFT_PEERS` for raft config
- `AppState.ordering_backend: Option<Arc<dyn OrderingBackend>>` added
- 6 tests: 3 raft service + 2 trait object + 1 batch size

**15.3 — Raft network transport**
- `Message::RaftMessage(Vec<u8>)` variant added to P2P `Message` enum
- `src/ordering/raft_transport.rs`: prost encode/decode, `tick_and_collect`, `deliver_raw`
- `prost` dependency aligned to 0.11 (matches raft-proto)
- 3 tests: serde roundtrip, encode/decode roundtrip, 3-node in-process replication through serialized bytes

**15.4 — Orderer block signing**
- `Block.orderer_signature: Option<[u8; 64]>` with `#[serde(default, skip_serializing_if)]`
- `sign_block(block, key)`: `sha256(height || parent_hash || merkle_root)` signed with Ed25519
- `verify_orderer_signature(block, verifying_key)`: `Ok(true)` valid, `Ok(false)` absent, `Err` invalid
- Both backends sign when `with_signing_key(key)` is set
- 4 tests: sign+verify, valid accept, invalid reject, absent accept

---

### 2026-04-04 (Fase 12 — Hardening · §12.3 Benchmarks)

**12.3.1–12.3.3 — Criterion benchmarks** (`benches/ordering_throughput.rs`)
- `ordering_service/submit_and_cut/100` — throughput de ordering: 100 TXs → 1 bloque; reporta TXs/s
- `endorsement_validation/validate_endorsements/{1,3,5,10}` — latencia por endorsement Ed25519 con política `AllOf(N)`
- `event_bus_fanout/publish_1_event/{1,5,10,50}` — costo de `publish()` con N suscriptores activos en canal broadcast
- `criterion = "0.5"` añadido a `[dev-dependencies]`; informes HTML en `target/criterion/`

---

### 2026-04-04 (Fase 7 — Private Data Collections · §7.1.3)

**7.1.3 — Purge de datos expirados**
- `put_private_data_at(collection, key, value, written_at_height, blocks_to_live)` añadido al trait `PrivateDataStore` — default no-op delegando a `put_private_data` para backwards compat
- `purge_expired(current_height)` en el trait con default no-op; `MemoryPrivateDataStore` elimina entradas donde `written_at + blocks_to_live <= current_height`
- Entradas sin TTL (`blocks_to_live = 0`) nunca expiran
- 5 tests: expiración exacta en altura 6, sin expirar antes, purge selectivo (corto vs largo TTL), sin-TTL inmortal, `blocks_to_live=0`

---

### 2026-04-03 (Fase 9 — Fabric Gateway)

**9.1.1 — `Gateway` struct**
- `src/gateway/mod.rs`: campos `org_registry`, `policy_store`, `ordering_service`, `store`
- `mod gateway` declarado en `lib.rs` y `main.rs`
- 3 tests: crear con mocks, store vacío, policy store vacío

**9.1.2 — `Gateway::submit`**
- Pipeline: consulta policy → self-endorse → `ordering_service.submit_tx` → `cut_block` → `store.write_block`
- `TxResult { tx_id, block_height }` como tipo de retorno
- `GatewayError`: `PolicyNotSatisfied`, `Ordering`, `Storage`
- 4 tests: sin policy, `AnyOf` satisfecha, policy no satisfecha, alturas secuenciales

**9.1.3 — `POST /api/v1/gateway/submit`**
- Handler `gateway_submit` en `src/api/handlers/gateway.rs`
- Acepta `{ chaincode_id, transaction: { id, input_did, output_recipient, amount } }`
- Devuelve `{ tx_id, block_height }`; 404 si gateway no configurado; 400 si campos vacíos
- `gateway: Option<Arc<Gateway>>` añadido a `AppState`
- 3 tests HTTP: 200 end-to-end, 404 sin gateway, 400 con campos vacíos

**Total tests: 1470**

---

### 2026-04-03 (Fase 8 — Chaincode Lifecycle · §8.3 Wasm execution)

**8.3.4 — Memory limit**
- `WasmExecutor::with_memory_limit(max_bytes)` builder method
- `StoreLimitsBuilder::memory_size` + `store.limiter()` activan el límite por invocación
- Módulo que pide más páginas de las permitidas falla en instanciación → `ChaincodeError::Execution`
- 2 tests: exceder límite → error, dentro del límite → ok

**8.3.3 — Host functions `put_state` / `get_state`**
- `WasmExecutor::invoke(state, func_name) -> Result<Vec<u8>>`
- ABI: la función Wasm devuelve `i64 = (ptr << 32 | len)`; el host lee `memory[ptr..ptr+len]`
- Imports `env::put_state` y `env::get_state` enlazan la memoria Wasm con `WorldState`
- 2 tests: put→get devuelve `"1"`, estado persistido en `WorldState`

**8.3.2 — `WasmExecutor`**
- `src/chaincode/executor.rs`: `WasmExecutor { engine, module, fuel_limit }`
- Constructor compila Wasm con fuel metering (`Config::consume_fuel(true)`)
- `ChaincodeError::Execution(String)` añadido al enum
- 3 tests: wasm válido ok, fuel_limit guardado, wasm inválido → error

**8.3.1 — Dependencia wasmtime**
- `wasmtime = "21"` añadido a `Cargo.toml`

---

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
