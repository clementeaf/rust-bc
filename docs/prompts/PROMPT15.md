You are a senior DevSecOps engineer specialized in Rust, blockchain protocols, cryptographic validation, and CI hardening.

Your task is to create a full automated CI pipeline for this Rust DLT so that every commit automatically validates:

* PQC security
* crypto boundary enforcement
* FIPS-oriented module behavior
* property tests
* fuzz tests
* chaos network tests
* Byzantine/equivocation tests
* slashing lifecycle
* persistence recovery
* Crypto-DoS resistance
* performance guardrails

---

# Objective

Create a CI pipeline that makes the DLT **self-auditing on every commit**.

The pipeline must fail if any regression appears in:

* PQC enforcement
* crypto boundary
* approved vs legacy separation
* consensus convergence
* Byzantine behavior
* storage persistence
* DoS resistance
* performance minimums

---

# Target CI

Prefer GitHub Actions.

Create:

```text
.github/workflows/
├── ci.yml
├── security-audit.yml
├── fuzz.yml
├── performance-guardrails.yml
└── nightly-chaos.yml
```

---

# 1. Main CI pipeline

Create:

```text
.github/workflows/ci.yml
```

Run on:

```yaml
on:
  pull_request:
  push:
    branches: [main, dev]
```

Steps:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo test -p pqc_crypto_module
cargo test --test crypto_boundary
cargo test --test fips_readiness
```

Must fail on:

* formatting issues
* clippy warnings
* test failures
* crypto boundary violations
* approved mode violations

---

# 2. Security audit pipeline

Create:

```text
.github/workflows/security-audit.yml
```

Run:

```bash
cargo audit
cargo deny check
cargo test --test pqc_security_audit
cargo test --test byzantine_equivocation
cargo test --test equivocation_persistence_partition
cargo test --test slashing_penalty_lifecycle
cargo test --test crypto_dos_flood
```

Also add:

```text
deny.toml
```

With policies for:

* banned duplicate dependencies where possible
* known vulnerable crates
* unmaintained crates
* unexpected licenses

---

# 3. Fuzz pipeline

Create:

```text
.github/workflows/fuzz.yml
```

Use:

```bash
cargo install cargo-fuzz
cargo fuzz run block_parser -- -max_total_time=60
cargo fuzz run transaction_parser -- -max_total_time=60
cargo fuzz run signature_parser -- -max_total_time=60
cargo fuzz run gossip_message_parser -- -max_total_time=60
```

If fuzz targets do not exist, create:

```text
fuzz/fuzz_targets/
├── block_parser.rs
├── transaction_parser.rs
├── signature_parser.rs
└── gossip_message_parser.rs
```

Targets must test:

* malformed blocks
* malformed transactions
* corrupted signatures
* malformed gossip messages
* random bytes
* truncated input
* oversized input

No panic allowed.

---

# 4. Performance guardrails pipeline

Create:

```text
.github/workflows/performance-guardrails.yml
```

Run:

```bash
cargo test --test performance_guardrails
```

Do NOT run full Criterion benchmarks on every PR unless fast enough.

Guardrails must validate:

* cheap rejection remains at least 10x faster than ML-DSA verification
* duplicate flood does not trigger unbounded verification
* RocksDB restart for 10K blocks stays under threshold
* strict PQC validation meets minimum blocks/sec
* SHA3 overhead remains bounded
* ML-DSA overhead remains bounded

---

# 5. Nightly chaos pipeline

Create:

```text
.github/workflows/nightly-chaos.yml
```

Run only nightly or manually:

```yaml
on:
  schedule:
    - cron: "0 3 * * *"
  workflow_dispatch:
```

Run heavy tests:

```bash
cargo test --test chaos_network -- --nocapture
cargo test --test persistent_crash_recovery -- --nocapture
cargo test --test crypto_dos_flood -- --nocapture
cargo test --test byzantine_equivocation -- --nocapture
cargo test --test equivocation_persistence_partition -- --nocapture
cargo bench --bench pqc_performance
```

Upload artifacts:

* benchmark output
* chaos logs
* failure diagnostics
* coverage reports if available

---

# 6. Property tests

If not already present, add:

```text
tests/property_invariants.rs
```

Using `proptest`.

Required invariants:

1. Tampering any signed payload invalidates signature.
2. Same proposer cannot create two canonical blocks for same position.
3. Duplicate messages do not change state.
4. Hash changes if block content changes.
5. Serialization roundtrip preserves PQC metadata.
6. Strict PQC mode never accepts classical-only signatures.

Run in CI:

```bash
cargo test --test property_invariants
```

---

# 7. Coverage

Add optional coverage workflow or step using:

```bash
cargo install cargo-llvm-cov
cargo llvm-cov --workspace --lcov --output-path lcov.info
```

Upload coverage artifact.

Do not fail build on coverage at first unless configured.

---

# 8. Secrets and environment

All CI jobs must set strict mode:

```yaml
env:
  REQUIRE_PQC_SIGNATURES: "true"
  TLS_PQC_KEM: "true"
  DUAL_SIGN_VERIFY_MODE: "both"
  HASH_ALGORITHM: "sha3-256"
  SIGNING_ALGORITHM: "ml-dsa-65"
```

---

# 9. README badges

Update root README with badges:

* CI
* Security audit
* Fuzz
* Nightly chaos
* Performance guardrails

---

# 10. Required final commands

After implementing, run locally:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo test -p pqc_crypto_module
cargo test --test crypto_boundary
cargo test --test fips_readiness
cargo test --test pqc_security_audit
cargo test --test property_invariants
cargo test --test performance_guardrails
```

---

# Final output format

Report:

1. CI workflows created
2. Fuzz targets created
3. Property tests added
4. Security audit tools configured
5. Performance guardrails wired into CI
6. Nightly chaos pipeline configured
7. README badges added
8. Whether all local commands passed
9. Any flaky/slow tests identified
10. Final statement:

```text
Self-auditing CI pipeline implemented.
Every commit now validates PQC security, crypto boundary, FIPS-oriented behavior, Byzantine safety, persistence, DoS resistance, and performance guardrails.
```

Be strict. The goal is to prevent any future commit from weakening the DLT silently.
