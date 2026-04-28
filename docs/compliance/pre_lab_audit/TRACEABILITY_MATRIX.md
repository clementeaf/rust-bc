# Requirements Traceability Matrix

**Module:** pqc_crypto_module v0.1.0
**Date:** 2026-04-28

> **Disclaimer:** This package does not imply FIPS 140-3 validation or CMVP certification.

---

## 1. Purpose

This matrix maps each security requirement to its implementation file, test file, documentation file, and current status. It provides traceability from requirement through code to evidence.

---

## 2. Traceability Table

| # | Requirement | Implementation File(s) | Test File(s) | Documentation | Status | Notes |
|---|---|---|---|---|---|---|
| R-01 | **Approved mode only uses FIPS-approved algorithms** | `crates/pqc_crypto_module/src/lib.rs`, `src/state.rs`, `approved_mode.rs` | `crates/pqc_crypto_module/src/self_test.rs`, `fips_readiness.rs`, unit tests in `state.rs` | `FIPS_140_3_IG_CHECKLIST.md` #3, #4 | PARTIAL | Approved mode exists and self-tests gate entry. `require_approved()` guard present. Non-approved algorithms not FSM-blocked at every call site (F-006). |
| R-02 | **Self-tests must pass before any crypto operation** | `crates/pqc_crypto_module/src/self_tests.rs`, `src/approved_mode.rs` | `self_tests.rs` (KAT tests), `fips_readiness.rs`, `tools/acvp_dry_run/` (15 ACVP-inspired vectors) | `MOCK_AUDIT_REPORT.md` F-002 (CLOSED), `FIPS_140_3_IG_CHECKLIST.md` #7 | PASS | Self-tests gate FSM. ACVP dry-run harness validates SHA3-256, ML-DSA-65, ML-KEM-768 with 15 vectors. Official ACVP server submission is external. |
| R-03 | **Error state is fail-closed** | `crates/pqc_crypto_module/src/state.rs`, `src/errors.rs`, `approved_mode.rs` | `state.rs` (error state tests), `fips_readiness.rs` (`error_state_rejects_all`) | `MOCK_AUDIT_REPORT.md` F-009 | PASS | `ModuleState::Error` rejects all operations. FSM transitions to Error on self-test failure. Recovery path undocumented but behavior is correct. |
| R-04 | **ML-DSA-65 sign and verify** | `crates/pqc_crypto_module/src/mldsa.rs` | `self_test.rs` (sign/verify/corrupt KAT), `no_fallback.rs` | `ACVP_DRY_RUN_PLAN.md` Section 3 | PASS | Fully functional via `sign_message()` and `verify_signature()`. Integrated end-to-end with `src/identity/` signing provider. |
| R-05 | **ML-KEM-768 encapsulate and decapsulate** | `crates/pqc_crypto_module/src/mlkem.rs` | `mlkem.rs` (8 unit tests), `self_tests.rs` (KAT with roundtrip + invalid ct), `api_boundary.rs`, `tools/acvp_dry_run/` (5 encapsThenDecaps vectors) | `MOCK_AUDIT_REPORT.md` F-001 (CLOSED) | PASS | Fully functional via `pqcrypto-mlkem` crate. `encapsulate()` and `decapsulate()` produce matching shared secrets. pk=1184B, sk=2400B, ct=1088B, ss=32B. Invalid ciphertext handling verified. |
| R-06 | **SHA3-256 hashing** | `crates/pqc_crypto_module/src/hashing.rs` | `self_test.rs` (SHA3 KAT), `no_fallback.rs` | `ACVP_DRY_RUN_PLAN.md` Section 5 | PASS | Functional via `sha3` crate and `sha3_256()`. Used in self-tests and available to callers. |
| R-07 | **Zeroization of private keys and shared secrets** | `crates/pqc_crypto_module/src/types.rs` (key types with `ZeroizeOnDrop`) | `key_zeroization.rs` | `MOCK_AUDIT_REPORT.md` F-004, F-005, `FIPS_140_3_IG_CHECKLIST.md` #10 | PARTIAL | `ZeroizeOnDrop` derive on `MldsaPrivateKey` (4032B), `MlKemPrivateKey`, `MlKemSharedSecret`. No `mlock` to prevent swap-out (F-004). Compiler optimization is theoretical risk only (F-005). |
| R-08 | **Legacy algorithms blocked in approved mode** | `crates/pqc_crypto_module/src/legacy.rs` (`ensure_not_approved()` guard), `src/identity/` | `approved_vs_legacy.rs`, `crypto_boundary.rs` | `MOCK_AUDIT_REPORT.md` F-006, `FIPS_140_3_IG_CHECKLIST.md` #4 | PARTIAL | Runtime-gated via `SIGNING_ALGORITHM` env var and feature flags. `ensure_not_approved()` guard exists but not enforced at every non-approved entry point. All `src/` imports route through `pqc_crypto_module::legacy::*`. |
| R-09 | **No raw crypto operations outside module boundary** | All 189 source files outside `crates/pqc_crypto_module/src/` | `tests/crypto_boundary.rs` (189/189 clean, 5 tests) | `MOCK_AUDIT_REPORT.md` F-008, `FIPS_140_3_IG_CHECKLIST.md` #2 | PASS | 100% boundary compliance enforced by automated test. Zero violations. No crypto operations leak outside the module. |
| R-10 | **Reproducible build** | `scripts/clean_room_build.sh`, `Cargo.lock` (pinned) | CI `pre-lab-audit.yml`, manual execution (build comparison) | `CLEAN_ROOM_BUILD.md` | PARTIAL | Process defined, `Cargo.lock` pinned, CI configured. Clean-room Docker build not yet independently verified with hash comparison. F-011. |
| R-11 | **RNG failure results in operation failure, no fallback** | `crates/pqc_crypto_module/src/rng.rs` | `self_test.rs` (continuous RNG test), `rng.rs` unit tests | `ENTROPY_RNG_EVIDENCE.md` Section 4 | PASS | `CryptoError::RngFailure` returned on failure. No fallback RNG. No degraded mode. Explicit error propagation. |
| R-12 | **Cryptographic boundary enforced (11 source files)** | `crates/pqc_crypto_module/src/` (11 files), crate isolation | `tests/crypto_boundary.rs` (5 tests) | `MOCK_AUDIT_REPORT.md` F-008, `FIPS_140_3_IG_CHECKLIST.md` #2 | PASS | Boundary enforced by crate isolation and automated test. 189/189 non-module files verified clean. Formal Module Specification document still needed for lab submission (F-008). |

