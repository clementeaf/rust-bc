# Findings Register

**Module:** pqc_crypto_module v0.1.0
**Date:** 2026-04-28

> **Disclaimer:** This package does not imply FIPS 140-3 validation or CMVP certification.

---

## 1. Purpose

This register aggregates all findings from the mock FIPS 140-3 lab audit (`MOCK_AUDIT_REPORT.md`) into a single tracking document. It serves as the master list for remediation planning before formal lab engagement.

---

## 2. Findings Table

| ID | Severity | Area | Description | Owner | Status | Target Date | Blocking for Lab Intake? |
|---|---|---|---|---|---|---|---|
| F-001 | HIGH | Approved Services | ML-KEM-768 (FIPS 203) is a structural placeholder. `encapsulate()` and `decapsulate()` exist but no production consumer uses ML-KEM output. A lab will reject the module if an advertised approved service is non-functional. Resolution: either complete the ML-KEM implementation end-to-end or remove it from the approved algorithm list. | TBD | Open | Pre-lab | **Yes** |
| F-002 | HIGH | Self-Tests / Algorithm Validation | No NIST ACVP (Automated Cryptographic Validation Protocol) test vectors integrated. KAT self-tests use internally generated vectors, not official ACVP vectors. Algorithm validation is mandatory for CMVP certification. | TBD | Open | Pre-lab | **Yes** |
| F-003 | MEDIUM | Entropy / RNG | Module sources randomness from `OsRng` (via `getrandom` crate) without an explicit SP 800-90A compliant DRBG (HMAC-DRBG, CTR-DRBG) within the module boundary. May be acceptable at Security Level 1 with documentation, but a lab may require an in-module DRBG. | TBD | Open | Lab phase | No |
| F-004 | LOW | Key Management / Zeroization | Private key types implement `ZeroizeOnDrop` but memory pages are not pinned via `mlock`. Key material could theoretically be written to swap, surviving zeroization. Not strictly required at Security Level 1 but noted as best practice. | TBD | Open | Lab phase | No |
| F-005 | INFO | Zeroization | The `zeroize` crate uses `write_volatile` + compiler fence to resist optimization. LLVM provides no formal guarantee that volatile writes to stack temporaries are preserved in all optimization levels. Theoretical risk only; no known LLVM version optimizes this out. | TBD | Accepted | -- | No |
| F-006 | MEDIUM | Non-Approved Services | `ensure_not_approved()` guard exists in `legacy.rs` but is not enforced at every non-approved algorithm entry point. The FSM does not explicitly block non-approved code paths when in Approved state. Gating relies partly on caller discipline (env var) rather than module-enforced state checks. | TBD | Open | Pre-lab | **Yes** (medium priority) |
| F-007 | LOW | Finite State Machine | FSM (Uninitialized -> SelfTesting -> Approved -> Error) uses `AtomicU8` with `SeqCst` ordering, which is correct. However, no formal state diagram exists in documentation, and no exhaustive test covers all 16 possible (state, transition) pairs including invalid ones. | TBD | Open | Lab phase | No |
| F-008 | MEDIUM | Module Specification / Boundary | Cryptographic boundary is enforced by `tests/crypto_boundary.rs` (189/189 clean). However, FIPS 140-3 requires a formal Module Specification document listing every public API, data input/output, control input, and status output. The test proves compliance but is not the required document. | TBD | Open | Pre-lab | **Yes** (medium priority) |
| F-009 | LOW | Error Handling | Error state is fail-closed (correct). However, there is no documented recovery path. It is not explicit whether Error is a terminal state or if the process can be re-initialized. Operator guidance is missing. | TBD | Open | Lab phase | No |
| F-010 | HIGH | Guidance Documents | No Security Policy document exists. FIPS 140-3 requires a Crypto Officer Guide and a User Guide (collectively the Security Policy). This is a mandatory deliverable for CMVP submission and cannot be waived. | TBD | Open | Pre-lab | **Yes** |
| F-011 | MEDIUM | Lifecycle Assurance / Build | Clean-room reproducible build process is defined (`scripts/clean_room_build.sh`, CI `pre-lab-audit.yml`) and `Cargo.lock` is pinned, but the process has not been independently executed with hash comparison evidence from two separate builds. | TBD | Open | Pre-lab | **Yes** (medium priority) |

---

## 3. Summary by Severity

| Severity | Count | IDs |
|---|---|---|
| CRITICAL | 0 | -- |
| HIGH | 3 | F-001, F-002, F-010 |
| MEDIUM | 4 | F-003, F-006, F-008, F-011 |
| LOW | 3 | F-004, F-007, F-009 |
| INFO | 1 | F-005 |

**Total findings:** 11

---

## 4. Lab Intake Blockers

The following findings must be resolved before engaging a CMVP-accredited laboratory:

### Hard blockers (HIGH)

| ID | Description | Recommended resolution |
|---|---|---|
| F-001 | ML-KEM placeholder | Remove ML-KEM from approved list (fastest) or complete implementation |
| F-002 | No ACVP vectors | Integrate official vectors; build `tools/acvp_dry_run/` |
| F-010 | No Security Policy | Author Security Policy per SP 800-140 series |

### Soft blockers (MEDIUM -- strongly recommended before intake)

| ID | Description | Recommended resolution |
|---|---|---|
| F-006 | Non-approved not FSM-blocked | Add FSM state check at all non-approved entry points |
| F-008 | No formal Module Specification | Author Module Specification document |
| F-011 | Reproducible build unverified | Execute clean-room build, archive hash comparison |

### Addressable during lab phase

| ID | Description |
|---|---|
| F-003 | Add SP 800-90A DRBG or document OS RNG justification |
| F-004 | Add `mlock` for key memory pages |
| F-007 | Add exhaustive FSM transition tests and state diagram |
| F-009 | Document Error as terminal state in Security Policy |
| F-005 | Accepted risk -- document `zeroize` approach |

---

## 5. Remediation Priority

Recommended order of work:

1. **F-001** -- Remove or complete ML-KEM (unblocks F-002 for ML-KEM vectors)
2. **F-010** -- Author Security Policy (addresses F-007, F-009 documentation gaps simultaneously)
3. **F-002** -- Integrate ACVP vectors (requires `tools/acvp_dry_run/` tooling)
4. **F-006** -- Add FSM enforcement at non-approved entry points
5. **F-008** -- Author formal Module Specification (can be section of Security Policy)
6. **F-011** -- Execute and verify clean-room build
7. **F-003** -- Add HMAC-DRBG or document OS RNG
8. **F-004** -- Add `mlock`
9. **F-007** -- Exhaustive FSM tests
10. **F-009** -- Document recovery procedures
11. **F-005** -- Assembly verification CI step (nice-to-have)

---

## 6. Change Log

| Date | Change | Author |
|---|---|---|
| 2026-04-28 | Initial findings register created from mock audit | Pre-lab self-assessment |

---

*End of findings register.*
