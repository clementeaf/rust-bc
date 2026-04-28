You are a senior FIPS 140-3 pre-validation auditor, CMVP readiness reviewer, and ACVP integration engineer.

Your task is to run a **pre-lab mock audit** for the `pqc_crypto_module` and produce the strongest possible evidence package before contacting a FIPS 140-3 accredited laboratory.

Important:

This does NOT guarantee certification.
The goal is to reduce technical and documentation risk before formal CMVP submission.

---

# Objective

Create a full pre-lab audit package covering:

1. Mock FIPS 140-3 lab review
2. ACVP dry-run readiness
3. FIPS 140-3 Implementation Guidance checklist
4. Clean-room reproducible build verification
5. Entropy/RNG evidence
6. Vendor evidence package
7. Requirement → evidence traceability matrix

---

# Target directory

Create:

```text
pre_lab_audit/
├── MOCK_AUDIT_REPORT.md
├── ACVP_DRY_RUN_PLAN.md
├── FIPS_140_3_IG_CHECKLIST.md
├── CLEAN_ROOM_BUILD.md
├── ENTROPY_RNG_EVIDENCE.md
├── VENDOR_EVIDENCE_PACKAGE.md
├── TRACEABILITY_MATRIX.md
├── FINDINGS_REGISTER.md
└── README.md
```

---

# 1. Mock FIPS lab audit

Create:

```text
pre_lab_audit/MOCK_AUDIT_REPORT.md
```

Act like a hostile but fair FIPS lab reviewer.

Review:

* module boundary
* approved mode
* finite state model
* self-tests
* error state behavior
* zeroization
* non-approved algorithms
* RNG
* reproducible build
* operational guidance
* documentation consistency

Classify findings:

```text
CRITICAL
HIGH
MEDIUM
LOW
INFO
```

For each finding include:

```text
ID:
Severity:
Area:
Observation:
Risk:
Evidence:
Recommended fix:
Status:
```

Do not hide gaps. Be strict.

---

# 2. ACVP dry-run readiness

Create:

```text
pre_lab_audit/ACVP_DRY_RUN_PLAN.md
```

Define how the module will support vector-based validation for:

* ML-DSA-65
* ML-KEM-768
* SHA3-256

Include:

* expected input format
* expected output format
* deterministic test mode requirements
* JSON vector parser plan
* response generator plan
* negative vector handling
* known gaps

If actual ACVP vectors are not integrated, mark:

```text
Status: PARTIAL — official ACVP vector integration pending
```

Add proposed harness:

```text
tools/acvp_dry_run/
├── README.md
├── Cargo.toml
└── src/main.rs
```

Harness should support placeholder commands:

```bash
cargo run -p acvp_dry_run -- --algorithm sha3-256 --vectors vectors/sha3.json
cargo run -p acvp_dry_run -- --algorithm ml-dsa-65 --vectors vectors/mldsa.json
cargo run -p acvp_dry_run -- --algorithm ml-kem-768 --vectors vectors/mlkem.json
```

---

# 3. FIPS 140-3 IG checklist

Create:

```text
pre_lab_audit/FIPS_140_3_IG_CHECKLIST.md
```

Build a checklist with columns:

```text
Requirement / IG Area
Status: PASS / PARTIAL / FAIL / N/A
Evidence
Gap
Owner
Next action
```

Cover at least:

* module specification
* cryptographic boundary
* approved services
* non-approved services
* roles
* finite state model
* self-tests
* error handling
* key management
* zeroization
* entropy/RNG
* operational environment
* physical security applicability
* mitigation of other attacks
* lifecycle assurance
* design assurance
* guidance documents

---

# 4. Clean-room reproducible build

Create:

```text
pre_lab_audit/CLEAN_ROOM_BUILD.md
```

Define a repeatable verification process:

* Docker/VM clean environment
* Rust toolchain pinning
* Cargo.lock enforcement
* no network after dependency fetch
* build command
* artifact hashing
* expected output location
* repeat build comparison

Add script if useful:

```text
scripts/clean_room_build.sh
```

The script should:

1. print rustc/cargo versions
2. run cargo fetch
3. run cargo build --locked --release -p pqc_crypto_module
4. hash the produced artifact
5. output build metadata

---

# 5. Entropy/RNG evidence

Create:

```text
pre_lab_audit/ENTROPY_RNG_EVIDENCE.md
```

Document:

* OS entropy source
* RNG wrapper
* continuous RNG test
* failure handling
* no fallback behavior
* test hooks
* assumptions by OS

Add required evidence tests if missing:

```rust
rng_rejects_repeated_output_test_hook
rng_failure_fails_closed
rng_never_returns_empty_output
```

---

# 6. Vendor evidence package

Create:

```text
pre_lab_audit/VENDOR_EVIDENCE_PACKAGE.md
```

Include:

* SBOM generation plan
* dependency list
* license review
* known vulnerability scan
* cargo audit
* cargo deny
* third-party crypto dependency statement
* maintenance risk

Add commands:

```bash
cargo generate-lockfile
cargo audit
cargo deny check
cargo tree
cargo metadata --format-version 1
```

Optional SBOM:

```bash
cargo install cargo-cyclonedx
cargo cyclonedx --format json --output-file sbom.json
```

---

# 7. Traceability matrix

Create:

```text
pre_lab_audit/TRACEABILITY_MATRIX.md
```

Map:

```text
Requirement
Implementation file
Test file
Documentation file
Status
Notes
```

Must include:

* approved mode enforced
* self-tests run before use
* error state fail-closed
* ML-DSA sign/verify
* ML-KEM encaps/decaps
* SHA3 hash
* zeroization
* legacy blocked in approved mode
* no raw crypto outside module
* reproducible build
* RNG failure handling

---

# 8. Findings register

Create:

```text
pre_lab_audit/FINDINGS_REGISTER.md
```

Aggregate all findings from the mock audit.

Columns:

```text
ID
Severity
Area
Description
Owner
Status
Target date
Blocking for lab intake?
```

---

# 9. README

Create:

```text
pre_lab_audit/README.md
```

Explain:

* purpose of package
* what is ready
* what is not ready
* how to use the package with a lab
* disclaimer:

```text
This package does not imply FIPS 140-3 validation or CMVP certification.
```

---

# 10. CI integration

Add optional workflow:

```text
.github/workflows/pre-lab-audit.yml
```

Run:

```bash
cargo test -p pqc_crypto_module
cargo test --test crypto_boundary
cargo test --test fips_readiness
cargo audit
cargo deny check
bash scripts/clean_room_build.sh
```

Upload artifacts:

* pre_lab_audit/
* build metadata
* SBOM if generated

---

# Final output format

Report:

1. Pre-lab audit package created
2. Mock audit findings by severity
3. ACVP dry-run readiness status
4. FIPS IG checklist status
5. Clean-room build status
6. Entropy/RNG evidence status
7. Vendor evidence status
8. Traceability matrix status
9. Blocking gaps before lab contact
10. Final statement:

```text
Pre-lab audit package complete.
Certification is not guaranteed, but technical and documentation risk before lab intake has been reduced as much as possible.
```

Be strict. Do not mark uncertain areas as complete.