---

## 3. Status Summary

| Status | Count | Requirements |
|---|---|---|
| PASS | 8 | R-02, R-03, R-04, R-05, R-06, R-09, R-11, R-12 |
| PARTIAL | 4 | R-01, R-07, R-08, R-10 |
| FAIL | 0 | -- |

---

## 4. Blocking Items for Lab Intake

| Requirement | Finding | Blocking? |
|---|---|---|
| R-05 (ML-KEM) | F-001 | **CLOSED** -- `pqcrypto-mlkem` implementation with roundtrip verification |
| R-02 (Self-tests / ACVP) | F-002 | **CLOSED / PARTIAL-OFFICIAL** -- ACVP dry-run harness functional |
| R-10 (Reproducible build) | F-011 | Yes -- lifecycle assurance requires verified hash comparison |
| R-01 (Approved mode) | F-006 | Medium -- FSM enforcement gap at some call sites |
| R-08 (Legacy blocked) | F-006 | Medium -- same root cause as R-01 |

---

## 5. Cross-References

- Finding IDs (F-xxx) reference `MOCK_AUDIT_REPORT.md`
- IG checklist items reference `FIPS_140_3_IG_CHECKLIST.md`
- All findings aggregated in `FINDINGS_REGISTER.md`

---

*End of traceability matrix.*
