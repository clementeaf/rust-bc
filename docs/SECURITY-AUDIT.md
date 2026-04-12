# Security Audit Report — rust-bc

**Date:** 2026-04-12
**Commit:** 48980be
**Scope:** Full codebase review

---

## Summary Table

| Area                        | Rating       | Highest Finding                                      |
|-----------------------------|--------------|------------------------------------------------------|
| Authentication / Authorization | **WEAK**   | CRITICAL: legacy routes have no auth                 |
| Cryptographic security      | **STRONG**   | MEDIUM: block hash omits tx content                  |
| Input validation            | **ADEQUATE** | MEDIUM: debug eprintln in production                 |
| Rate limiting / DoS         | **WEAK**     | HIGH: token-bucket RateLimiter is dead code          |
| TLS / mTLS                  | **STRONG**   | MEDIUM: plaintext default                            |
| Chaincode / Wasm sandboxing | **STRONG**   | MEDIUM: no Wasm module signature                     |
| Consensus attack vectors    | **ADEQUATE** | MEDIUM: nothing-at-stake between checkpoints         |
| P2P network security        | **ADEQUATE** | HIGH: network_security.rs is dead code               |
| Private data protection     | **ADEQUATE** | MEDIUM: header-based auth in non-mTLS mode           |
| Secret management           | **ADEQUATE** | MEDIUM: JWT secret unused                            |
| SQL / command injection     | **STRONG**   | None                                                 |
| Hardcoded secrets           | **ADEQUATE** | None                                                 |
| Double-spend protection     | **WEAK**     | HIGH: weak heuristic, not persisted                  |
| Replay attack prevention    | **WEAK**     | HIGH: in-memory only, no drift check                 |

---

## Strengths

### Cryptography (STRONG)
- Ed25519 via `ed25519_dalek` with `ZeroizeOnDrop` on `SigningKey`.
- ML-DSA-65 (FIPS 204) post-quantum signatures with manual zeroize on drop.
- FIPS 140-3 Known-Answer Tests at startup for Ed25519, ML-DSA-65, and SHA-256. Node refuses to start on failure.
- `OsRng` for all key generation.
- Transaction signatures cover hash of `(id, from, to, amount, fee, data, timestamp)`.

### TLS / mTLS (STRONG)
- `rustls` throughout (no OpenSSL).
- mTLS via `WebPkiClientVerifier` with optional `PinningClientCertVerifier`.
- OCSP stapling via `TLS_OCSP_STAPLE_PATH`.
- CRL store wired into `AppState`.

### Chaincode / Wasm Sandboxing (STRONG)
- Wasmtime v36 (upgraded from v21, eliminating 15 CVEs).
- `consume_fuel(true)` and `StoreLimitsBuilder` per invocation.
- `MAX_CHAINCODE_DEPTH = 8` prevents unbounded recursion.
- Fresh `Store` per call — fuel/memory limits are per-invocation.

### Injection (STRONG)
- No SQL databases. RocksDB and in-memory stores use typed Rust APIs.
- No `std::process::Command` calls found.

---

## Critical Findings

### C1 — Legacy API routes bypass ACL entirely

**File:** `src/api_legacy.rs`
**Severity:** CRITICAL

The following endpoints have zero authentication or ACL enforcement:

| Endpoint | Risk |
|----------|------|
| `POST /api/v1/mine` | Any caller mines blocks, redirects reward to arbitrary address |
| `POST /api/v1/contracts` | Unauthenticated contract deployment |
| `POST /api/v1/contracts/{addr}/execute` | Unauthenticated contract execution |
| `POST /api/v1/peers/{addr}/connect` | Connect node to arbitrary P2P address |
| `POST /api/v1/sync` | Trigger full chain sync |

The `mine_block` handler accepts a caller-controlled `miner_address` without verifying ownership:
```rust
// src/api_legacy.rs:265
let address_to_use = validator_addr.as_ref().unwrap_or(&miner_address_clone);
```

**Fix:** Add `enforce_acl(...)` to all legacy routes. Verify `miner_address` belongs to the authenticated caller.

### C2 — Header spoofing bypasses all ACL in non-mTLS mode

**File:** `src/api/errors.rs` (lines 213-257)
**Severity:** CRITICAL (when TLS disabled, which is the default)

`enforce_acl` falls back to reading `X-Msp-Role` and `X-Org-Id` from request headers when no `TlsIdentity` is set. Without TLS (the default startup mode), any client can self-assert `X-Msp-Role: admin` to pass all ACL checks, including access to other orgs' private data.

**Fix:** Reject ACL-protected requests entirely when TLS is not configured, or require a separate auth mechanism (JWT, API key).

---

## High Findings

### H1 — RateLimiter is dead code

**File:** `src/api/rate_limit.rs`
**Severity:** HIGH

The entire `RateLimiter` struct (`allow_request`, `get_remaining_tokens`, `reset`) is annotated `#[allow(dead_code)]`. Never instantiated in `main.rs` or any middleware chain.

The active middleware in `src/middleware.rs` has separate issues:
- Per-worker state (not shared via `Arc` across Actix workers), multiplying effective limits.
- Exempts `/api/v1/billing/create-key` — the one unauthenticated POST creating persistent state.
- Uses `unwrap_or_else(|e| e.into_inner())` on poisoned mutex.

**Fix:** Wire `RateLimiter` into a real middleware. Share state with `Arc` across workers.

### H2 — network_security.rs is dead code

