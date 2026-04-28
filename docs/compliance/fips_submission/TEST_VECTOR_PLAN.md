# Test Vector Plan — pqc_crypto_module v0.1.0

> **Disclaimer**: Prepared for FIPS 140-3 evaluation, not currently validated.

---

## 1. Overview

FIPS 140-3 validation requires that each approved algorithm pass testing against official NIST test vectors via the Cryptographic Algorithm Validation Program (CAVP). This document identifies the current state of test vector coverage and the actions required to achieve CAVP compliance.

## 2. Algorithm Test Vector Status

### 2.1 ML-DSA-65 (FIPS 204 — Digital Signatures)

| Item | Status |
|------|--------|
| Internal KATs | Implemented. `self_tests.rs` runs sign/verify KATs at module initialization. |
| Known Answer Test vectors | Internal vectors derived from library output; not sourced from NIST. |
| NIST ACVP vectors | NOT INTEGRATED. NIST ACVP server support for ML-DSA is being finalized. |
| CAVP certificate | NOT OBTAINED. Requires lab engagement. |

**Action Items:**
1. Monitor NIST ACVP server for ML-DSA-65 test vector availability.
2. Implement ACVP test harness to consume NIST JSON vector format.
3. Validate: KeyGen, SigGen, SigVer operations against official vectors.
4. Coordinate with selected lab for CAVP algorithm certificate submission.

### 2.2 ML-KEM-768 (FIPS 203 — Key Encapsulation)

| Item | Status |
|------|--------|
| Internal KATs | Implemented. Encapsulate/decapsulate round-trip test at initialization. |
| Known Answer Test vectors | Internal vectors; structural placeholder implementation (SHA3-based). |
| NIST ACVP vectors | NOT INTEGRATED. NIST ACVP server support for ML-KEM is being finalized. |
| CAVP certificate | NOT OBTAINED. Requires lab engagement. |

**Action Items:**
1. Replace placeholder ML-KEM-768 implementation with a FIPS 203-compliant library when available in the Rust ecosystem.
2. Monitor NIST ACVP server for ML-KEM-768 test vector availability.
3. Implement ACVP test harness for: KeyGen, Encapsulate, Decapsulate.
4. Coordinate with selected lab for CAVP algorithm certificate submission.

### 2.3 SHA3-256 (FIPS 202 — Hashing)

| Item | Status |
|------|--------|
| Internal KATs | Implemented. SHA3-256 KAT with known input/output in `self_tests.rs`. |
| NIST test vectors | NOT INTEGRATED. NIST provides published SHA-3 test vectors (CAVP). |
| CAVP certificate | NOT OBTAINED. SHA-3 CAVP testing is well-established. |

**Action Items:**
1. Download official NIST SHA-3 test vectors from the CAVP page.
2. Integrate short-message, long-message, and Monte Carlo test vectors.
3. Run validation suite and document results.
4. Submit for CAVP SHA-3 algorithm certificate via selected lab.

### 2.4 Random Number Generation (SP 800-90B Entropy Source)

| Item | Status |
|------|--------|
| RNG implementation | `OsRng` via `getrandom` crate (delegates to OS CSPRNG). |
| Continuous RNG test | Implemented in `rng.rs` (consecutive output comparison). |
| SP 800-90B compliance | NOT DOCUMENTED. OS entropy source compliance assumed but not formally verified. |
| Entropy source validation | NOT STARTED. |

**Action Items:**
1. Document the entropy source chain: `OsRng` -> `getrandom` -> OS kernel CSPRNG.
2. Determine SP 800-90B applicability for the target OS platforms.
3. Assess whether the OS-provided entropy source has existing SP 800-90B validation (e.g., Linux `/dev/urandom` via DRBG, macOS SecRandomCopyBytes).
4. Discuss entropy source compliance path with selected lab.

## 3. ACVP Test Harness Requirements

To obtain CAVP certificates, an Automated Cryptographic Validation Protocol (ACVP) test harness must be implemented:

| Requirement | Description |
|-------------|-------------|
| ACVP client | Software that communicates with the NIST ACVP server |
| JSON vector parsing | Parse NIST-provided JSON test vector files |
| Algorithm registration | Register supported algorithms and capabilities with ACVP server |
| Response generation | Execute crypto operations and format responses per ACVP spec |
| Lab coordination | Lab may provide their own ACVP proxy or tooling |

**Implementation plan:**
1. Evaluate existing open-source ACVP clients (e.g., `libacvp`, `acvp-client`).
2. Build a thin Rust wrapper that calls `pqc_crypto_module::api` functions.
3. Test against NIST ACVP demo server before lab engagement.

## 4. Timeline

| Milestone | Estimated Duration | Dependencies |
|-----------|--------------------|--------------|
| SHA3-256 NIST vectors integrated | 1-2 weeks | None (vectors available now) |
| ACVP harness prototype | 2-4 weeks | None |
| ML-DSA ACVP vectors available | TBD | NIST ACVP server timeline |
| ML-KEM implementation upgrade | TBD | Rust ecosystem crate availability |
| CAVP certificates obtained | 2-6 months | Lab engagement |
