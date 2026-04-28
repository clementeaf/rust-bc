You are a senior Rust cryptographic boundary auditor.

Your task is to **enforce strict separation between Approved (FIPS-oriented) APIs and non-approved legacy APIs** inside `pqc_crypto_module`.

Currently:

* All crypto flows through `pqc_crypto_module`
* Legacy crypto is exposed via `pqc_crypto_module::legacy::*`
* Approved-mode state machine exists

Now we must guarantee:

👉 **Legacy APIs CANNOT be used when the module is in Approved mode**
👉 **Any accidental use must fail (compile-time if possible, otherwise runtime fail-closed)**

---

## 🎯 Objective

Ensure:

* Approved mode → ONLY approved APIs usable
* Legacy APIs:

  * explicitly marked non-approved
  * gated behind feature flags or runtime guards
  * cannot be used silently
* Any violation → panic in tests OR explicit error

---

## 📁 Target files

Modify:

```text
crates/pqc_crypto_module/src/
├── api.rs
├── approved_mode.rs
├── legacy.rs
├── errors.rs
```

Add tests:

```text
crates/pqc_crypto_module/tests/approved_vs_legacy.rs
```

---

## 🔴 Rule 1 — Approved mode must reject legacy APIs

Any call to:

```rust
pqc_crypto_module::legacy::*
```

must fail when:

```rust
ModuleState::Approved
```

---

## 🧠 Implementation approach

### Option A (preferred): runtime guard

Inside `legacy.rs`, wrap all functions:

```rust
fn ensure_not_approved() -> Result<(), CryptoError> {
    match get_module_state() {
        ModuleState::Approved => Err(CryptoError::NonApprovedAlgorithm),
        _ => Ok(())
    }
}
```

Then:

```rust
pub fn ed25519_verify(...) -> Result<(), CryptoError> {
    ensure_not_approved()?;
    ...
}
```

---

### Option B (stronger): feature gating

Add in `Cargo.toml`:

```toml
[features]
approved-only = []
```

Then:

```rust
#[cfg(feature = "approved-only")]
compile_error!("Legacy crypto is disabled in approved-only mode");
```

---

## 🔴 Rule 2 — Approved API must not expose legacy types

Ensure:

* `api.rs` does NOT return or accept any type from `legacy::*`
* No re-export of legacy symbols in approved API

---

## 🔴 Rule 3 — No implicit fallback

Ensure:

* ML-DSA failure does NOT fallback to Ed25519
* SHA3 failure does NOT fallback to SHA256

Add explicit checks:

```rust
assert!(matches!(error, CryptoError::NonApprovedAlgorithm));
```

---

## 🧪 Tests to implement

### 1. `legacy_api_fails_in_approved_mode`

```rust
#[test]
fn legacy_api_fails_in_approved_mode() {
    initialize_approved_mode().unwrap();

    let result = pqc_crypto_module::legacy::ed25519_verify(...);

    assert!(matches!(result, Err(CryptoError::NonApprovedAlgorithm)));
}
```

---

### 2. `legacy_api_works_before_approved_mode`

```rust
#[test]
fn legacy_api_works_before_approved_mode() {
    // module NOT initialized

    let result = pqc_crypto_module::legacy::ed25519_verify(...);

    assert!(result.is_ok());
}
```

---

### 3. `no_legacy_usage_in_approved_api`

Scan API surface:

```rust
#[test]
fn approved_api_does_not_expose_legacy() {
    // compile-time check: no legacy types in public API
}
```

(Use type assertions or manual inspection test)

---

### 4. `approved_mode_rejects_non_approved_hash`

```rust
#[test]
fn sha256_rejected_in_approved_mode() {
    initialize_approved_mode().unwrap();

    let result = pqc_crypto_module::legacy::sha256(...);

    assert!(matches!(result, Err(CryptoError::NonApprovedAlgorithm)));
}
```

---

### 5. `approved_mode_enforces_strict_crypto_only`

Test full flow:

```rust
#[test]
fn approved_mode_allows_only_mldsa_sha3_mlkem() {
    initialize_approved_mode().unwrap();

    assert!(sign_message(...).is_ok());
    assert!(sha3_256(...).is_ok());

    assert!(pqc_crypto_module::legacy::ed25519_sign(...).is_err());
}
```

---

## 🧪 Optional — compile-time enforcement

If feasible:

* create a test crate with `--features approved-only`
* ensure build fails if legacy module is used

---

## ⚠️ Important rules

* DO NOT silently allow legacy in approved mode
* DO NOT fallback automatically
* DO NOT mix approved + legacy in same execution path
* DO NOT expose legacy types in approved API

---

## 🧾 Documentation update

Update:

```text
SECURITY_POLICY_DRAFT.md
```

Add section:

```text
Non-Approved Algorithms

Legacy algorithms (Ed25519, SHA-256, etc.) are present only for backward compatibility.

When the module is in Approved mode:
- all non-approved algorithms are disabled
- any attempt to use them returns an error
- no fallback occurs

These algorithms are outside the approved cryptographic boundary.
```

---

## 🧾 Final output format

Report:

1. Whether legacy APIs are blocked in Approved mode
2. Whether approved APIs are clean of legacy types
3. Tests added
4. Files modified
5. Whether compile-time enforcement added (if any)
6. Whether fallback paths removed
7. Test results
8. Final statement:

```text
Approved vs Non-Approved crypto separation enforced.
Legacy crypto cannot be used in Approved mode.
Prepared for FIPS-oriented boundary enforcement.
```

---

## 🧠 Mindset

This is the final crypto boundary hardening step.

Goal:

👉 Make it **impossible** to accidentally use non-approved crypto in Approved mode.
