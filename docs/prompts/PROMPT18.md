You are a senior FIPS 140-3 / CMVP readiness engineer and Rust cryptographic validation specialist.

Your task is to close ALL remaining pre-lab blockers for the `pqc_crypto_module` so the package is as strong as possible before contacting a FIPS 140-3 accredited laboratory.

Current pre-lab audit status:

* Mock audit findings: 0 CRITICAL, 2 HIGH, 2 MEDIUM, 2 LOW, 3 INFO
* Traceability: 11/12 PASS, 1 PARTIAL
* Blocking gaps:

  * F-01: ML-KEM placeholder
  * F-02: ACVP vectors / dry-run not complete

Goal:

```text
0 CRITICAL
0 HIGH
0 MEDIUM blocking findings
Traceability: 12/12 PASS
No placeholder cryptographic implementation
ACVP dry-run harness functional
Pre-lab package ready for lab intake
```

Important:

This still does NOT mean FIPS certified or CMVP validated.
The goal is to remove all avoidable technical/documentation gaps before lab engagement.

---

# PART 1 — Remove ML-KEM placeholder blocker

## Objective

Eliminate any “placeholder” ML-KEM implementation or wording.

ML-KEM-768 must be implemented through a real cryptographic backend already used by the project or an explicit approved candidate dependency.

---

## Required actions

1. Inspect:

```text
crates/pqc_crypto_module/src/mlkem.rs
crates/pqc_crypto_module/src/api.rs
crates/pqc_crypto_module/src/self_tests.rs
crates/pqc_crypto_module/SECURITY_POLICY.md
crates/pqc_crypto_module/DESIGN_DOCUMENT.md
crates/pqc_crypto_module/SELF_TEST_DOCUMENTATION.md
pre_lab_audit/GAP_ANALYSIS.md
pre_lab_audit/FINDINGS_REGISTER.md
pre_lab_audit/TRACEABILITY_MATRIX.md
```

2. Remove or replace all wording like:

```text
placeholder
stub
mock
temporary
future implementation
TODO ML-KEM
```

3. Implement ML-KEM-768 using a real backend.

Preferred options:

```text
pqcrypto-mlkem
pqcrypto-kyber if already used and clearly mapped to ML-KEM-768 compatibility
rustls-post-quantum backend if it exposes ML-KEM primitives cleanly
```

4. Expose only approved-boundary APIs:

```rust
generate_mlkem_keypair()
mlkem_encapsulate()
mlkem_decapsulate()
```

5. Ensure:

```rust
decapsulate(private_key, ciphertext_from_encapsulate(public_key))
    == shared_secret_from_encapsulate
```

6. Ensure invalid ciphertext fails closed or returns an explicit error.

7. Ensure private keys and shared secrets zeroize on drop.

8. Add/verify tests:

```rust
mlkem_keypair_encaps_decaps_roundtrip
mlkem_invalid_ciphertext_rejected
mlkem_private_key_zeroizes_on_drop
mlkem_shared_secret_zeroizes_on_drop
mlkem_api_rejects_before_approved_mode
mlkem_api_works_after_approved_mode
```

9. Update self-tests so ML-KEM is tested during:

```rust
initialize_approved_mode()
```

10. Re-run:

```bash
cargo test -p pqc_crypto_module
cargo test --test fips_readiness
```

---

# PART 2 — Build functional ACVP dry-run harness

## Objective

Create a local ACVP-style vector runner that proves the module can process deterministic vector files for:

* SHA3-256
* ML-DSA-65
* ML-KEM-768

This is not official ACVP submission.
It is a dry-run readiness harness.

---

## Target structure

Create or complete:

```text
tools/acvp_dry_run/
├── Cargo.toml
├── README.md
├── src/
│   ├── main.rs
│   ├── vectors.rs
│   ├── sha3.rs
│   ├── mldsa.rs
│   ├── mlkem.rs
│   └── report.rs
└── vectors/
    ├── sha3_256.json
    ├── mldsa_65.json
    └── mlkem_768.json
```

---

## CLI requirements

Support:

```bash
cargo run -p acvp_dry_run -- --algorithm sha3-256 --vectors tools/acvp_dry_run/vectors/sha3_256.json
cargo run -p acvp_dry_run -- --algorithm ml-dsa-65 --vectors tools/acvp_dry_run/vectors/mldsa_65.json
cargo run -p acvp_dry_run -- --algorithm ml-kem-768 --vectors tools/acvp_dry_run/vectors/mlkem_768.json
cargo run -p acvp_dry_run -- --all
```

