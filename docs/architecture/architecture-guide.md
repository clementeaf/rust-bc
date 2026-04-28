# Architecture

rust-bc implements the Hyperledger Fabric 2.5 transaction flow in Rust.

## Transaction flow

```
Client
  │
  ▼
Gateway (POST /gateway/submit)
  │
  ├─── 1. ENDORSE ────────────────────────────────────────┐
  │    Query discovery for endorsers                       │
  │    Send ProposalRequest to each peer via P2P           │
  │    Each peer simulates chaincode (Wasm)                │
  │    Each peer signs rwset hash → Endorsement            │
  │    Gateway collects responses, validates rwset match   │
  │◄──────────────────────────────────────────────────────┘
  │
  ├─── 2. ORDER ──────────────────────────────────────────┐
  │    Submit transaction to OrderingService               │
  │    Solo: immediate enqueue                             │
  │    Raft: propose via consensus, commit on majority     │
  │    Cut block (batch by size or timeout)                 │
  │◄──────────────────────────────────────────────────────┘
  │
  ├─── 3. VALIDATE (MVCC) ────────────────────────────────┐
  │    For each TX in block:                               │
  │      Compare read-set versions vs committed versions   │
  │      If conflict → mark TX invalid (block still saved) │
  │      If valid → apply write-set to world state         │
  │◄──────────────────────────────────────────────────────┘
  │
  ├─── 4. COMMIT ─────────────────────────────────────────┐
  │    Write block to BlockStore                           │
  │    Gossip block to peers (push, fanout=3)              │
  │◄──────────────────────────────────────────────────────┘
  │
  └─── 5. EVENTS ─────────────────────────────────────────┐
       Emit BlockCommitted event                           │
       Emit TransactionCommitted event (per TX)            │
       WebSocket + REST long-polling                       │
       ◄──────────────────────────────────────────────────┘
```

## Components

### Gateway (`src/gateway/mod.rs`)

Orchestrates the full lifecycle. Three endorsement paths:
- **Multi-peer**: P2P `ProposalRequest` to remote peers (when discovery + p2p_node configured)
- **Local simulation**: Wasm execution against world state
- **Policy-only**: Org registry check (no simulation)

### Ordering Service (`src/ordering/`)

Two backends behind the `OrderingBackend` trait:
- **Solo** (`OrderingService`): in-memory queue, immediate cut_block
- **Raft** (`RaftOrderingService`): tikv/raft consensus, networked via P2P `RaftMessage`

Selected at startup via `ORDERING_BACKEND=raft`.

### World State (`src/storage/world_state.rs`)

Versioned key-value store. Each `put` increments a monotonic version. Used for:
- Chaincode simulation (read/write keys)
- MVCC conflict detection (compare read versions)
- Rich queries via `get_range`

Backends:
- `MemoryWorldState` (default)
- `CouchDbWorldState` (persistent, `STATE_DB=couchdb`)

### Block Store (`src/storage/`)

Persists blocks, transactions, identities, credentials. Backends:
- `MemoryStore` (default)
- `RocksDbBlockStore` (persistent, `STORAGE_BACKEND=rocksdb`, 15 column families)

### P2P Network (`src/network/mod.rs`)

TCP + optional TLS. Message types:
- `ProposalRequest`/`ProposalResponse` — multi-peer endorsement
- `RaftMessage` — Raft consensus
- `OrderedBlock` — block broadcast from orderer
- `StateRequest`/`StateResponse` — pull-based sync
- `PrivateDataPush`/`PrivateDataAck` — private data dissemination
- `Alive` — gossip liveness

### Chaincode (`src/chaincode/`)

Wasm execution via Wasmtime. Lifecycle: `Installed → Approved → Committed`.

Host functions exposed to Wasm:
- `put_state(key_ptr, key_len, val_ptr, val_len)` — write to world state
- `get_state(key_ptr, key_len, buf_ptr, buf_len)` — read from world state

### Identity & Access Control

- **MSP** (`src/msp/`): Ed25519 identity, CRL revocation, role classification (Admin/Client/Peer/Orderer)
- **ACL** (`src/acl/`): Per-resource access control with endorsement policies
- **TLS** (`src/tls.rs`): Mutual TLS with certificate pinning, OCSP stapling

### Channels (`src/channel/`)

Isolated ledgers with per-channel block stores, config versioning, membership enforcement. Each channel starts with a genesis block containing the initial configuration.

### Private Data (`src/private_data/`)

Off-chain data shared only with member orgs. Features:
- Collection membership enforcement
- P2P dissemination to member peers
- TTL-based automatic purge (`blocks_to_live`)
- SHA-256 hash for on-chain integrity proof

### Discovery (`src/discovery/`)

Peer registry with endorsement plan resolution. Queries return peers that satisfy an endorsement policy for a given chaincode + channel.

## Data flow diagram

```
                    ┌──────────┐
                    │  Client  │
                    └────┬─────┘
                         │ REST/WebSocket
                    ┌────▼─────┐
                    │ Gateway  │
                    └────┬─────┘
              ┌──────────┼──────────┐
              ▼          ▼          ▼
         ┌────────┐ ┌────────┐ ┌────────┐
         │ Peer 1 │ │ Peer 2 │ │ Peer 3 │
         │ (org1) │ │ (org2) │ │ (org1) │
         └───┬────┘ └───┬────┘ └───┬────┘
             │   Gossip  │  Gossip  │
             └─────┬─────┘─────────┘
                   ▼
            ┌──────────────┐
            │ Orderer(s)   │
            │ Solo or Raft │
            └──────────────┘
```
