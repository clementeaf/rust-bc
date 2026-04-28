# FIPS 140-3 Implementation Guidance Checklist

**Module:** pqc_crypto_module v0.1.0
**Security Level Target:** 1 (Software)
**Date:** 2026-04-28

> **Disclaimer:** This package does not imply FIPS 140-3 validation or CMVP certification.

---

## Assessment Key

| Status | Meaning |
|---|---|
| PASS | Requirement met with evidence |
| PARTIAL | Partially implemented, gaps identified |
| FAIL | Not implemented or insufficient |
| N/A | Not applicable to this module type/level |

---

## Checklist

| # | IG Area | Status | Evidence | Gap | Owner | Next Action |
|---|---|---|---|---|---|---|
| 1 | **Module Specification** | PARTIAL | Module name, version, algorithm list defined in `Cargo.toml` and `lib.rs`. Boundary enforced by `tests/crypto_boundary.rs`. | No formal Module Specification document (Security Policy). Missing: ports/interfaces table, service descriptions, mode of operation narrative. See F-008, F-010. | Module owner | Author formal Module Specification document per FIPS 140-3 Section 7.2. |
| 2 | **Cryptographic Module Boundary** | PASS | 11 source files in `crates/pqc_crypto_module/src/`. `tests/crypto_boundary.rs` enforces 100% compliance (189/189 files). All crypto operations route through the module's public API. | None for the code. Boundary definition needs to appear in the formal spec document (see #1). | Module owner | Include boundary diagram in Module Specification. |
| 3 | **Approved Cryptographic Services** | PARTIAL | ML-DSA-65 (FIPS 204): fully functional, sign/verify integrated end-to-end. SHA3-256 (FIPS 202): functional, used in self-tests and hashing. ML-KEM-768 (FIPS 203): placeholder only. | ML-KEM-768 is listed as approved but not functional. See F-001. | Module owner | Either complete ML-KEM or remove from approved list. |
| 4 | **Non-Approved Cryptographic Services** | PARTIAL | Ed25519, SHA-256, HMAC-SHA256 identified as non-approved. Runtime-gated via `SIGNING_ALGORITHM` env var. Feature-flag exclusion documented. | No FSM-enforced blocking of non-approved algorithms in Approved mode. See F-006. | Module owner | Add explicit FSM state check at entry points of all non-approved code paths. |
| 5 | **Roles and Authentication** | PARTIAL | Implicit single role (Crypto Officer = process owner). No separate User role defined. | FIPS 140-3 requires at least Crypto Officer and User roles. No role-based authentication within the module (acceptable at Level 1 but must be documented). | Module owner | Document roles in Security Policy. CO = process that initializes module. User = any caller after init. |
| 6 | **Finite State Machine (FSM)** | PASS | 4 states: Uninitialized, SelfTesting, Approved, Error. `AtomicU8` with `SeqCst` ordering. Transitions well-defined. Error is terminal. | No formal state diagram in documentation. No exhaustive invalid-transition tests. See F-007. | Module owner | Add state diagram to Security Policy. Add exhaustive transition tests. |
| 7 | **Self-Tests** | PARTIAL | Power-on self-tests: SHA3 KAT, ML-DSA sign/verify/corrupt KAT, ML-KEM encaps/decaps KAT, continuous RNG test. Self-tests must pass before FSM enters Approved. | No ACVP vectors. Self-test vectors are internal. See F-002. No conditional self-tests on algorithm parameter changes. | Module owner | Integrate ACVP vectors. Add conditional self-test hooks. |
| 8 | **Error Handling** | PASS | Self-test failure transitions FSM to Error. All crypto operations check FSM state and return `CryptoError` on failure. Fail-closed behavior confirmed. | Recovery path undocumented. See F-009. | Module owner | Document Error as terminal state in Security Policy. |
| 9 | **Key Management** | PARTIAL | `MldsaPrivateKey` (4032 bytes) with `ZeroizeOnDrop`. `MlKemPrivateKey` with `ZeroizeOnDrop`. `MlKemSharedSecret` with `ZeroizeOnDrop`. Keys generated via OsRng. | No key import/export format specification. No key usage period enforcement. No mlock. See F-004. | Module owner | Document key lifecycle. Add mlock. Define key transport if needed. |
| 10 | **Zeroization** | PARTIAL | `ZeroizeOnDrop` on all private key types and shared secrets. `zeroize` crate uses `write_volatile` + fence. | No mlock (key pages could swap). Compiler optimization concern is theoretical. See F-004, F-005. | Module owner | Add mlock. Add assembly verification CI step. |
| 11 | **Entropy / RNG** | PARTIAL | `OsRng` via `getrandom` crate. Continuous RNG test (generate, check non-zero/non-repeating). `CryptoError::RngFailure` on failure. No fallback RNG. | No SP 800-90A DRBG within module boundary. No SP 800-90B entropy source documentation. See F-003. | Module owner | Add HMAC-DRBG or document OS RNG justification per SP 800-90B. |
| 12 | **Operational Environment** | PARTIAL | Targets general-purpose OS (Linux, macOS). Rust nightly toolchain. Single-process, multi-threaded. | No documented minimum OS requirements. No attestation of OS integrity (acceptable at Level 1). Need to specify supported OS versions. | Module owner | Document supported OS list and Rust toolchain requirements. |
| 13 | **Physical Security** | N/A | Software-only module at Security Level 1. No physical enclosure. | N/A | N/A | N/A |
| 14 | **Non-Invasive Attack Mitigation** | N/A | Not required at Security Level 1. | N/A | N/A | N/A |
| 15 | **Lifecycle Assurance** | PARTIAL | Version control (git). CI/CD with `cargo fmt`, `cargo clippy`, `cargo test`. `Cargo.lock` committed. | Reproducible build not yet proven. See F-011. No formal CM plan document. | Module owner | Execute clean-room build. Document CM procedures. |
| 16 | **Design Assurance** | PARTIAL | Source code available. Test suite with KATs. Boundary test. Architecture documented in `CLAUDE.md`. | No formal design document per FIPS 140-3 requirements. No formal code review records (lab expects documented evidence). | Module owner | Produce design document. Archive code review records. |
| 17 | **Guidance Documents** | FAIL | No Security Policy. No Crypto Officer Guide. No User Guide. | All three are mandatory deliverables for CMVP submission. See F-010. | Module owner | Author Security Policy (combined CO + User guide) per SP 800-140 series. |

---

## Summary

| Status | Count |
|---|---|
| PASS | 3 |
| PARTIAL | 12 |
| FAIL | 1 |
| N/A | 2 |

**Key blockers for lab intake:**
1. Guidance Documents (FAIL) — Security Policy is mandatory
2. Approved Services — ML-KEM placeholder must be resolved
3. Self-Tests — ACVP vectors required
4. RNG — SP 800-90A/B compliance gap

---

*End of FIPS 140-3 IG checklist.*
