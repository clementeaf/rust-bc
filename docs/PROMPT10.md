You are a senior Rust architecture auditor focused on enforcing strict cryptographic boundaries.

Your task is to **enforce that ALL cryptography in the DLT goes exclusively through the `pqc_crypto_module` crate**.

This is a hard requirement for future FIPS-oriented validation.

---

## 🎯 Objective

Guarantee that:

* NO production code imports raw crypto crates directly
* ALL cryptographic operations go through:

```rust
pqc_crypto_module::api
```

* Any violation MUST fail the build (via tests or compile-time checks)

---

## 🔴 Forbidden direct dependencies (production code)

The following must NOT appear in any DLT module outside `pqc_crypto_module`:

```text
pqcrypto
sha2
sha3
ring
openssl
ed25519
k256
p256
rsa
rand (except inside crypto module)
blake
```

---

## 📁 Target scope

Scan ALL production code:

```text
src/
consensus/
network/
storage/
transaction/
identity/
```

Exclude:

```text
crates/pqc_crypto_module/
tests/
benches/
```

---

## 🧪 Step 1 — Create boundary enforcement test

Create:

```text
tests/crypto_boundary.rs
```

---

## 🧠 Test logic

Recursively scan `.rs` files and detect forbidden imports.

Example implementation:

```rust
#[test]
fn no_raw_crypto_imports_outside_crypto_module() {
    let forbidden = [
        "pqcrypto",
        "sha2",
        "sha3",
        "ring",
        "openssl",
        "ed25519",
        "k256",
        "p256",
        "rsa",
        "rand::",
        "blake",
    ];

    let root = std::path::Path::new("src");

    fn scan_dir(path: &std::path::Path, forbidden: &[&str]) {
        for entry in std::fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();

            if path.is_dir() {
                scan_dir(&path, forbidden);
            } else if path.extension().map(|e| e == "rs").unwrap_or(false) {
                let content = std::fs::read_to_string(&path).unwrap();

                for f in forbidden {
                    if content.contains(f) {
                        panic!(
                            "Forbidden crypto import '{}' found in file: {:?}",
                            f, path
                        );
                    }
                }
            }
        }
    }

    scan_dir(root, &forbidden);
}
```

---

## 🧪 Step 2 — Enforce usage of pqc_crypto_module

Add a second test:

```rust
#[test]
fn crypto_must_go_through_pqc_crypto_module() {
    let root = std::path::Path::new("src");

    fn scan(path: &std::path::Path) {
        for entry in std::fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();

            if path.is_dir() {
                scan(&path);
            } else if path.extension().map(|e| e == "rs").unwrap_or(false) {
                let content = std::fs::read_to_string(&path).unwrap();

                if content.contains("sign(")
                    || content.contains("verify(")
                    || content.contains("sha")
                    || content.contains("hash")
                {
                    if !content.contains("pqc_crypto_module") {
                        panic!(
                            "Possible direct crypto usage without pqc_crypto_module in file: {:?}",
                            path
                        );
                    }
                }
            }
        }
    }

    scan(root);
}
```

---

## ⚙️ Step 3 — Refactor violations

If violations are found:

* Replace direct crypto usage with:

```rust
use pqc_crypto_module::api::*;
```

Examples:

### Before ❌

```rust
use sha3::Sha3_256;
use pqcrypto_mldsa::sign;
```

### After ✔️

```rust
use pqc_crypto_module::api::{sha3_256, sign_message};
```

---

## 🧪 Step 4 — Optional compile-time guard (advanced)

If possible, restrict Cargo dependencies:

* remove raw crypto crates from main `Cargo.toml`
* only allow them inside `pqc_crypto_module`

Or enforce via feature flags:

```toml
[features]
no-raw-crypto = []
```

And gate builds.

---

## 🧾 Step 5 — Add documentation

Update root README:

```text
All cryptographic operations MUST go through pqc_crypto_module.

Direct use of cryptographic libraries in production code is forbidden and enforced by automated tests.
```

---

## 🧪 Step 6 — Run tests

```bash
cargo test --test crypto_boundary
cargo test
```

---

## ⚠️ Important rules

* DO NOT allow exceptions for convenience
* DO NOT whitelist files unless strictly necessary
* DO NOT allow test-only shortcuts to leak into production code
* DO NOT allow fallback to classical crypto outside module

---

## 🧾 Final output format

Report:

1. Boundary test file created
2. Violations found (if any)
3. Files refactored
4. Whether all crypto now goes through pqc_crypto_module
5. Whether forbidden imports are eliminated
6. Tests passing
7. Exact cargo commands used

---

## 🧠 Mindset

This is not a style rule.

This is a **security boundary enforcement** required for:

* FIPS-oriented design
* auditability
* crypto agility
* preventing silent downgrade paths

Fail the build if violated.
