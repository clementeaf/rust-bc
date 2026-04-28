# Validation Timeline — pqc_crypto_module v0.1.0

> **Disclaimer**: Prepared for FIPS 140-3 evaluation, not currently validated.

---

## 1. Phase Overview

| Phase | Duration (Est.) | Description |
|-------|-----------------|-------------|
| 1. Lab Selection | 2 weeks | Evaluate labs, send contact packages, receive proposals |
| 2. Lab Onboarding | 2-4 weeks | Contract signing, NDA, scope definition, kickoff meeting |
| 3. Pre-Testing | 4-8 weeks | ACVP harness build, CAVP submissions, documentation review with lab |
| 4. Formal Testing & Iterations | 2-6 months | Lab testing, findings resolution, re-testing cycles |
| 5. CMVP Queue & Review | 6-12 months | CMVP processes the validation report; may request clarifications |
| **Total Estimated** | **12-24 months** | From lab selection to FIPS 140-3 certificate issuance |

## 2. Detailed Phase Breakdown

### Phase 1: Lab Selection (Weeks 1-2)

| Milestone | Target | Owner |
|-----------|--------|-------|
| Send contact packages to 2-3 labs | Week 1 | [TBD] |
| Receive lab proposals and cost estimates | Week 2 | [TBD] |
| Select lab and notify | Week 2 | [TBD] |

**Deliverables:** Signed letter of intent or contract initiation.

### Phase 2: Lab Onboarding (Weeks 3-6)

| Milestone | Target | Owner |
|-----------|--------|-------|
| Execute NDA and services agreement | Week 3 | [TBD] |
| Kickoff meeting: scope, timeline, deliverables | Week 4 | [TBD] |
| Receive lab-specific documentation requirements | Week 5 | [TBD] |
| Submit initial documentation package for review | Week 6 | [TBD] |

**Deliverables:** Executed contract, agreed scope, documentation submission.

### Phase 3: Pre-Testing (Weeks 7-14)

| Milestone | Target | Owner |
|-----------|--------|-------|
| Build ACVP test harness | Weeks 7-8 | Engineering |
| Integrate official NIST SHA-3 test vectors | Week 8 | Engineering |
| Submit SHA-3 for CAVP certificate | Week 9 | Lab |
| Address lab documentation feedback (round 1) | Weeks 10-12 | Engineering |
| Integrate ML-DSA/ML-KEM ACVP vectors (if available) | Weeks 12-14 | Engineering |
| Submit PQC algorithms for CAVP certificates | Week 14 | Lab |

**Deliverables:** ACVP harness, CAVP submissions, revised documentation.

### Phase 4: Formal Testing & Iterations (Months 4-9)

| Milestone | Target | Owner |
|-----------|--------|-------|
| Lab begins formal FIPS 140-3 testing | Month 4 | Lab |
| Receive initial test findings | Month 5 | Lab |
| Remediate findings (code or documentation) | Months 5-7 | Engineering |
| Lab re-testing after remediation | Months 7-8 | Lab |
| Final test report prepared by lab | Month 9 | Lab |

**Deliverables:** Passing test results, final validation report.

### Phase 5: CMVP Queue & Review (Months 10-21)

| Milestone | Target | Owner |
|-----------|--------|-------|
| Lab submits validation report to CMVP | Month 10 | Lab |
| CMVP assigns reviewer | Months 10-12 | CMVP |
| CMVP review and potential questions | Months 12-18 | CMVP / Engineering |
| FIPS 140-3 certificate issued | Months 18-21 | CMVP |

**Deliverables:** FIPS 140-3 validation certificate.

## 3. Key Milestones

```
Month 0        Lab selected
Month 1        Onboarding complete, documentation submitted
Month 2-3      ACVP harness ready, CAVP submissions initiated
Month 4-5      SHA-3 CAVP certificate obtained
Month 6-9      Formal testing and iterations
Month 9-10     Validation report submitted to CMVP
Month 18-21    Certificate issued (optimistic)
Month 24       Certificate issued (conservative)
```

## 4. Risk Factors

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| NIST ACVP PQC support delayed | Blocks ML-DSA/ML-KEM CAVP certificates | Medium | Begin with SHA-3; proceed with PQC when available |
| ML-KEM-768 Rust crate not available | Blocks ML-KEM CAVP testing | Medium | Monitor ecosystem; consider C FFI wrapper as fallback |
| Lab findings require architectural changes | Adds 2-4 months | Low | Module architecture is well-aligned with FIPS requirements |
| CMVP queue backlog exceeds 12 months | Extends total timeline | Medium | Submit early; no mitigation for queue delays |
| Documentation revision cycles | Each cycle adds 2-4 weeks | Medium | Produce high-quality docs upfront (current state is strong) |
| Budget constraints delay lab engagement | Delays entire timeline | Medium | Obtain budget approval before Phase 1 |
| Rust-specific lab unfamiliarity | Adds onboarding time | Low-Medium | Provide clear build instructions and reproducible environment |

## 5. Cost Considerations

Typical FIPS 140-3 Level 1 software module validation costs:

| Item | Estimated Range (USD) |
|------|----------------------|
| Lab testing fees | $50,000 - $150,000 |
| CAVP algorithm testing | $10,000 - $30,000 (per algorithm family) |
| Documentation review iterations | Included in lab fees (typically) |
| CMVP submission fee | Included in lab fees (typically) |
| Internal engineering effort | 3-6 person-months |
| **Total estimated** | **$80,000 - $250,000** |

Note: Costs vary significantly by lab, module complexity, and number of iteration cycles. PQC algorithms may carry a premium due to novelty. Request detailed quotes during Phase 1.

## 6. Success Criteria

- [ ] CAVP certificates obtained for ML-DSA-65, ML-KEM-768, SHA3-256
- [ ] Lab testing completed with all findings resolved
- [ ] Validation report accepted by CMVP
- [ ] FIPS 140-3 Level 1 certificate issued for `pqc_crypto_module v0.1.0`
