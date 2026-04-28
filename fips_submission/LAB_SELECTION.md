# FIPS 140-3 Accredited Lab Selection — pqc_crypto_module v0.1.0

> **Disclaimer**: Prepared for FIPS 140-3 evaluation, not currently validated.

---

## 1. Candidate Labs

### 1.1 atsec Information Security

- **Location**: Austin, TX, USA (with offices in Germany)
- **Website**: https://www.atsec.com
- **FIPS 140-3 Experience**: Long-standing NVLAP-accredited lab; extensive FIPS 140-2 and 140-3 validation history across software, firmware, and hardware modules.
- **PQC Experience**: Active in standards evolution; positioned for FIPS 203/204 algorithm testing as NIST finalizes ACVP support.
- **Estimated Responsiveness**: High. Known for structured onboarding and clear communication.
- **Notes**: Strong reputation in open-source and Linux-based module validation. Good fit for Rust/software-only modules.

### 1.2 UL Solutions (formerly UL Verification Services / InfoGard successor programs)

- **Location**: Northbrook, IL, USA (global presence)
- **Website**: https://www.ul.com
- **FIPS 140-3 Experience**: One of the largest NVLAP-accredited labs with hundreds of completed validations.
- **PQC Experience**: Broad cryptographic algorithm testing capabilities; expected early adoption of PQC CAVP testing.
- **Estimated Responsiveness**: Medium-High. Large lab with multiple concurrent engagements; scheduling may vary.
- **Notes**: Full-service lab covering FIPS 140-3, Common Criteria, and PCI. Suitable for vendors seeking multiple certifications.

### 1.3 Acumen Security

- **Location**: McLean, VA, USA
- **Website**: https://www.acumensecurity.com
- **FIPS 140-3 Experience**: Specialized FIPS 140-3 testing lab with deep expertise in software cryptographic modules.
- **PQC Experience**: Actively tracking NIST PQC standards; positioned for early ML-DSA/ML-KEM validation support.
- **Estimated Responsiveness**: High. Smaller lab with focused attention per engagement.
- **Notes**: Known for working closely with vendors on documentation and design alignment. Good fit for first-time CMVP submitters.

### 1.4 Leidos (Cyber Innovation Center)

- **Location**: Reston, VA, USA
- **Website**: https://www.leidos.com
- **FIPS 140-3 Experience**: NVLAP-accredited lab with government and defense sector validation experience.
- **PQC Experience**: Government contracts position them close to NIST PQC transition timelines.
- **Estimated Responsiveness**: Medium. Larger organization; engagement timelines depend on current workload.
- **Notes**: Strong government sector relationships. Appropriate if targeting US federal procurement channels.

### 1.5 InfoGard Laboratories

- **Location**: San Luis Obispo, CA, USA
- **Website**: https://www.infogard.com
- **FIPS 140-3 Experience**: Established NVLAP-accredited lab with decades of FIPS validation experience.
- **PQC Experience**: General cryptographic testing; PQC-specific CAVP testing readiness to be confirmed.
- **Estimated Responsiveness**: Medium-High. Mid-sized lab with dedicated validation teams.
- **Notes**: Well-known in the CMVP community. Solid choice for straightforward software module validations.

## 2. Selection Criteria

| Criterion | Weight | Description |
|-----------|--------|-------------|
| FIPS 140-3 Track Record | High | Number of completed FIPS 140-3 validations, especially software modules |
| PQC Readiness | High | Ability to test ML-DSA-65 (FIPS 204), ML-KEM-768 (FIPS 203), SHA3-256 (FIPS 202) |
| Rust Module Experience | Medium | Prior experience validating Rust-based cryptographic modules |
| Communication Quality | Medium | Responsiveness, clarity of feedback, structured onboarding process |
| Timeline Predictability | Medium | Ability to provide and adhere to estimated schedules |
| Cost Transparency | Medium | Clear pricing structure; no hidden fees for iterations |
| Geographic Accessibility | Low | Timezone alignment for meetings; in-person visits if needed |

## 3. Recommended Approach

1. **Contact 2-3 labs** with the executive summary and module overview from `CONTACT_PACKAGE/`.
2. **Request**: estimated timeline, cost range, PQC algorithm testing readiness, and any Rust-specific considerations.
3. **Evaluate** responses against the criteria above.
4. **Select** based on PQC readiness and communication quality as primary differentiators, given the novelty of ML-DSA/ML-KEM validation.

## 4. Initial Contact Priority

| Priority | Lab | Rationale |
|----------|-----|-----------|
| 1 | Acumen Security | Specialized focus, vendor-friendly, high responsiveness |
| 2 | atsec | Strong open-source track record, structured process |
| 3 | UL Solutions | Scale and breadth; fallback if top choices unavailable |
