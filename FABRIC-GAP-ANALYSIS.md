# Fabric 2.5 Gap Analysis

Comparison of rust-bc against [Hyperledger Fabric 2.5](https://hyperledger-fabric.readthedocs.io/en/release-2.5/).

Last updated: 2026-04-06

---

## 1. Verified E2E

42 tests, 0 fail, 0 skip. Run with `./scripts/e2e-test.sh`.

| Category | What's tested | Tests |
|---|---|---|
| Health & connectivity | 4 nodes healthy, 3 P2P peers each via mutual TLS | 5 |
| Organizations | Create org1/org2, list | 3 |
| Endorsement policies | Set NOutOf policy, read back, discovery policy | 3 |
| Channels | Create channel, list, write TX to channel, verify isolation from default | 4 |
| Block propagation | Mine on node1, verify same hash on node2/node3 | 5 |
| Transactions | Create wallet, fund via mining, submit to mempool, mine into block | 3 |
| Private data | Register collection, write as member, read as member, deny non-member | 4 |
| Discovery | Register peers, query endorsers for chaincode, query channel peers | 3 |
| Gateway | Submit TX through endorse-order-commit pipeline, verify tx_id and block height | 3 |
| Chain integrity | Verify chain on all 3 peers (status 200, block count) | 3 |
| Observability | Prometheus metrics, Prometheus scraping, Grafana healthy | 3 |
| Store CRUD | Write/read identity, write/read credential | 4 |

---

## 2. Implemented, not yet E2E tested

Code exists and passes unit tests. Needs integration testing against the Docker network.

### 2.1 Chaincode lifecycle

| Step | Endpoint | Status |
|---|---|---|
| Install Wasm binary | `POST /chaincode/install` | Handler exists |
| Approve per-org | `POST /chaincode/{id}/approve` | Handler exists |
| Commit (majority) | `POST /chaincode/{id}/commit` | Handler exists |
| Simulate | `POST /chaincode/{id}/simulate` | Handler exists |

Source: `src/chaincode/`, `src/api/handlers/chaincode.rs`

### 2.2 MVCC conflict detection

`validate_rwset()` compares read-set versions against current world state. Conflicting TXs are marked `mvcc_conflict` (Fabric behavior: block accepted, TX invalidated).

Source: `src/transaction/mvcc.rs`

### 2.3 Raft ordering

Leader election and log replication via `raft` crate v0.7 (TiKV). Configured via `ORDERING_BACKEND=raft`.

Source: `src/ordering/raft_service.rs`

### 2.4 Channel configuration updates

`ConfigTransaction` supports: AddOrg, RemoveOrg, SetPolicy, SetAcl, SetBatchSize, SetAnchorPeer. Validates signatures against modification policy.

Source: `src/channel/config.rs`

### 2.5 WebSocket block events

`EventBus` with `tokio::sync::broadcast` fan-out. Supports filtered blocks and private data delivery.

Source: `src/events/`, `src/api/handlers/events.rs`

### 2.6 MSP revocation

CRL persistence in RocksDB. Revocation check integrated in endorsement validation.

Source: `src/msp/`

### 2.7 ACL enforcement

`AclProvider` with memory and RocksDB backends. `check_access()` resolves policy reference and evaluates.

Source: `src/acl/`

### 2.8 Crash recovery

RocksDB column families persist blocks, transactions, identities, credentials, world state.

Source: `src/storage/adapters.rs`

### 2.9 Snapshots

`create_snapshot()` serializes world state with SHA-256 integrity hash. `restore_snapshot()` verifies and rebuilds.

Source: `src/storage/snapshot.rs`

### 2.10 Key-level endorsement policies

Per-key policies override chaincode-level policy. Integrated in Gateway simulation path.

Source: `src/endorsement/key_policy.rs`

---

## 3. Not implemented

### High impact

| Gap | Fabric | rust-bc | Notes |
|---|---|---|---|
| Client SDKs | Go, Node.js, Java | HTTP/curl only | No programmatic client for applications |
| CouchDB rich queries | JSON field queries on world state | RocksDB key-value only | Limits query flexibility for complex apps |
| HSM (PKCS#11) | Hardware key storage | Trait interface, no hardware binding | Required for production deployments |

### Medium impact

| Gap | Fabric | rust-bc | Notes |
|---|---|---|---|
| Native CLI (`peer`) | Channel join, chaincode install, invoke | `bcctl.sh` (bash, 14 commands) | Functional but not portable |
| Channel-level MSP | Per-channel membership rules | Global MSP only | Limits multi-tenant isolation |
| Gossip anti-entropy | Full state digest + verified pull | Basic pull-sync | Not verified under partition |
| Block explorer | Hyperledger Explorer web UI | Not implemented | Visual chain inspection |

### Low impact

| Gap | Fabric | rust-bc | Notes |
|---|---|---|---|
| `configtxgen` | Genesis block generator from YAML | Genesis via API | Different approach, same result |
| `cryptogen` | Org crypto material generator | `generate-tls.sh` + runtime Ed25519 | Different identity model |
| Gossip leader election | Org-level leader for distribution | Not implemented | Optimization, not correctness |
| Caliper benchmarks | Standardized performance framework | Criterion micro-benchmarks | Different scope |
| Cross-chain (Cacti/FireFly) | Interoperability bridges | Not implemented | Ecosystem feature |

---

## 4. Task backlog (detailed)

### P0 — Code exists, high value

#### P0.1 Chaincode lifecycle E2E

**Research findings:**
- Install stores raw Wasm bytes but does NOT create a `ChaincodeDefinition`. Approve expects a definition to exist already — **gap: install must auto-create it, or a seeding step is needed**.
- Simulate does NOT require committed status. It only needs the Wasm package in the store.
- Wasm module must export a function matching the `function` field (e.g. `"run"`) with signature `() -> i64`. Return encodes `(ptr << 32) | len` pointing to result in linear memory.
- Host functions registered in module `"env"`: `put_state`, `get_state`, `set_event`, `get_history_for_key`, `invoke_chaincode`, `set_key_endorsement_policy`.
- Executor tests use inline WAT modules. No external `.wasm` files exist in the repo.

**Minimal WAT for simulate (proven in unit tests):**
```wat
(module
  (import "env" "put_state" (func $put (param i32 i32 i32 i32) (result i32)))
  (import "env" "get_state" (func $get (param i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 0) "x")
  (data (i32.const 4) "1")
  (func (export "run") (result i64)
    (drop (call $put (i32.const 0) (i32.const 1) (i32.const 4) (i32.const 1)))
    (i64.or
      (i64.shl (i64.const 8) (i64.const 32))
      (i64.extend_i32_u
        (call $get (i32.const 0) (i32.const 1) (i32.const 8) (i32.const 64))))
  )
)
```

**E2E steps (install + simulate — no code change needed):**

| # | Action | Endpoint | Expected |
|---|---|---|---|
| 1 | Install | `POST /chaincode/install?chaincode_id=basic&version=1.0` body=WAT, `Content-Type: application/octet-stream` | 200, `{ chaincode_id, version, size_bytes }` |
| 2 | Simulate | `POST /chaincode/basic/simulate?version=1.0` body `{"function":"run"}` | 200, `{ result, rwset }` |
| 3 | Verify rwset | Parse response | `writes` contains key `x` with value `1` |

**Full lifecycle (approve+commit) — code change needed:**

| # | Task | Effort |
|---|---|---|
| 1 | Make install handler auto-create `ChaincodeDefinition` with `Installed` status and default `AnyOf([])` policy | Small — 10 lines in `src/api/handlers/chaincode.rs` |
| 2 | E2E: install → approve org1 → approve org2 → commit → simulate | Test only |

**Files:** `src/api/handlers/chaincode.rs`, `src/chaincode/executor.rs`, `src/chaincode/mod.rs`

---

#### P0.2 Crash recovery E2E

**Research findings:**
- RocksDB opens the DB handle at startup but does NOT load blocks into memory. Data persists on disk and is readable via store API, but the app doesn't reconstruct state from it.
- Legacy `Blockchain` struct loads from file-backed `BlockStorage` (separate from RocksDB). These are independent systems.
- `org_registry`, `policy_store`, `discovery_service`, `collection_registry` are all in-memory — **lost on restart**.
- Pull-sync (`start_pull_sync_loop`) is NOT auto-triggered at startup.
- RocksDB CFs that persist: `blocks`, `transactions`, `identities`, `credentials`, `organizations`, `world_state`, `chaincode_packages`, `acls`, `channel_configs`, `crl`, `key_history`.

**E2E steps (no code change needed — tests what RocksDB actually persists):**

| # | Action | Expected |
|---|---|---|
| 1 | Write transaction via `POST /store/transactions` | 200 |
| 2 | Write identity via `POST /store/identities` | 200 |
| 3 | Record height via `GET /store/blocks/latest` | Save baseline |
| 4 | `docker compose stop node1` | Node down |
| 5 | `docker compose start node1`, poll `/health` | Node back |
| 6 | `GET /store/transactions/{id}` | Same data (RocksDB persisted) |
| 7 | `GET /store/identities/{did}` | Same data |
| 8 | `GET /store/blocks/latest` | Same height |

**What will NOT survive (expected, document in test):**
- In-memory orgs, policies, discovery peers, private data collections
- Legacy blockchain blocks (unless BlockStorage file path is on volume)

**Files:** `docker-compose.yml` (volumes), `src/storage/adapters.rs`

---

#### P0.3 WebSocket block events E2E

**Research findings:**
- Long-polling: `GET /events/blocks?from_height=N` returns block array. No external tools needed.
- WebSocket: upgrades connection, client sends JSON filter `{channel_id, chaincode_id, start_block, client_id, ack}`, server pushes `BlockCommitted`, `TransactionCommitted`, `ChaincodeEvent`.
- EventBus is initialized in AppState. Gateway does NOT emit events (its `event_bus` field is `None`).
- Events would only fire if something in the commit path calls `event_bus.send()`.

**E2E steps (long-polling — bash only, no external tools):**

| # | Action | Endpoint | Expected |
|---|---|---|---|
| 1 | Baseline | `GET /stats` → block_count=N | |
| 2 | Mine | `POST /mine` | block N+1 |
| 3 | Poll | `GET /events/blocks?from_height=N` | Array with ≥1 block |
| 4 | Mine 2 more | `POST /mine` x2 | |
| 5 | Poll from N | `GET /events/blocks?from_height=N` | Array with 3 blocks |
| 6 | Poll from tip | `GET /events/blocks?from_height=$((N+3))` | Empty array |

**Code change needed for real-time WebSocket events:**

| # | Task | Effort |
|---|---|---|
| 1 | Attach `event_bus` to Gateway: `gateway.event_bus = Some(event_bus.clone())` in `main.rs` | Tiny |
| 2 | Verify `Gateway::submit()` calls `event_bus.send(BlockCommitted{...})` after writing block | Read `src/gateway/mod.rs:200-220` |

**Files:** `src/api/handlers/events.rs`, `src/events/mod.rs`, `src/gateway/mod.rs`, `src/main.rs`

---

### P1 — Code exists, medium value

#### P1.1 Raft multi-orderer

**Research findings:**
- Uses `raft` crate v0.7 (TiKV). Nodes are in-process only — NO network transport built in.
- `RaftOrderingService::new(id, peers, batch, timeout)` takes `peers: Vec<u64>` (numeric IDs, not addresses).
- `raft_transport.rs` provides `encode_raft_msg`/`decode_raft_msg` (protobuf via prost) but no TCP/gRPC layer.
- Tests use `route_bytes(&mut nodes, ticks)` to deliver messages in-process.
- **Raft does NOT work across Docker containers** without implementing a network transport layer.

**Code changes needed:**

| # | Task | Effort |
|---|---|---|
| 1 | Add `Message::RaftMessage(Vec<u8>)` variant to P2P `Message` enum | Small |
| 2 | In `process_message`, deliver `RaftMessage` to local `RaftNode::step()` | Small |
| 3 | Create tick loop: call `tick_and_collect()`, send outbound messages to peers via P2P | Medium |
| 4 | Map Raft node IDs to P2P peer addresses (env var or discovery) | Small |
| 5 | Add orderer2/orderer3 to `docker-compose.yml` | Small |
| 6 | E2E: submit TX → block cut → replicated; kill leader → failover | Medium |

**Files:** `src/ordering/raft_service.rs`, `src/ordering/raft_transport.rs`, `src/network/mod.rs`

---

#### P1.2 MVCC conflict detection

**Research findings:**
- `Gateway::submit()` does NOT call `validate_rwset`. It submits to ordering, cuts a block, writes it — no MVCC.
- `validate_rwset` is only called from `commit_block()` in `src/transaction/mvcc.rs` — a utility function not wired to any active code path.
- No HTTP endpoint triggers MVCC validation.
- **MVCC is implemented as a library but not integrated into the transaction commit pipeline.**

**Code changes needed:**

| # | Task | Effort |
|---|---|---|
| 1 | Initialize `world_state` in `main.rs` (`MemoryWorldState::new()`) | Tiny |
| 2 | Attach `world_state` to Gateway (`gateway.world_state = Some(...)`) | Tiny |
| 3 | In `Gateway::submit()`, after `cut_block()`, call `commit_block()` on the block's TXs to apply MVCC | Small |
| 4 | Return per-TX status (`committed` or `mvcc_conflict`) in gateway response | Small |
| 5 | E2E: write key, submit two conflicting TXs, verify one gets conflict | Medium |

**Files:** `src/gateway/mod.rs`, `src/transaction/mvcc.rs`, `src/main.rs`

---

#### P1.3 Channel configuration updates

**Research findings:**
- Handler validates endorsement signatures via `validate_config_tx()`.
- New channels get `ChannelConfig::default()` with `endorsement_policy: AnyOf([])` — **accepts any signer, including empty signatures**.
- Modification policy falls back to channel's `endorsement_policy`.
- **Empty `signatures: []` works on freshly created channels** because `AnyOf([])` is satisfied by 0 orgs.

**E2E steps (no code change needed):**

| # | Action | Endpoint | Expected |
|---|---|---|---|
| 1 | Create channel | `POST /channels` `{"channel_id":"govtest"}` | 200 |
| 2 | Get config | `GET /channels/govtest/config` | version=0, `AnyOf([])` |
| 3 | Add org1 | `POST /channels/govtest/config` body: `{"tx_id":"cfg-1","channel_id":"govtest","updates":[{"AddOrg":"org1"}],"signatures":[],"created_at":1000}` | 200, version=1 |
| 4 | Verify | `GET /channels/govtest/config` | member_orgs=["org1"] |
| 5 | Set batch size | Same with `{"SetBatchSize":50}` | 200, version=2 |
| 6 | History | `GET /channels/govtest/config/history` | 3 entries |

**Files:** `src/api/handlers/channels.rs`, `src/channel/config.rs`

---

#### P1.4 MSP revocation + ACL enforcement

**Research findings:**
- `MemoryCrlStore` does NOT exist as a reusable struct — only inline test implementations. Must be extracted.
- `MemoryAclProvider` exists and is ready to use (`src/acl/provider.rs`).
- Both `crl_store` and `acl_provider` are `None` in `AppState`.

**Code changes needed:**

| # | Task | Effort |
|---|---|---|
| 1 | Extract `MemoryCrlStore` from test inline impl to `src/msp/mod.rs` | Small — ~15 lines |
| 2 | Wire `crl_store: Some(Arc::new(MemoryCrlStore::new()))` in `main.rs` | Tiny |
| 3 | Wire `acl_provider: Some(Arc::new(MemoryAclProvider::new()))` in `main.rs` | Tiny |

**MSP E2E steps:**

| # | Action | Endpoint | Expected |
|---|---|---|---|
| 1 | Get MSP info | `GET /msp/Org1MSP` | `{ msp_id, crl_size: 0 }` |
| 2 | Revoke serial | `POST /msp/Org1MSP/revoke` `{"serial":"cert-001"}` | 200 |
| 3 | Verify | `GET /msp/Org1MSP` | crl_size=1 |
| 4 | Idempotent | Same revoke | crl_size still 1 |

**ACL E2E steps:**

| # | Action | Endpoint | Expected |
|---|---|---|---|
| 1 | Set ACL | `POST /acls` `{"resource":"peer/Invoke","policy_ref":"mycc"}` | 200 |
| 2 | List | `GET /acls` | 1 entry |
| 3 | Get | `GET /acls/peer%2FInvoke` | `{ resource, policy_ref }` |

**Files:** `src/msp/mod.rs`, `src/acl/provider.rs`, `src/main.rs`

---

#### P1.5 Snapshots

**Research findings:**
- `create_snapshot` requires `world_state` — returns 404 if `None`.
- Snapshots serialize world state key-value pairs (via `get_range`), not blocks. Store is used only for height metadata.
- `world_state` is `None` in current `AppState`.

**Code change needed:**

| # | Task | Effort |
|---|---|---|
| 1 | Initialize `world_state` in `main.rs` | Tiny |

**E2E steps:**

| # | Action | Endpoint | Expected |
|---|---|---|---|
| 1 | Create snapshot | `POST /snapshots/default` | 200, snapshot_id |
| 2 | List | `GET /snapshots/default` | Array with 1 entry |
| 3 | Download | `GET /snapshots/default/{id}` | Binary `.snap` file |

**Files:** `src/main.rs`, `src/storage/world_state.rs`, `src/api/handlers/snapshots.rs`

---

### P2 — New implementation needed

#### P2.1 Node.js SDK

Thin HTTP client wrapping the REST API.

**Scope:** `connect`, `submitTransaction`, `registerOrg`, `setPolicy`, `createChannel`, `subscribeBlocks`, `putPrivateData`.

**Effort:** ~500 lines TypeScript.

#### P2.2 Native Rust CLI

Replace `bcctl.sh` with compiled binary using `clap`. Same 14 commands, `--format json`, proper exit codes.

**Effort:** ~800 lines. Uses `reqwest`.

#### P2.3 CouchDB state adapter

Implement `WorldState` trait for CouchDB. `put` → doc, `get_range` → Mango selector. New env var `STATE_DB=couchdb`.

**Effort:** ~400 lines + tests.

#### P2.4 Block explorer UI

React/Next.js SPA. Block list, TX detail, org list. Real-time via WebSocket. Docker service.

**Effort:** ~2000 lines frontend.
