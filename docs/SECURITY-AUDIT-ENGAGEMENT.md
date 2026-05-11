# Security Audit Engagement Package — Cerulean Ledger

**Prepared for:** External security auditor
**Date:** 2026-05-11
**Version:** 1.0

---

## 1. Project Overview

Cerulean Ledger is a post-quantum DLT (Distributed Ledger Technology) node written in Rust. It targets institutional use cases: verifiable credentials, document certification, governance voting, and tokenized assets.

- **Language:** Rust (nightly)
- **Lines of code:** ~35,000 (src/)
- **Test count:** ~1,700 unit + integration tests
- **License:** MIT

## 2. Audit Scope

### In scope

| Component | Path | Priority | Why |
|-----------|------|----------|-----|
| Crypto module (FIPS) | `crates/pqc_crypto_module/` | CRITICAL | ML-DSA-65, ML-KEM-768, SHA3, key lifecycle, FSM |
| Identity & signing | `src/identity/` | HIGH | DID, key rotation, Ed25519 + ML-DSA-65 verification |
| Consensus (BFT) | `src/consensus/bft/` | HIGH | Quorum validation, vote collection, round management |
| Storage layer | `src/storage/` | HIGH | RocksDB adapters, BlockStore trait, migrations |
| API auth & ACL | `src/api/errors.rs`, `src/middleware.rs`, `src/acl/` | HIGH | enforce_acl, TLS identity extraction, rate limiting |
| Chaincode execution | `src/chaincode/executor.rs` | HIGH | Wasm sandbox, host functions, fuel/memory limits |
| Transaction validation | `src/transaction/` | MEDIUM | MVCC, parallel execution, conflict detection |
| Tokenomics | `src/tokenomics/` | MEDIUM | Supply cap, fee burn, halving, storage deposits |
| Bridge | `src/bridge/` | MEDIUM | Cross-chain escrow, Merkle proofs, replay protection |
| Governance | `src/governance/` | MEDIUM | Proposal lifecycle, stake-weighted voting |

### Out of scope

- Frontend apps (`block-explorer-vite/`, `cerulean-voto/`) — UI only, no business logic
- Documentation (`docs/`) — except for verifying accuracy of security claims
- Tesseract prototype (`tesseract/`) — standalone research crate, not production

## 3. Architecture Summary

```
Client → [TLS/mTLS] → Actix-Web API → AppState
                                          ├── Blockchain (legacy, in-memory)
                                          ├── BlockStore (trait, Memory or RocksDB)
                                          ├── Consensus (BFT + DPoS)
                                          ├── Chaincode (Wasm via wasmtime)
                                          ├── Identity (DID + VC + PQC signing)
                                          └── P2P Network (gossip, peer discovery)
```

### Key data flows

1. **Transaction:** Client → ACL check → mempool → mine → consensus → store
2. **Chaincode:** Install (Wasm bytes) → sandbox validation → approve (multi-org) → commit
3. **Identity:** Create DID → store identity → issue credential → verify/revoke
4. **Consensus:** Propose block → BFT vote rounds (Prepare→PreCommit→Commit→Decide) → commit with QC

## 4. Cryptographic Inventory

| Algorithm | Standard | Usage | Implementation |
|-----------|----------|-------|----------------|
| ML-DSA-65 | FIPS 204 | Block/endorsement signing | `pqcrypto-mldsa` via `pqc_crypto_module` |
| ML-KEM-768 | FIPS 203 | Key encapsulation | `pqcrypto-mlkem` via `pqc_crypto_module` |
| SHA3-256 | FIPS 202 | Block hashing (configurable) | `sha3` crate |
| SHA-256 | FIPS 180-4 | Block hashing (default), commitments | `sha2` crate |
| Ed25519 | RFC 8032 | Legacy signing (classical) | `ed25519-dalek` |
| Argon2id | RFC 9106 | PIN hashing | `argon2` crate |
| X25519+ML-KEM-768 | — | TLS hybrid KEM (optional) | `rustls-post-quantum` |
| HMAC-SHA256 | RFC 2104 | Checkpoint integrity, oracle signatures | `hmac` + `sha2` |

### Crypto boundary

ALL production crypto routed through `pqc_crypto_module`. Direct imports of `sha2`, `ed25519_dalek`, `pqcrypto_mldsa` in `src/` are forbidden (enforced by `tests/crypto_boundary.rs`).

