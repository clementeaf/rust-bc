# Executive Summary — pqc_crypto_module v0.1.0

> **Disclaimer**: Prepared for FIPS 140-3 evaluation, not currently validated.

---

## What We Are

**pqc_crypto_module** is a standalone Rust-based post-quantum cryptographic module developed for the **Cerulean Ledger** distributed ledger platform. It isolates all cryptographic operations behind a strict API boundary with approved-mode enforcement, startup self-tests, and automatic key zeroization.

## What It Implements

| Algorithm | Standard | Purpose |
|-----------|----------|---------|
| ML-DSA-65 | FIPS 204 | Digital signatures (key generation, signing, verification) |
| ML-KEM-768 | FIPS 203 | Key encapsulation and decapsulation |
| SHA3-256 | FIPS 202 | Cryptographic hashing |

Non-approved legacy algorithms (Ed25519, SHA-256, HMAC-SHA256) are present for backward compatibility with pre-PQC ledger data. They are runtime-gated and blocked when the module operates in approved mode. A compile-time feature flag (`approved-only`) removes them entirely.

## Why It Matters

The transition to post-quantum cryptography is a pressing requirement for organizations handling sensitive data with long-term confidentiality needs. NIST has finalized FIPS 203 (ML-KEM) and FIPS 204 (ML-DSA) as the first post-quantum standards. Cerulean Ledger is built from the ground up with PQC as the default, positioning it ahead of the migration curve mandated by NSM-10 and OMB M-23-02.

## Current Readiness

- **Architecture**: Clean cryptographic boundary (11 source files, single public API module).
- **Documentation**: 9 FIPS 140-3 documentation artifacts produced (Security Policy, Design Document, FSM, Key Management, Self-Tests, Non-Approved Usage, Boundary Definition, Reproducible Build, Operational Guidance).
- **Testing**: 1500+ tests across 12 test suites; 100% boundary compliance verified.
- **Self-Tests**: Known Answer Tests (KATs) for all approved algorithms run at module initialization; module enters error state on failure.
- **Key Management**: All key types implement `ZeroizeOnDrop`; keys are never exposed outside the boundary.

## What We Are Seeking

We are seeking engagement with an NVLAP-accredited laboratory for **FIPS 140-3 Level 1 validation** of `pqc_crypto_module`. Specifically:

1. **CAVP algorithm certificate testing** for ML-DSA-65, ML-KEM-768, and SHA3-256.
2. **FIPS 140-3 module validation** at Security Level 1 (software-only).
3. **Guidance** on NIST ACVP test vector availability for post-quantum algorithms.
4. **Consultation** on SP 800-90B entropy source compliance for `OsRng`.

## Contact

[Organization contact details to be filled in before sending]

---

*For technical details, see the accompanying `module_overview.md`.*
