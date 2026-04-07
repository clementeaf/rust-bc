# Fabric 2.5 Comparison

How rust-bc compares to Hyperledger Fabric 2.5 as of 2026-04-07.

---

## Parity achieved

| Capability | Fabric 2.5 | rust-bc | Status |
|-----------|-----------|---------|--------|
| Endorse → Order → Commit | gRPC pipeline | HTTP/P2P pipeline | Parity |
| Endorsement policies (AnyOf, AllOf, NOutOf) | Protobuf policies | JSON policies | Parity |
| Raft ordering | etcd/raft cluster | tikv/raft in-process + P2P | MVP |
| MVCC validation | Block validation | `validate_rwset()` in commit path | Parity |
| Channels (multi-ledger) | Per-channel ledger | Per-channel BlockStore | Parity |
| Private data collections | Gossip + side-DB | P2P push + side-store + TTL purge | Parity |
| Chaincode lifecycle | Install → Approve → Commit | Same flow | Parity |
| Wasm chaincode execution | Docker/Go/Java/Node | Wasmtime + fuel/memory limits | Different but functional |
| External chaincode | Chaincode-as-a-service | HTTP client + runtime field | Parity |
| World state + history | LevelDB/CouchDB | Memory + CouchDB + key history | Parity |
| MSP roles (admin/peer/client) | X.509 cert-based | Header-based + TLS middleware | Functional parity |
| ACL enforcement | Per-resource policies | `enforce_acl()` deny-by-default | Parity |
| Gossip protocol | Alive + pull + anchor | Alive + pull-sync + anchor peers | Parity |
| Block events | Deliver/DeliverFiltered | WebSocket + filtered + private | Parity |
| State snapshots | Ledger snapshot | Create/restore + SHA-256 verification | Parity |
| Certificate pinning | Channel config | SHA-256 fingerprint allowlist | Parity |
| HSM signing | PKCS#11 BCCSP | Feature-gated `cryptoki` | Scaffold |
| Node SDK | fabric-network | @rust-bc/sdk (TypeScript) | Parity |
| CLI operator | peer CLI | bcctl (Rust binary, 14 commands) | Parity |
| Block explorer | Hyperledger Explorer | Next.js app | Parity |
| Docker deployment | docker-compose samples | 3 peers + orderer + Prometheus + Grafana | Parity |
| Hot cert rotation | Configurable | SIGHUP + periodic reload | Parity |
| Persistent storage | LevelDB/CouchDB | RocksDB (8 service stores + blocks) | Parity |
| Graceful shutdown | Orderer/peer shutdown | SIGTERM/SIGINT with connection drain | Parity |

---

## Remaining gaps

### Critical (blocks production enterprise use)

| Gap | Description | Impact | Effort |
|-----|------------|--------|--------|
| Raft not multi-process | Raft nodes share memory within a single process. No crash tolerance between orderers. A single process crash loses all ordering state. | High | High |
| No gRPC protocol | All communication is HTTP/JSON, not Protobuf/gRPC. Incompatible with native Fabric SDKs, peers, and tooling. | High (if interop needed) | High |
| MSP based on headers, not X.509 | Identity comes from `X-Org-Id` / `X-Msp-Role` headers, not extracted from verified client certificate subject. TLS middleware scaffolded but not enforcing real X.509 identity chain. | Medium | Medium |

### Moderate

| Gap | Description | Impact | Effort |
|-----|------------|--------|--------|
| No Fabric CA | No enrollment/registration service for identities. Certificates managed externally. | Medium | High |
| No automatic service discovery | Discovery is manual peer registration, not gossip-based automatic discovery of chaincode/channel capabilities. | Medium | Medium |
| No VSCC/ESCC | No configurable validation/endorsement system chaincode per chaincode definition. Validation is hardcoded. | Low | Medium |

### Low priority

| Gap | Description | Impact | Effort |
|-----|------------|--------|--------|
| No Kafka ordering | Only Solo and Raft backends. Kafka was deprecated in Fabric 2.x anyway. | None | — |
| External chaincode protocol | Uses HTTP POST, not Fabric's CDS (Chaincode Development Shim) protocol. | Low | Low |
| DiscoveryService not persistent | Peer registrations lost on restart (only in-memory store without RocksDB impl). | Low | Low |

---

## Verdict

### As an MVP / Proof of Concept: ~90% parity

The full Fabric transaction lifecycle works end-to-end: endorsement with configurable policies, Raft ordering, MVCC validation, channel isolation, private data with ACL, chaincode lifecycle, gossip protocol, block events, and state snapshots. Backed by 2034 unit tests, 71 E2E tests, and a fully green CI pipeline.

### As a production enterprise replacement: ~65% parity

Three critical gaps prevent drop-in replacement:

1. **Raft multi-process** — Orderers need to be independent processes with persistent Raft log for real crash tolerance.
2. **gRPC protocol** — Required for interoperability with existing Fabric networks, SDKs, and tooling.
3. **X.509-based MSP** — Real certificate chain validation instead of header-based identity.

### Recommended next steps for production

```
1. Raft multi-process ordering    ← highest impact, enables real fault tolerance
2. X.509 MSP enforcement          ← security requirement for enterprise
3. Fabric CA integration           ← identity lifecycle management
4. gRPC protocol                   ← only if interop with Fabric networks is required
```

---

## Test coverage summary

| Category | Count |
|----------|-------|
| Unit + integration tests | 2034 |
| E2E tests (Docker network) | 71 |
| CouchDB integration tests | 3 |
| CI status | All green |
| Test failures | 0 |