## 5. Known Risks & Accepted Limitations

| ID | Risk | Status | Notes |
|----|------|--------|-------|
| R-001 | ZKP module is commitment-based, not zero-knowledge | DOCUMENTED | Verifier sees claim value. Module named `zkp.rs` for API compat. |
| R-002 | Legacy storage (`Blockchain` struct) coexists with new `BlockStore` | DOCUMENTED | 17 production refs. No data corruption — systems operate independently. |
| R-003 | JWT middleware declared but not implemented | ACCEPTED | mTLS + ACL is the active auth mechanism. |
| R-004 | P2P not tested with >3 real nodes | ACCEPTED | BFT E2E tests simulate up to 10 nodes in-process. |
| R-005 | FIPS 140-3 not certified | IN PROGRESS | Documentation package ready, lab not yet engaged. |
| R-006 | No external audit performed yet | THIS ENGAGEMENT | First external audit. |

## 6. Previous Internal Security Work

- **Pentest suite:** 33 attack scenarios in `src/forensic_pentest.rs` (integrity, crypto, ACL, consensus, network, EVM, economic)
- **Security audit doc:** `docs/architecture/security/SECURITY-AUDIT.md` — 10/10 findings remediated
- **Chaos tests:** `tests/chaos_network.rs` — 11 multi-node adversarial scenarios
- **Crypto DOS:** `tests/crypto_dos_flood.rs` — 6 flood resistance tests
- **Equivocation:** `tests/byzantine_equivocation.rs` — 9 detection + penalty tests
- **Property tests:** `tests/property_invariants.rs` — 7 proptest invariants
- **Fuzzing:** `fuzz/` — 3 libfuzzer targets (block parser, signature parser, gossip message)

## 7. Test Coverage

```bash
# Run all unit tests (~1,700)
cargo test --lib

# Run specific integration suites
cargo test --test cross_subsystem       # 7 cross-module tests
cargo test --test bft_e2e               # 16 BFT adversarial tests
cargo test --test pqc_security_audit    # 24 PQC enforcement tests
cargo test --test chaos_network         # 11 multi-node chaos tests

# Coverage report
cargo llvm-cov --html
```

## 8. Environment Setup for Auditors

```bash
# Clone
git clone https://github.com/clementeaf/rust-bc.git
cd rust-bc

# Build (requires Rust nightly)
rustup default nightly
cargo build

# Run node (permissive mode for testing)
ACL_MODE=permissive cargo run --bin rust-bc

# Run full test suite
cargo test --lib

# Lint
cargo clippy -- -D warnings
```

### Docker (multi-node)

```bash
docker compose build
docker compose up -d
# 3 peers + orderers + Prometheus + Grafana
```

## 9. Key Files for Review

| File | What to look for |
|------|------------------|
| `src/api/errors.rs` | `enforce_acl()` — all mutation endpoints pass through this |
| `src/api/middleware.rs` | TLS identity extraction, audit middleware, input validation |
| `src/middleware.rs` | Rate limiting implementation |
| `src/chaincode/executor.rs` | Wasm host functions — can chaincode escape sandbox? |
| `src/chaincode/sandbox.rs` | Import whitelist — are all dangerous imports blocked? |
| `src/consensus/bft/round.rs` | BFT state machine — can a Byzantine node force invalid decisions? |
| `src/consensus/equivocation.rs` | Equivocation detection — can it be bypassed? |
| `src/storage/adapters.rs` | RocksDB operations — injection via key construction? |
| `src/identity/signing.rs` | Signature verification dispatch — algorithm confusion? |
| `src/identity/pqc_policy.rs` | PQC enforcement — can classical sigs bypass? |
| `src/bridge/` | Cross-chain escrow — double-spend via replay? |
| `crates/pqc_crypto_module/src/lib.rs` | FSM — can crypto be used before self-test? |

## 10. Communication

- **Primary contact:** [PROJECT OWNER]
- **Repository:** https://github.com/clementeaf/rust-bc
- **Branch for audit:** `main` (commit to be pinned at engagement start)
- **Findings format:** Severity (Critical/High/Medium/Low/Info) + PoC + recommendation
- **Expected timeline:** 2-4 weeks for initial report

---

*This document was prepared to facilitate the audit engagement. All claims about test coverage, crypto usage, and architecture are verifiable from the source code.*
