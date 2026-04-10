# Enterprise Distributed Ledger Platform

A permissioned, modular blockchain built in Rust for enterprise workloads.

---

## What is this

A distributed ledger technology (DLT) platform designed for organizations that need tamper-proof record keeping, multi-party trust, and data privacy. It follows the same architectural model as Hyperledger Fabric but is written from scratch in Rust for performance and memory safety.

It is not a cryptocurrency. It is infrastructure for business networks where participants are known and accountable.

---

## Why enterprises need a permissioned blockchain

Public blockchains (Bitcoin, Ethereum) are open to anyone. That model does not fit regulated industries where:

- Participants must be identified and authorized
- Transaction data must remain confidential between parties
- Throughput must be predictable and low-latency
- Regulatory compliance (audit trails, data residency) is mandatory

A permissioned blockchain solves these problems by restricting participation to known organizations while preserving the core benefits: immutability, distributed consensus, and a shared source of truth.

---

## Architecture

### Execute-Order-Validate

Transactions follow a three-phase pipeline:

1. **Execute (Endorse)** — Client sends a transaction proposal to endorsing peers. Each peer simulates the smart contract against its local state and returns a signed result.
2. **Order** — Endorsed transactions are sent to the ordering service, which establishes a total order and packages them into blocks.
3. **Validate & Commit** — All peers validate the ordered transactions (endorsement policy check, read-write conflict detection) and commit the block to their ledger.

This model allows parallel execution, deterministic ordering, and flexible endorsement policies.

### Core components

| Component | Role |
|---|---|
| **Peer nodes** | Execute smart contracts, maintain the ledger, endorse proposals |
| **Ordering service** | Raft-based consensus, block creation, crash fault tolerance |
| **Gateway** | Client-facing entry point for the endorse-order-commit pipeline |
| **Channels** | Isolated ledgers shared between subsets of organizations |
| **Chaincode** | Smart contracts executed in WebAssembly or as external services |
| **World state** | Current key-value state backed by RocksDB or CouchDB |

### Node types

```
                    ┌─────────────┐
 Client ──────────► │   Gateway   │
                    └──────┬──────┘
                           │
              ┌────────────┼────────────┐
              ▼            ▼            ▼
         ┌────────┐  ┌────────┐  ┌──────────┐
         │ Peer 1 │  │ Peer 2 │  │ Orderer  │
         │ (org1) │  │ (org2) │  │  (Raft)  │
         └────────┘  └────────┘  └──────────┘
```

---

## Privacy and confidentiality

### Channels

Each channel is a separate ledger with its own set of member organizations. Data on one channel is invisible to organizations on another. A single network can host many channels simultaneously.

Use case: In a supply chain, a manufacturer and its two suppliers can share one channel, while the manufacturer and its distributors share a different one. Neither group sees the other's transactions.

### Private data collections

Within a channel, private data collections allow subsets of organizations to share confidential data (e.g., pricing, PII) without exposing it to all channel members. Only authorized organizations receive the actual data; the rest see only a hash on the ledger.

### Access control

- **Deny-by-default ACL** — Requests without valid identity are rejected
- **Organization-based policies** — Endorsement, read, and write permissions are scoped per organization
- **MSP (Membership Service Provider)** — X.509 certificate-based identity with organizational unit support
- **Mutual TLS** — All peer-to-peer and client-to-node communication uses mTLS

---

## Consensus

The ordering service uses **Raft** for crash fault tolerance:

- Persistent Raft log (survives process crashes and restarts)
- Leader election with automatic failover
- Configurable cluster size (1, 3, 5 orderer nodes)
- Each orderer maintains its own RocksDB-backed Raft state

Raft provides strong consistency and finality. Once a block is committed by the ordering service, it is final — there are no forks or probabilistic confirmation.

---

## Smart contracts (Chaincode)

Two runtime models:

| Model | Description | Use case |
|---|---|---|
| **WebAssembly** | Chaincode compiled to Wasm, executed in a sandboxed runtime (Wasmtime) | Deterministic, portable, no external dependencies |
| **External (Chaincode-as-a-Service)** | Chaincode runs as an external HTTP service; the peer delegates invocation | Legacy systems, complex dependencies, any language |

Smart contracts interact with the world state through a read-write set model. MVCC (Multi-Version Concurrency Control) detects conflicts at commit time.

---

## Endorsement policies

Endorsement policies define how many and which organizations must sign a transaction for it to be valid. Policies can be set at the chaincode level or per individual state key.

Examples:
- `AnyOf(org1, org2, org3)` — At least one organization must endorse
- `AllOf(org1, org2)` — Both organizations must endorse
- `NOutOf(2, [org1, org2, org3])` — Any two of three must endorse

Key-level policies override chaincode-level policies for sensitive state entries.

---

## Post-quantum cryptography

The platform supports **ML-DSA-65** (FIPS 204), a NIST-standardized post-quantum digital signature algorithm. This protects against future quantum computing threats.

| Property | Ed25519 (classic) | ML-DSA-65 (post-quantum) |
|---|---|---|
| Signature size | 64 bytes | 3,309 bytes |
| Public key size | 32 bytes | 1,952 bytes |
| NIST standard | — | FIPS 204 (Aug 2024) |
| Security level | 128-bit classical | Level 3 (quantum-safe) |

The signing algorithm is selected per node via the `SIGNING_ALGORITHM` environment variable. Both algorithms coexist — the network can operate in mixed mode during a migration period.

