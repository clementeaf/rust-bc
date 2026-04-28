# CMVP Submission Checklist — pqc_crypto_module v0.1.0

> **Disclaimer**: Prepared for FIPS 140-3 evaluation, not currently validated.

---

## 1. Required CMVP Artifacts

| # | Artifact | Status | Owner | Location |
|---|----------|--------|-------|----------|
| 1 | Security Policy | READY | [TBD] | `crates/pqc_crypto_module/SECURITY_POLICY.md` |
| 2 | Design Document | READY | [TBD] | `crates/pqc_crypto_module/DESIGN_DOCUMENT.md` |
| 3 | Finite State Model | READY | [TBD] | `crates/pqc_crypto_module/FINITE_STATE_MODEL.md` |
| 4 | Key Management | READY | [TBD] | `crates/pqc_crypto_module/KEY_MANAGEMENT.md` |
| 5 | Self-Test Documentation | READY | [TBD] | `crates/pqc_crypto_module/SELF_TEST_DOCUMENTATION.md` |
| 6 | Non-Approved Usage | READY | [TBD] | `crates/pqc_crypto_module/NON_APPROVED_USAGE.md` |
| 7 | Boundary Definition | READY | [TBD] | `crates/pqc_crypto_module/build/module_boundary_definition.md` |
| 8 | Reproducible Build | READY | [TBD] | `crates/pqc_crypto_module/build/reproducible_build.md` |
| 9 | Test Coverage Report | READY | [TBD] | 1500+ tests across 12 suites; `cargo test` |
| 10 | Operational Guidance | READY | [TBD] | `crates/pqc_crypto_module/OPERATIONAL_GUIDANCE.md` |

## 2. Pending Artifacts

| # | Artifact | Status | Owner | Notes |
|---|----------|--------|-------|-------|
| 11 | NIST Official Test Vectors | NEEDS WORK | [TBD] | Internal KATs exist; NIST ACVP vectors not yet integrated |
| 12 | CAVP Algorithm Certificates | NEEDS WORK | [TBD] | Requires lab engagement; ML-DSA, SHA3-256, ML-KEM |
| 13 | Lab Engagement | NOT STARTED | [TBD] | No lab selected; see `LAB_SELECTION.md` |
| 14 | Entropy Source Documentation (SP 800-90B) | NEEDS WORK | [TBD] | OsRng used; compliance documentation pending |
| 15 | Vendor Evidence Package | NOT STARTED | [TBD] | Lab-specific format; prepared after lab selection |

## 3. Pre-Submission Verification

- [x] All approved algorithms documented with standards references (FIPS 202/203/204)
- [x] Non-approved algorithms identified, gated, and documented
- [x] State machine: Uninitialized -> SelfTesting -> Approved -> Error
- [x] Boundary definition: 11 source files in `src/`, Rust crate isolation
- [x] Self-tests: KATs for ML-DSA-65, SHA3-256 at startup
- [x] Key zeroization: `ZeroizeOnDrop` on all key types
- [x] Approved-mode enforcement: `require_approved()` guard on all operations
- [x] `approved-only` Cargo feature excludes legacy module at compile time
- [ ] Official NIST ACVP test vectors integrated
- [ ] CAVP certificates obtained for each approved algorithm
- [ ] Lab selected and engagement initiated
- [ ] SP 800-90B entropy source analysis completed

## 4. Document Review Status

| Document | Internal Review | External Review |
|----------|----------------|-----------------|
| Security Policy | Complete | Pending lab |
| Design Document | Complete | Pending lab |
| Finite State Model | Complete | Pending lab |
| Key Management | Complete | Pending lab |
| Self-Test Documentation | Complete | Pending lab |
| Non-Approved Usage | Complete | Pending lab |
| Boundary Definition | Complete | Pending lab |
| Reproducible Build | Complete | Pending lab |

## 5. Next Steps

1. Integrate NIST ACVP official test vectors (see `TEST_VECTOR_PLAN.md`)
2. Select accredited lab (see `LAB_SELECTION.md`)
3. Prepare contact package for lab outreach (see `CONTACT_PACKAGE/`)
4. Address gap analysis findings (see `GAP_ANALYSIS.md`)
5. Begin CAVP algorithm certificate process with selected lab
