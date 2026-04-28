You are a senior Rust cryptographic module architect with experience preparing software for FIPS 140-3 validation.

Your task is to refactor the existing Rust DLT cryptography into a separate, auditable, FIPS-oriented crypto module.

Important: the goal is NOT to claim FIPS certification now.
The goal is to prepare the architecture, boundaries, APIs, self-tests, and documentation structure so the module can later be reviewed by an accredited lab.

---

## Objective

Create a standalone Rust crate:

```text
crates/pqc_crypto_module/
```

This crate must isolate all cryptographic operations used by the DLT:

* ML-DSA signing and verification
* ML-KEM encapsulation/decapsulation
* SHA3-256 hashing
* secure RNG
* key zeroization
* startup self-tests
* strict approved-mode enforcement

The DLT must call cryptography only through this module.

---

## Critical rule

Do NOT claim:

* “FIPS certified”
* “NIST certified”
* “FIPS compliant”

Use only:

```text
FIPS-oriented
FIPS-ready architecture
aligned with FIPS 203/204/202
prepared for future validation
```

---

## Target structure

Create:

```text
crates/pqc_crypto_module/
├── Cargo.toml
├── README.md
├── SECURITY_POLICY_DRAFT.md
├── src/
│   ├── lib.rs
│   ├── api.rs
│   ├── approved_mode.rs
│   ├── errors.rs
│   ├── hashing.rs
│   ├── mldsa.rs
│   ├── mlkem.rs
│   ├── rng.rs
│   ├── self_tests.rs
│   ├── zeroize.rs
│   └── types.rs
└── tests/
    ├── self_tests.rs
    ├── api_boundary.rs
    ├── key_zeroization.rs
    └── no_fallback.rs
```

---

## Module boundary

All cryptographic operations must go through public APIs in:

```rust
api.rs
```

Expose only high-level safe functions:

```rust
pub fn initialize_approved_mode() -> Result<(), CryptoError>;

pub fn sign_message(
    private_key: &MldsaPrivateKey,
    message: &[u8],
) -> Result<MldsaSignature, CryptoError>;

pub fn verify_signature(
    public_key: &MldsaPublicKey,
    message: &[u8],
    signature: &MldsaSignature,
) -> Result<(), CryptoError>;

pub fn sha3_256(data: &[u8]) -> Result<Hash256, CryptoError>;

pub fn mlkem_encapsulate(
    public_key: &MlKemPublicKey,
) -> Result<(MlKemCiphertext, MlKemSharedSecret), CryptoError>;

pub fn mlkem_decapsulate(
    private_key: &MlKemPrivateKey,
    ciphertext: &MlKemCiphertext,
) -> Result<MlKemSharedSecret, CryptoError>;
```

No DLT code should directly call `pqcrypto`, `sha3`, `rand`, `ed25519`, or any raw crypto crate.

---

## Approved mode

Implement an approved-mode guard.

Required behavior:

* module starts in `Uninitialized`
* `initialize_approved_mode()` runs all self-tests
* if self-tests pass → `Approved`
* if any self-test fails → `Error`
* all crypto APIs must reject calls unless state is `Approved`

Example states:

```rust
pub enum ModuleState {
    Uninitialized,
    SelfTesting,
    Approved,
    Error,
}
```

Add tests proving:

* crypto call before initialization fails
* failed self-test locks module in error state
* no fallback to classical algorithms is possible

---

## Self-tests

Implement startup Known Answer Tests or deterministic self-tests for:

1. ML-DSA sign/verify
2. ML-KEM encaps/decaps
3. SHA3-256
4. RNG sanity / continuous random number generator test

Behavior:

```rust
initialize_approved_mode()
    -> run_self_tests()
    -> Approved or Error
```

If any self-test fails, all crypto operations must fail closed.

---

## RNG requirements

Implement RNG wrapper:

```rust
pub struct ApprovedRng;
```

Requirements:

* use OS-backed randomness
* reject repeated outputs in continuous RNG test
* never silently fallback to insecure RNG
* return explicit error on RNG failure

Add tests for:

* RNG produces non-empty output
* repeated output detection works via test hook
* RNG unavailable/failure path fails closed

---

## Key zeroization

Use `zeroize`.

Private key and shared secret types must zeroize on drop:

```rust
MldsaPrivateKey
MlKemPrivateKey
MlKemSharedSecret
```

Add tests or structural assertions proving zeroize is derived/implemented.

---

## Error handling

Create:

```rust
pub enum CryptoError {
    ModuleNotInitialized,
    ModuleInErrorState,
    SelfTestFailed(String),
    InvalidKey,
    InvalidSignature,
    VerificationFailed,
    RngFailure,
    NonApprovedAlgorithm,
    SerializationError,
}
```

No panic for normal crypto failure.

---

## No fallback / no downgrade

The module must not expose:

* Ed25519
* ECDSA
* RSA
* SHA1
* MD5
* non-approved test algorithms

If legacy algorithms still exist elsewhere, isolate them outside this module and mark them non-approved.

Add tests:

```rust
no_ed25519_available_in_approved_api
no_sha256_available_in_approved_api
no_classical_fallback_on_mldsa_failure
```

---

## Integration with DLT

Refactor DLT code so:

* consensus signing uses `pqc_crypto_module::sign_message`
* consensus verification uses `pqc_crypto_module::verify_signature`
* hashing uses `pqc_crypto_module::sha3_256`
* TLS/KEM code uses the module boundary where possible
* old direct crypto calls are removed or explicitly marked legacy/non-approved

Add a check/test that searches the source tree or enforces architectural boundary:

```text
DLT production code must not directly import raw crypto crates.
```

---

## Documentation

Create `SECURITY_POLICY_DRAFT.md` with:

1. Module name
2. Version
3. Cryptographic boundary
4. Approved algorithms
5. Non-approved algorithms excluded
6. Roles and services
7. Key lifecycle
8. Self-test behavior
9. Error state behavior
10. Zeroization behavior
11. Future validation notes

Create `README.md` explaining:

* this is FIPS-oriented, not certified
* how to initialize approved mode
* how DLT should call crypto APIs
* how to run tests

---

## Tests to run

Add or update tests so the following pass:

```bash
cargo test -p pqc_crypto_module
cargo test
```

Optional architectural guard:

```bash
cargo test --test crypto_boundary
```

---

## Final output format

Report:

1. Crate created
2. Public approved APIs exposed
3. Self-tests implemented
4. Approved-mode lifecycle implemented
5. RNG wrapper implemented
6. Zeroization implemented
7. Documentation files created
8. DLT integration changes made
9. Boundary violations found/fixed
10. Tests passing
11. Clear statement: “Prepared for future FIPS-oriented review, not certified.”

Be strict. The goal is to make the cryptographic boundary clean enough that a future FIPS 140-3 lab review is realistic.
