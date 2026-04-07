# MVP Roadmap

What remains to ship rust-bc as a deliverable Minimum Viable Product.

Last updated: 2026-04-07

---

## Current state

The core blockchain platform is feature-complete for a Fabric 2.5-style network:

- 71 E2E tests across 20 categories, ~1820 unit tests, CI fully green
- Endorsement policies, ordering (Solo + Raft), MVCC validation
- Channels with isolation, membership enforcement, config governance
- Gateway pipeline: endorse → order → commit → event emission
- Private data collections with ACL and TTL purge
- Chaincode lifecycle: install → approve → commit → Wasm simulation
- MSP roles (admin/peer/client), ACL deny-by-default with permissive fallback
- Gossip protocol, pull-based state sync, peer discovery, mutual TLS
- RocksDB persistence, state snapshots, pagination
- Docker deployment: 3 peers + orderer + Prometheus + Grafana

All 12 phases of the Fabric gap analysis are closed. Phase 13 (new implementations) is the remaining work.

---

## Block 1 — Required for MVP

Without these, the system is a demo, not a deliverable product.

### 1.1 Graceful shutdown

**Problem:** No SIGTERM/SIGINT handler. Killing the process can corrupt RocksDB WAL or drop in-flight transactions.

**Tasks:**

| # | Task | File | Detail |
|---|------|------|--------|
| 1 | Add `tokio::signal::ctrl_c()` + unix SIGTERM listener | `src/main.rs` | Use `tokio::select!` on the signal future alongside the HTTP server |
| 2 | Create `shutdown_notify: Arc<Notify>` in main | `src/main.rs` | Pass to all background tasks |
| 3 | Stop background tasks on signal | `src/main.rs` | Batch loop, gossip alive loop, pull-sync loop, purge loop, raft tick loop |
| 4 | Drain P2P connections | `src/network/mod.rs` | Stop TCP listener, wait for in-flight handlers |
| 5 | Flush and close RocksDB | `src/storage/adapters.rs` | `DB::flush()` before drop |
| 6 | Stop Actix HTTP server | `src/main.rs` | `Server::handle().stop(true)` for graceful drain |

**Effort:** Low (1 session)

### 1.2 Persistent service stores

**Problem:** Services use in-memory backends that lose all state on restart.

**Investigation result: 5 of 9 stores already have RocksDB implementations.**

Already done (just need wiring in `main.rs` when `STORAGE_BACKEND=rocksdb`):

| Service | RocksDB impl | CF name |
|---------|-------------|---------|
| OrgRegistry | `adapters.rs:618–668` | `organizations` |
| PrivateDataStore | `adapters.rs:810–842` | `private_{name}` |
| ChaincodePackageStore | `adapters.rs:769–788` | `chaincode_packages` |
| AclProvider | `adapters.rs:844–886` | `acls` |
| CrlStore | `adapters.rs:670–692` | `crl` |

Remaining work (4 stores):

| # | Task | File | Detail |
|---|------|------|--------|
| 1 | Add `Serialize/Deserialize` to `PrivateDataCollection` | `src/private_data/mod.rs:11` | Add `#[derive(serde::Serialize, serde::Deserialize)]` |
| 2 | Impl `CollectionRegistry` for `RocksDbBlockStore` | `src/storage/adapters.rs` | New CF `collections`, JSON ser/de, 3 methods: `register`, `get`, `list` |
| 3 | Add `Serialize/Deserialize` to `ChaincodeDefinition` + `ChaincodeStatus` | `src/chaincode/definition.rs:8` | Both structs need serde derives |
| 4 | Impl `ChaincodeDefinitionStore` for `RocksDbBlockStore` | `src/storage/adapters.rs` | New CF `chaincode_definitions`, 2 methods: `upsert_definition`, `get_definition` |
| 5 | Impl `PolicyStore` for `RocksDbBlockStore` | `src/storage/adapters.rs` | New CF `endorsement_policies`, 2 methods: `set_policy`, `get_policy` |
| 6 | Extract `DiscoveryService` storage into a `PeerRegistry` trait | `src/discovery/service.rs` | Separate data operations from query logic |
| 7 | Impl `PeerRegistry` for `RocksDbBlockStore` | `src/storage/adapters.rs` | New CF `peer_descriptors`, methods: `register`, `unregister`, `list`, `heartbeat` |
| 8 | Wire all 9 RocksDB impls in `main.rs` when `STORAGE_BACKEND=rocksdb` | `src/main.rs` | Replace `MemoryXxx::new()` with RocksDB-backed impls |
| 9 | Add startup log confirming which stores are persistent | `src/main.rs` | "Services: persistent (RocksDB)" vs "Services: in-memory" |

**Effort:** Medium (2-3 sessions). Tasks 1-5 follow the established pattern exactly. Tasks 6-7 need a small refactor. Task 8 is wiring.

### 1.3 Client SDK

**Problem:** No programmatic interface for Fabric-style operations.

**Investigation result: SDK is ~70% complete.** Core blockchain ops (wallets, blocks, transactions, smart contracts, mining, billing) work. Fabric-style operations are missing.

Already implemented in `sdk-js/src/client.ts`:
- `health()`, `createWallet()`, `getWalletBalance()`, `getBlocks()`, `getBlockByHash/Index()`
- `createTransaction()`, `getMempool()`, `mineBlock()`
- `deployContract()`, `getContract()`, `executeContractFunction()`
- `getPeers()`, `connectPeer()`, `syncBlockchain()`
- Response envelope unwrapping for both gateway and legacy formats

Missing methods to add:

