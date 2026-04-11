# Security Audit Package

This document prepares the codebase for an external security audit. It describes the attack surface, critical code paths, and areas that require focused review.

Provide this document to the auditing firm before engagement to reduce billable hours.

---

## Scope

| Area | In scope | Files |
|---|---|---|
| Cryptographic operations | Yes | `src/identity/signing.rs`, `src/identity/hsm.rs` |
| Authentication and authorization | Yes | `src/api/middleware.rs`, `src/acl/`, `src/msp/` |
| API input handling | Yes | `src/api/handlers/` (all files) |
| Consensus protocol | Yes | `src/ordering/`, `src/consensus/` |
| P2P network protocol | Yes | `src/network/mod.rs`, `src/network/gossip.rs` |
| Private data handling | Yes | `src/private_data/`, `src/api/handlers/private_data.rs` |
| Smart contract sandbox | Yes | `src/chaincode/executor.rs` |
| TLS configuration | Yes | `src/tls.rs`, `src/pki.rs` |
| Block explorer (Next.js) | Out of scope | `block-explorer/` |
| JS SDK | Out of scope | `sdk-js/` |

## Architecture overview

```
Internet --> TLS termination --> Actix-Web API --> Handlers --> AppState
                                    |                              |
                                 Middleware                   BlockStore (RocksDB)
                              (Correlation ID)                WorldState
                              (TLS Identity)                  Chaincode Executor
                              (Audit Trail)                   Gateway Pipeline
                              (Input Validation)              Raft Ordering
                              (Rate Limiting)
```

All external traffic enters through the Actix-Web HTTP server. The P2P layer accepts TCP connections from other nodes (mTLS required).

## Critical code paths

### 1. Signature verification (`HIGH`)

**Files:** `src/identity/signing.rs`, `src/endorsement/validator.rs`, `src/models.rs`

**Risk:** If signature verification can be bypassed, an attacker can forge transactions, endorsements, or blocks.

**What to check:**
- `SigningProvider::verify()` — does it correctly reject corrupted signatures?
- `verify_endorsement()` — does it check the right payload hash?
- `Transaction::verify_signature()` — does the size-based algorithm detection have edge cases?
- ML-DSA-65 `from_bytes` — does it accept malformed inputs silently?

### 2. Wasm chaincode sandbox (`HIGH`)

**Files:** `src/chaincode/executor.rs`

**Risk:** A malicious chaincode could escape the sandbox, access host memory, or exhaust resources.

**What to check:**
- Fuel limits — can they be bypassed?
- Memory limits — does `StoreLimits` actually prevent OOM?
- Host functions (`put_state`, `get_state`) — can pointer/length arguments cause out-of-bounds reads?
- `read_str` and `read_bytes` helpers — do they validate bounds correctly?
- Cross-chaincode invocation — can depth limits be circumvented?
- Can a chaincode write to another chaincode's state namespace?

### 3. ACL and channel membership (`HIGH`)

**Files:** `src/acl/checker.rs`, `src/api/handlers/channels.rs`, `src/api/middleware.rs`

**Risk:** Unauthorized access to channels or private data.

**What to check:**
- `enforce_acl()` — are there paths that bypass it?
- `enforce_channel_membership()` — what happens when `X-Org-Id` header is missing?
- `ACL_MODE=permissive` — does it truly skip all checks? Is it documented as dev-only?
- Can a request without TLS client cert access protected endpoints?

### 4. JWT authentication (`MEDIUM`)

**Files:** `src/api/mod.rs`

**Risk:** JWT secret compromise allows full API access.

**What to check:**
- Is `JWT_SECRET` enforced in production mode?
- Is the default secret (`change-me-in-production`) actually rejected?
- Token expiration handling
- Token validation before use in handlers

### 5. P2P message handling (`MEDIUM`)

**Files:** `src/network/mod.rs`

**Risk:** A malicious peer could send crafted messages to crash nodes or corrupt state.

**What to check:**
- Message deserialization — what happens with oversized messages?
- `P2P_RESPONSE_BUFFER_BYTES` — can a peer exhaust memory with large responses?
- Raft message handling — can a non-voter inject proposals?
- Gossip signature verification — is it enforced on all message types?
- `PrivateDataPush` — does it validate sender org against collection membership?

### 6. Input validation (`MEDIUM`)

**Files:** `src/api/middleware.rs` (InputValidationMiddleware), all handlers

**Risk:** Payload too large, wrong content type, injection.

**What to check:**
- Max payload enforcement — is it applied before deserialization?
- JSON deserialization errors — do they leak internal state?
- Path parameters — are they sanitized before use in storage keys?
- `X-Channel-Id`, `X-Org-Id` headers — can they contain injection payloads?

### 7. Private data dissemination (`LOW`)

**Files:** `src/api/handlers/private_data.rs`, `src/network/mod.rs`

**Risk:** Unauthorized access to private data, or data leakage to non-member orgs.

**What to check:**
- `put_private_data` handler — does it validate caller org membership?
- `PrivateDataPush` network handler — does it validate sender org?
- Can a peer request private data it shouldn't have access to?
- TTL purge — does it actually delete data?

## Dependencies with known attack surface

| Crate | Version | Purpose | Risk |
|---|---|---|---|
| `wasmtime` | 21 | Wasm execution | Sandbox escape (CVE tracking) |
| `rustls` | 0.23 | TLS | Protocol downgrade, cert validation |
| `ed25519-dalek` | 2.1 | Signatures | Implementation correctness |
| `pqcrypto-mldsa` | 0.1.2 | PQC signatures | New algorithm, less audited |
| `rocksdb` | 0.22 | Storage | Data corruption, C FFI safety |
| `actix-web` | 4.5 | HTTP server | Request smuggling, DoS |

Run `cargo audit` for current CVE status.

## Self-assessment results

| Category | Score | Notes |
|---|---|---|
| Key zeroization | Done | Ed25519 `ZeroizeOnDrop`, ML-DSA-65 custom Drop |
| Power-up self-tests | Done | KAT for Ed25519, ML-DSA-65, SHA-256 at startup |
| Input validation | Done | Content-Type, max payload middleware |
| Audit trail | Done | Every API request logged with trace ID |
| Rate limiting | Done | Token bucket per IP |
| TLS | Done | mTLS for P2P, optional for API |
| Secrets management | Partial | JWT_SECRET enforced in prod, no external KMS |

## Test coverage

- 992 unit tests + integration tests
- 42+ E2E assertions on Docker network
- 5 property-based tests (proptest) for crypto invariants
- No fuzzing (recommended as audit follow-up)

## Recommended audit duration

Based on scope and codebase size (~50K lines of Rust):

| Depth | Duration | Cost estimate |
|---|---|---|
| Focused (crypto + ACL + sandbox) | 2-3 weeks | $15K-25K |
| Standard (all in-scope areas) | 4-6 weeks | $30K-50K |
| Comprehensive (+ load test + pentest) | 8-10 weeks | $50K-80K |
