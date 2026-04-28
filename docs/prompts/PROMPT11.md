You are a senior Rust architecture auditor and refactoring engineer.

Goal: migrate the DLT to **100% crypto boundary compliance**.

Current state:

* `tests/crypto_boundary.rs` exists
* 5 crypto boundary tests pass
* 189 Rust files scanned
* 162/189 files clean
* 27/28 legacy direct-crypto files documented in allowlist
* New files with direct crypto imports fail the build
* Current status: 85.7% clean

Now we must reach:

```text
100% clean
0 legacy allowlisted files
all production cryptography goes through crates/pqc_crypto_module
```

---

## Hard requirement

At the end:

* no production file outside `crates/pqc_crypto_module/` imports raw crypto crates
* allowlist must be empty
* boundary tests must fail if any raw crypto appears outside the crypto module
* all existing tests must pass

---

## Forbidden production imports

Outside `crates/pqc_crypto_module/`, remove direct usage of:

```text
pqcrypto
sha2
sha3
ring
openssl
ed25519
ed25519_dalek
k256
p256
rsa
rand
rand_core
blake
```

---

## Required strategy

Do NOT refactor all 28 files at once.

Refactor in small batches and run tests after each batch.

Recommended order:

### Batch 1 — network layer

Files likely involved:

```text
src/network/gossip.rs
```

Goal:

* replace direct signing/verification/hash calls with `pqc_crypto_module::api`
* preserve existing message validation behavior
* keep PQC policy enforcement intact

Run:

```bash
cargo test --test crypto_boundary
cargo test --test byzantine_equivocation
cargo test --test crypto_dos_flood
```

---

### Batch 2 — endorsement layer

Files likely involved:

```text
src/endorsement/validator.rs
src/endorsement/types.rs
```

Goal:

* endorsement signature verification must use crypto module APIs
* algorithm consistency checks must remain intact
* no direct Ed25519/ML-DSA calls outside crypto module

Run:

```bash
cargo test --test crypto_boundary
cargo test --test pqc_security_audit
cargo test --test byzantine_equivocation
```

---

### Batch 3 — identity/signing layer

Files likely involved:

```text
src/identity/signing.rs
src/identity/pqc_policy.rs
src/identity/dual_signing.rs
```

Goal:

* key generation, sign, verify go through `pqc_crypto_module`
* legacy Ed25519 must either:

  * move inside crypto module as explicitly `non_approved_legacy`, or
  * be removed from production path
* PQC strict mode must still reject classical signatures

Run:

```bash
cargo test --test crypto_boundary
cargo test --test pqc_security_audit
cargo test --test slashing_penalty_lifecycle
```

---

### Batch 4 — hashing / block hashing

Files likely involved:

```text
src/crypto/hasher.rs
src/storage/traits.rs
src/models.rs
```

Goal:

* all SHA3/SHA256 implementation details move behind crypto module
* production hashing uses `pqc_crypto_module::api::sha3_256`
* if SHA256 remains for legacy block verification, isolate it inside crypto module under explicit legacy/non-approved API
* hash_algorithm metadata must remain preserved

Run:

```bash
cargo test --test crypto_boundary
cargo test --test persistent_crash_recovery
cargo test --test performance_guardrails
```

---

### Batch 5 — remaining allowlist

After batches 1–4:

1. inspect all remaining allowlisted files
2. migrate each one
3. remove them from allowlist
4. rerun boundary test

Run:

```bash
cargo test --test crypto_boundary
```

---

## Crypto module changes allowed

You may extend:

```text
crates/pqc_crypto_module/
```

with:

```rust
legacy_non_approved
```

ONLY if needed for historical verification or migration.

Rules:

* legacy APIs must be clearly named:

```rust
legacy_non_approved_ed25519_verify()
legacy_non_approved_sha256()
```

* they must NOT be available in approved mode APIs
* they must be documented as non-approved
* production strict PQC mode must not call them
* boundary test should allow raw crypto only inside `crates/pqc_crypto_module/`

---

## Boundary test final state

Update `tests/crypto_boundary.rs` so:

```rust
let legacy_allowlist: &[&str] = &[];
```

or remove allowlist entirely.

Required assertions:

```rust
assert_no_raw_crypto_imports_outside_crypto_module();
assert_no_forbidden_crypto_symbols_outside_crypto_module();
assert_all_crypto_public_calls_go_through_pqc_crypto_module();
assert_legacy_allowlist_is_empty();
```

---

## Important compatibility rule

Do not break existing behavior:

* old blocks with SHA256 must still verify if legacy support exists
* strict PQC mode must reject classical signatures
* dual signing must still work
* all Byzantine/slashing/DoS/persistence tests must pass

---

## Failure diagnostics

If a batch fails, report:

* file changed
* exact test failure
* whether failure is type-level, behavior-level, or boundary-level
* smallest safe rollback or fix
* next file to migrate

---

## Final required commands

At the end run:

```bash
cargo test --test crypto_boundary
cargo test --test pqc_security_audit
cargo test --test byzantine_equivocation
cargo test --test equivocation_persistence_partition
cargo test --test persistent_crash_recovery
cargo test --test crypto_dos_flood
cargo test --test slashing_penalty_lifecycle
cargo test --test performance_guardrails
cargo test
```

---

## Final output format

Report:

1. Initial legacy count
2. Final legacy count
3. Files migrated
4. Any APIs added to `pqc_crypto_module`
5. Whether SHA256/Ed25519 were moved to legacy non-approved APIs
6. Whether allowlist is empty
7. Boundary compliance percentage
8. Tests passing
9. Exact commands used
10. Clear final statement:

```text
Crypto boundary compliance: 100%.
All production cryptography now goes through pqc_crypto_module.
Prepared for future FIPS-oriented review, not certified.
```

Be careful. Prefer small safe migrations over one large risky rewrite.