---

## JSON format requirements

Use ACVP-inspired JSON, not necessarily official final schema.

### SHA3 vector example

```json
{
  "algorithm": "sha3-256",
  "testGroups": [
    {
      "tgId": 1,
      "tests": [
        {
          "tcId": 1,
          "msgHex": "",
          "expectedDigestHex": "a7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a"
        }
      ]
    }
  ]
}
```

### ML-DSA vector example

Use deterministic test vectors if the backend supports deterministic signing.

If signing is randomized/non-deterministic, split into:

* verify vectors
* sign-then-verify generated vectors

Example:

```json
{
  "algorithm": "ml-dsa-65",
  "testGroups": [
    {
      "tgId": 1,
      "tests": [
        {
          "tcId": 1,
          "messageHex": "74657374",
          "mode": "signThenVerify"
        }
      ]
    }
  ]
}
```

### ML-KEM vector example

If encapsulation is randomized, use:

* encapsThenDecaps
* decapsKnownCiphertext if deterministic vectors are available

Example:

```json
{
  "algorithm": "ml-kem-768",
  "testGroups": [
    {
      "tgId": 1,
      "tests": [
        {
          "tcId": 1,
          "mode": "encapsThenDecaps"
        }
      ]
    }
  ]
}
```

---

## Dry-run behavior

For each vector:

* parse JSON
* run module API
* compare expected output if provided
* otherwise perform generated invariant validation
* produce a result JSON/report:

```json
{
  "algorithm": "ml-kem-768",
  "passed": 10,
  "failed": 0,
  "results": [
    {
      "tcId": 1,
      "status": "passed"
    }
  ]
}
```

---

## Required tests for dry-run tool

Add:

```text
tools/acvp_dry_run/tests/acvp_dry_run.rs
```

Tests:

```rust
sha3_vectors_pass
mldsa_sign_then_verify_vectors_pass
mlkem_encaps_then_decaps_vectors_pass
invalid_vector_file_fails_cleanly
unknown_algorithm_rejected
```

---

## Required commands

Run:

```bash
cargo test -p acvp_dry_run
cargo run -p acvp_dry_run -- --all
```

---

# PART 3 — Update pre-lab audit package

## Objective

Close F-01 and F-02 in the audit docs.

Update:

```text
pre_lab_audit/FINDINGS_REGISTER.md
pre_lab_audit/GAP_ANALYSIS.md
pre_lab_audit/TRACEABILITY_MATRIX.md
pre_lab_audit/ACVP_DRY_RUN_PLAN.md
pre_lab_audit/MOCK_AUDIT_REPORT.md
```

Expected final status:

```text
F-01 ML-KEM placeholder: CLOSED
F-02 ACVP dry-run: CLOSED / PARTIAL-OFFICIAL
```

Use this distinction:

* CLOSED for local dry-run readiness
* PARTIAL-OFFICIAL only because official lab ACVP execution is external

---

# PART 4 — Add CI coverage for ACVP dry-run

Update or create:

```text
.github/workflows/pre-lab-audit.yml
```

Add:

```bash
cargo test -p acvp_dry_run
cargo run -p acvp_dry_run -- --all
cargo test -p pqc_crypto_module
cargo test --test fips_readiness
```

Upload:

```text
pre_lab_audit/
tools/acvp_dry_run/report.json
```

as artifacts if generated.

---

# PART 5 — Final validation sweep

Run all critical validation:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test -p pqc_crypto_module
cargo test -p acvp_dry_run
cargo run -p acvp_dry_run -- --all
cargo test --test crypto_boundary
cargo test --test fips_readiness
cargo test --test pqc_security_audit
cargo test --test property_invariants
cargo test --test performance_guardrails
cargo test
```

---

# Final output format

Report:

1. F-01 ML-KEM placeholder status
2. ML-KEM backend used
3. ML-KEM tests added/passed
4. F-02 ACVP dry-run status
5. ACVP dry-run algorithms covered
6. ACVP dry-run commands and results
7. Pre-lab findings remaining by severity
8. Traceability matrix final status
9. CI pre-lab audit integration status
10. Any remaining non-blocking caveats
11. Exact commands run
12. Final statement:

```text
All avoidable pre-lab technical gaps closed.
The module is ready for FIPS 140-3 laboratory intake.
Certification is still not guaranteed and requires accredited lab testing and CMVP validation.
```

Be strict. Do not mark official ACVP or CMVP validation as complete unless it actually happened.
