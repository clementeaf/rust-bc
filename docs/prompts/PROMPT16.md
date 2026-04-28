You are a senior Rust DevSecOps engineer focused on CI reliability, deterministic testing, and coverage engineering.

Your task is to harden the existing self-auditing CI pipeline by addressing the final operational risks:

1. Chaos/fuzz flakiness
2. CI runtime cost
3. Numeric coverage visibility

The DLT already has:

* PQC security tests
* FIPS-oriented crypto module tests
* crypto boundary enforcement
* property tests
* fuzz targets
* chaos network tests
* Byzantine/equivocation tests
* persistence tests
* DoS tests
* performance guardrails
* GitHub Actions workflows

Now optimize the pipeline so it is reliable, fast enough for PRs, and measurable.

---

# Objective

Make CI:

* deterministic
* reproducible
* non-flaky
* split correctly between PR-critical and nightly-heavy jobs
* coverage-aware
* easy to diagnose when failing

---

# Part 1 — Deterministic chaos/fuzz execution

## Required changes

All chaos, network, Byzantine, partition, DoS, and property tests must use fixed seeds.

Create a shared seed list:

```rust
pub const CI_SEEDS: &[u64] = &[
    1,
    42,
    1337,
    9001,
    123456789,
];
```

Use these seeds consistently in:

```text
tests/chaos_network.rs
tests/byzantine_equivocation.rs
tests/equivocation_persistence_partition.rs
tests/crypto_dos_flood.rs
tests/property_invariants.rs
```

---

## Required diagnostics

On failure, print:

* seed
* test name
* node id
* state hash
* canonical tip
* height
* rejected count by reason
* last 10 blocks
* elapsed time

---

## Add retry only for known nondeterministic external conditions

For CI only:

* retry chaos tests once on failure
* do NOT retry deterministic unit tests
* do NOT hide real failures

Implement either:

* GitHub Actions retry wrapper
* or `cargo-nextest` with retry config

Preferred:

```bash
cargo nextest run --retries 1 --test chaos_network
```

---

# Part 2 — Split PR vs nightly pipeline

## PR pipeline must run critical fast gates only

Update:

```text
.github/workflows/ci.yml
```

PR should run:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test -p pqc_crypto_module
cargo test --test crypto_boundary
cargo test --test fips_readiness
cargo test --test pqc_security_audit
cargo test --test property_invariants
cargo test --test performance_guardrails
```

Do NOT run full chaos, full fuzz, full Criterion benchmarks in PR.

---

## Nightly pipeline must run full suite

Update:

```text
.github/workflows/nightly-chaos.yml
```

Nightly should run:

```bash
cargo test
cargo test --test chaos_network -- --nocapture
cargo test --test persistent_crash_recovery -- --nocapture
cargo test --test crypto_dos_flood -- --nocapture
cargo test --test byzantine_equivocation -- --nocapture
cargo test --test equivocation_persistence_partition -- --nocapture
cargo bench --bench pqc_performance
```

---

# Part 3 — Add cargo-nextest

Add:

```text
.nextest/config.toml
```

Suggested config:

```toml
[profile.default]
retries = 0
slow-timeout = { period = "60s", terminate-after = 2 }

[profile.chaos]
retries = 1
slow-timeout = { period = "120s", terminate-after = 2 }
failure-output = "immediate-final"
success-output = "never"
```

Update workflows to install:

```bash
cargo install cargo-nextest --locked
```

Use nextest for large test groups.

---

# Part 4 — Coverage visibility

Create:

```text
.github/workflows/coverage.yml
```

Run on:

```yaml
on:
  pull_request:
  push:
    branches: [main]
```

Use:

```bash
cargo install cargo-llvm-cov --locked
cargo llvm-cov --workspace --lcov --output-path lcov.info
cargo llvm-cov --workspace --summary-only
```

Upload:

```text
lcov.info
coverage-summary.txt
```

as artifacts.

Do not fail CI on threshold initially.

Optional: add soft target:

```text
Target coverage: 80% lines
Status: informational only
```

---

# Part 5 — CI runtime budget report

Add a lightweight script:

```text
scripts/ci_runtime_report.sh
```

It should print:

* job name
* elapsed time
* number of tests run
* slowest tests if available
* recommendation if runtime exceeds target

Suggested budgets:

```text
PR CI: <= 15 minutes
Security audit: <= 20 minutes
Performance guardrails: <= 10 minutes
Nightly full suite: <= 90 minutes
```

---

# Part 6 — README update

Add section:

```text
## CI Strategy

PR gates:
- fast critical safety checks
- crypto boundary
- FIPS readiness
- property invariants
- performance guardrails

Nightly:
- full chaos
- fuzz
- persistence
- benchmarks

Coverage:
- informational coverage report generated automatically
```

---

# Final commands

Run locally:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test -p pqc_crypto_module
cargo test --test crypto_boundary
cargo test --test fips_readiness
cargo test --test pqc_security_audit
cargo test --test property_invariants
cargo test --test performance_guardrails
cargo nextest run --profile chaos --test chaos_network
cargo llvm-cov --workspace --summary-only
```

---

# Final output format

Report:

1. Seeds standardized
2. Flaky tests stabilized
3. PR pipeline optimized
4. Nightly pipeline confirmed full coverage
5. cargo-nextest config added
6. Coverage workflow added
7. Runtime budget script added
8. README updated
9. Any flaky/slow tests found
10. Final statement:

```text
CI reliability hardening complete.
PRs run fast critical gates, nightly runs full adversarial validation, and coverage is now visible.
```

Be strict. The goal is not more tests — it is reliable, reproducible, and maintainable verification.
