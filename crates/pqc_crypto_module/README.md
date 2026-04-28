# pqc_crypto_module

FIPS-oriented post-quantum cryptographic module for Cerulean Ledger DLT.

> **Important**: This module is FIPS-oriented and architecturally aligned with FIPS 202/203/204. It is **NOT FIPS certified**. It is prepared for future validation by an accredited lab.

## Approved algorithms

| Algorithm | Standard | Purpose |
|---|---|---|
| ML-DSA-65 | FIPS 204 | Digital signatures |
| ML-KEM-768 | FIPS 203 | Key encapsulation (placeholder) |
| SHA3-256 | FIPS 202 | Hashing |

## Non-approved algorithms (excluded)

Ed25519, ECDSA, RSA, SHA-1, SHA-256, MD5 — none of these are available through this module's API.

## Usage

```rust
use pqc_crypto_module::api;

// Must be called once at startup — runs self-tests
api::initialize_approved_mode().expect("self-tests failed");

// Sign and verify
let kp = api::generate_mldsa_keypair().unwrap();
let sig = api::sign_message(&kp.private_key, b"hello").unwrap();
api::verify_signature(&kp.public_key, b"hello", &sig).unwrap();

// Hash
let hash = api::sha3_256(b"data").unwrap();
```

## Module lifecycle

1. `Uninitialized` — module loaded, no crypto available
2. `SelfTesting` — running Known Answer Tests
3. `Approved` — all self-tests passed, crypto operations available
4. `Error` — self-test failure, all operations rejected

All API calls before `initialize_approved_mode()` return `CryptoError::ModuleNotInitialized`.

## Running tests

```bash
cargo test -p pqc_crypto_module
```