All signature fields across the stack (blocks, endorsements, proposals, gossip messages) use variable-length encoding to accommodate both classical and post-quantum signatures.

---

## Storage

### Ledger

The block ledger is stored in **RocksDB** with zero-padded keys for efficient range scans. Column families separate blocks, transactions, identities, credentials, and secondary indexes.

### World state

Current state is queryable through a key-value interface backed by:
- **RocksDB** — Default, embedded, high performance
- **CouchDB** — Optional, enables rich JSON queries

MVCC versioning tracks state changes across block heights for conflict detection.

### Persistence

All services (policy store, collection registry, chaincode definitions, Raft log) persist to RocksDB when configured. The node exits immediately if the database fails to open — no silent fallback to in-memory.

---

## Operations

### Monitoring

- **Prometheus metrics** — Block count, transaction throughput, mempool size, network latency
- **Grafana dashboards** — Pre-configured in the Docker deployment
- **Health endpoint** — `/api/v1/health` reports storage, peer, and ordering status

### Audit trail

Every API request is logged to an append-only audit store with:
- Timestamp, HTTP method, path
- Organization ID and client IP
- Response status and duration
- Trace ID for correlation

### Deployment

The platform ships with Docker Compose for multi-node networks:

| Component | Default ports |
|---|---|
| Peer node 1 | 8080 (API), 8081 (P2P) |
| Peer node 2 | 8082 (API), 8083 (P2P) |
| Peer node 3 | 8084 (API), 8085 (P2P) |
| Orderer | 8086 (API), 8087 (P2P) |
| Prometheus | 9090 |
| Grafana | 3000 |

TLS certificates are generated automatically. Graceful shutdown drains connections and flushes storage.

### Operator CLI

```bash
bcctl.sh status        # Health and block height across all nodes
bcctl.sh consistency   # Compare chain tips for divergence
bcctl.sh mine          # Create a wallet and mine a block
bcctl.sh orgs          # List registered organizations
bcctl.sh logs node1    # Tail container logs
```

---

## Developer tools

### REST API

68 endpoints documented with OpenAPI 3.0. Swagger UI available at `/swagger`.

Covers: wallets, transactions, blocks, channels, organizations, endorsement policies, private data, chaincode lifecycle, identities, credentials, events, discovery, and health.

### JavaScript/TypeScript SDK

```bash
npm install rust-bc-sdk
```

Operations: submit transactions, evaluate queries, register organizations, manage channels, read/write private data. 21 automated tests.

### Block explorer

A Next.js web application for browsing blocks, transactions, and network status.

---

## Use cases

### Supply chain traceability

Track goods from origin to destination across multiple organizations. Each participant endorses their step. Channels isolate competing supply chains. Private data protects commercial terms.

### Food safety and wine provenance

Record temperature, handling, and custody transfers on an immutable ledger. Regulatory auditors get read access without seeing commercial data. Post-quantum signatures future-proof the chain of custody.

### Financial document management

Multi-party workflows (loan origination, trade finance) where each institution endorses document transitions. MVCC prevents double-processing. Audit trails satisfy compliance requirements.

### Healthcare data sharing

Hospitals and insurers share patient records on private channels. Private data collections ensure only authorized parties access PII. Endorsement policies enforce consent workflows.

### Government and public records

Land registries, permits, and certifications recorded on an immutable ledger. Organizations (agencies) endorse state transitions. Deny-by-default ACL prevents unauthorized access.

---

## Technical summary

| Feature | Implementation |
|---|---|
| Language | Rust |
| Consensus | Raft (crash fault tolerant, persistent log) |
| Smart contracts | WebAssembly (Wasmtime) + external chaincode |
| Storage | RocksDB (default) + CouchDB (optional world state) |
| Identity | X.509 MSP + DID + Ed25519/ML-DSA-65 signing |
| Privacy | Channels + private data collections + mTLS |
| API | REST (68 endpoints) + OpenAPI 3.0 + Swagger UI |
| SDK | JavaScript/TypeScript (npm) |
| Monitoring | Prometheus + Grafana |
| Deployment | Docker Compose (multi-node) |
| Post-quantum | ML-DSA-65 (FIPS 204, NIST Level 3) |
| Tests | 986 unit tests + 42 E2E assertions |

---

## How it compares to Hyperledger Fabric

This platform follows the same architectural model as Hyperledger Fabric:

| Capability | Hyperledger Fabric | This platform |
|---|---|---|
| Execute-order-validate | Yes | Yes |
| Channels | Yes | Yes |
| Private data collections | Yes | Yes |
| Pluggable consensus | Raft | Raft |
| Chaincode runtime | Docker/Go/Java/Node | WebAssembly + external |
| World state | CouchDB/LevelDB | RocksDB + CouchDB |
| Identity (MSP) | X.509 | X.509 + DID |
| Post-quantum crypto | No | ML-DSA-65 (FIPS 204) |
| Language | Go | Rust |
| Memory safety | Runtime (GC) | Compile-time (ownership) |

Key differentiators:
- **Rust** — No garbage collector, predictable latency, memory safety without runtime overhead
- **WebAssembly chaincode** — Portable, sandboxed, language-agnostic compilation target
- **Post-quantum ready** — ML-DSA-65 signing integrated across the full stack
- **Single binary** — One compiled binary includes all node roles (peer, orderer, gateway)