**File:** `src/network_security.rs`
**Severity:** HIGH

Line 1: `#![allow(dead_code)]`. `NetworkSecurityManager`, `PeerScore`, and `PeerRateLimit` are defined but not integrated into the P2P message handler. Reputation, blacklisting, and per-peer rate limiting do not execute at runtime.

**Fix:** Integrate into the P2P receive loop, or remove the module to avoid false confidence.

### H3 — Weak double-spend detection

**File:** `src/blockchain.rs` (line 823)
**Severity:** HIGH

`is_double_spend` matches on `(from, amount, timestamp)`:
```rust
existing_tx.from == tx.from
    && existing_tx.id != tx.id
    && existing_tx.amount == tx.amount
    && existing_tx.timestamp == tx.timestamp
```

Two transactions with different amounts or timestamps bypass this entirely. Balance check in `validate_transaction` provides a second layer, but only at mempool insertion — not when blocks arrive from peers.

**Fix:** Replace with nonce-based or UTXO-based detection. Match on `tx.id` uniqueness across chain + persisted seen-set.

### H4 — Replay prevention is in-memory only

**File:** `src/transaction_validation.rs`
**Severity:** HIGH

`seen_transaction_ids` and `sender_states` are freshly constructed at startup. After a restart, confirmed transactions can be re-submitted. No server-side timestamp window validation — clients can submit transactions with arbitrary future timestamps.

**Fix:** Persist `seen_transaction_ids` to RocksDB. Add timestamp drift check (reject > 30s future, > 10min past).

### H5 — Wallet creation requires no authentication

**File:** `src/api_legacy.rs` (lines 97-111)
**Severity:** HIGH

`POST /api/v1/wallets/create` checks an optional `X-API-Key` header. When absent, the handler proceeds without any identity check. The API key has no cryptographic binding.

---

## Medium Findings

### M1 — Block hash omits transaction content

**File:** `src/blockchain.rs` (line 110)

`Block::calculate_hash()` hashes `transactions.len()` but not transaction data directly. Only `merkle_root` represents content. If `calculate_merkle_root` has a bug, block hash provides no second layer of protection.

### M2 — Debug logging in production

**File:** `src/api_legacy.rs` (lines 717-757)

`eprintln!("[DEPLOY]...")` emits request body content (owner, contract_type, names) unconditionally. Not gated by a debug flag.

**Fix:** Replace with `log::debug!`.

### M3 — Content-Length bypass

**File:** `src/middleware.rs`

`InputValidationMiddleware` checks the `Content-Length` header but chunked requests (no header) bypass the check.

**Fix:** Add `web::JsonConfig::default().limit(...)` as defense in depth.

### M4 — JWT secret exists but is never used

**File:** `src/api/mod.rs`

`ApiConfig::jwt_secret` is populated and validated at startup, but no JWT parsing or `Authorization: Bearer` validation exists anywhere. False confidence in documentation.

**Fix:** Implement JWT middleware or remove the field.

### M5 — No Wasm module signature verification

**File:** `src/api/handlers/chaincode.rs`

`POST /api/v1/chaincode/install` stores raw Wasm bytes without checksum or signature verification. Compromised admin can install any payload.

**Fix:** Add SHA-256 hash verification at install time. Verify at instantiation.

### M6 — Nothing-at-stake window between checkpoints

**File:** `src/consensus/mod.rs`

Between checkpoints (up to 2000 blocks), validators can sign competing forks. Slashing catches identical-index double-signs but not multi-chain strategies with different indices.

### M7 — Checkpoint integrity relies on filesystem only

**File:** `src/checkpoint.rs`

Checkpoints stored as JSON files. No HMAC or signature over content. Local attacker with filesystem write access can modify checkpoint hashes.

**Fix:** Add HMAC-SHA256 over checkpoint content.

### M8 — Private data auth falls back to headers

**File:** `src/api/handlers/private_data.rs`

Same as C2 — when TLS is off, `X-Org-Id` header controls private data access.

### M9 — TLS headers from untrusted sources

**File:** `src/middleware.rs` (line 142)

`extract_tls_identity` reads `X-TLS-Client-CN` and `X-TLS-Client-O` from any caller. Without a trusted proxy, any client can set these.

---

## Priority Remediation

| Priority | Fix | Files | Status |
|----------|-----|-------|--------|
| **P0** | Add `enforce_acl` to all legacy routes + verify `miner_address` ownership | `api_legacy.rs` | DONE |
| **P0** | Reject header-based identity in strict mode (X-Org-Id, X-Msp-Role only in permissive) | `api/errors.rs` | DONE |
| **P1** | Rate limiter: remove billing exempt, fix logging | `middleware.rs` | DONE |
| **P1** | Replace `is_double_spend` with `tx.id` uniqueness | `blockchain.rs` | DONE |
| **P1** | Add timestamp drift validation | `transaction_validation.rs` | DONE |
| **P1** | Integrate `NetworkSecurityManager` into P2P loop | `network_security.rs`, `network/mod.rs` | DONE |
| **P2** | Document `jwt_secret` as reserved, not active auth | `api/mod.rs` | DONE |
| **P2** | Add Wasm module hash verification at install | `api/handlers/chaincode.rs` | DONE |
| **P2** | Replace `eprintln!` debug logging with `log::debug!` | `api_legacy.rs` | DONE |
| **P2** | Add HMAC over checkpoint files | `checkpoint.rs` | DONE |