| # | Task | Backend endpoint | Detail |
|---|------|-----------------|--------|
| 1 | `submitTransaction(chaincodeId, channelId, tx)` | `POST /gateway/submit` | Gateway-style transaction submission |
| 2 | `evaluate(chaincodeId, function)` | `POST /chaincode/simulate` | Read-only chaincode query |
| 3 | `registerOrg(org)` | `POST /store/organizations` | Wrap existing endpoint |
| 4 | `setPolicy(resourceId, policy)` | `POST /store/policies/{id}` | Wrap existing endpoint |
| 5 | `createChannel(id, opts)` | `POST /chain/channels` | Wrap existing endpoint |
| 6 | `putPrivateData(collection, key, value, orgId)` | `PUT /private-data/{collection}/{key}` | Add `X-Org-Id` header |
| 7 | `getPrivateData(collection, key, orgId)` | `GET /private-data/{collection}/{key}` | Add `X-Org-Id` header |
| 8 | `subscribeBlocks(channelId, opts)` | WebSocket `/events/blocks` | Needs `ws` dependency (axios doesn't support WS) |
| 9 | Add TLS/HTTPS support to `connect()` | Constructor config | `httpsAgent` with custom CA cert |
| 10 | Write Jest tests for all new methods | `sdk-js/tests/` | Currently empty; target 80%+ coverage |
| 11 | Update `README.md` and examples | `sdk-js/README.md`, `sdk-js/examples/` | Document new Fabric-style methods |

**Effort:** Medium (2 sessions). Tasks 1-7 are thin wrappers. Task 8 needs a WS library. Tasks 10-11 are tests and docs.

### 1.4 Documentation

**Problem:** No user-facing documentation. CLAUDE.md is developer-internal.

**Tasks:**

| # | Task | Output file | Content |
|---|------|-------------|---------|
| 1 | Quick-start guide | `docs/QUICK-START.md` | From `git clone` to 4-node network + first transaction in < 10 min |
| 2 | API reference | `docs/API-REFERENCE.md` | All endpoints, request/response with curl examples (source: E2E test script + handler code) |
| 3 | Deployment guide | `docs/DEPLOYMENT.md` | Env vars, TLS setup, Docker Compose, monitoring, production checklist |
| 4 | Update root README | `README.md` | Project overview, architecture diagram, links to docs |

**Effort:** Medium (1-2 sessions)

---

## Block 2 — Recommended

Difference between a working demo and a trustworthy product.

### 2.1 Native Rust CLI

**Problem:** `bcctl.sh` has no exit codes, no structured output, no tab completion.

**Tasks:**

| # | Task | Detail |
|---|------|--------|
| 1 | Create `src/bin/bcctl.rs` with `clap` derive | 14 subcommands matching current bash script |
| 2 | Add `--format json\|table` flag | Default: table; JSON for scripting |
| 3 | Add `--node` flag for target URL | Default: `https://localhost:8080` |
| 4 | Proper exit codes | 0 success, 1 error, 2 usage |
| 5 | Colored table output | `comfy-table` or similar |

**Effort:** Medium (1-2 sessions)

### 2.2 Health check with dependency verification

**Tasks:**

| # | Task | Detail |
|---|------|--------|
| 1 | Check RocksDB readable | Attempt `get_latest_height()` |
| 2 | Check peer connectivity | At least 1 peer if bootstrap nodes configured |
| 3 | Check ordering service | `pending_count()` doesn't error |
| 4 | Return degraded status with detail | `{ status: "degraded", checks: { rocksdb: "ok", peers: "none" } }` |

**Effort:** Low (1 session)

### 2.3 Explicit RocksDB failure handling

**Tasks:**

| # | Task | Detail |
|---|------|--------|
| 1 | If `STORAGE_BACKEND=rocksdb` and DB fails to open, exit with error | `src/main.rs` |
| 2 | Only fall back to memory when no backend specified | Remove silent fallback |

**Effort:** Low (< 1 session)

### 2.4 Mutex poison handling

**Tasks:**

| # | Task | Detail |
|---|------|--------|
| 1 | Create `fn lock_or_recover<T>(mutex) -> MutexGuard<T>` helper | Log poison, return inner |
| 2 | Replace 167 `.lock().unwrap()` calls | Project-wide search-replace |
| 3 | Evaluate which locks should abort vs recover | Critical stores: abort. Caches: recover. |

**Effort:** Medium (1 session)

---

## Block 3 — Post-MVP

Valuable but not blocking an initial release.

| Item | Description |
|------|-------------|
| Block explorer UI | React SPA with block list, TX detail, org list, real-time WebSocket updates |
| CouchDB world state | `WorldState` trait impl for CouchDB — rich queries via Mango selectors |
| TLS Identity Middleware | Extract peer identity from TLS certificate into request extensions |
| HSM integration | Hardware Security Module support for key storage |
| External chaincode | Run chaincode as external processes (Docker containers) |
| Hot certificate rotation | Rotate TLS certs without node restart |

---

## Suggested execution order

```
1.1 Graceful shutdown          ← quick win, high reliability impact
1.2 Persistent service stores  ← without this, restarts break everything
2.2 Health check               ← small, pairs well with 1.2
2.3 RocksDB failure handling   ← small, pairs well with 1.2
1.3 Client SDK                 ← usability gate for external consumers
1.4 Documentation              ← can run in parallel with SDK work
2.1 Native CLI                 ← polish
2.4 Mutex poison handling      ← resilience
```

---

## Success criteria

The MVP is shippable when:

- [ ] A 4-node network survives `docker compose restart` without data loss
- [ ] A developer can go from `git clone` to submitting a transaction in < 10 minutes using the SDK
- [ ] CI passes all unit + E2E tests on every push (already true)
- [ ] API reference documents every public endpoint with examples
- [ ] No silent data loss scenarios (RocksDB fallback, missing persistence)
