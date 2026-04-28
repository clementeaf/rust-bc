You are a senior cryptographic compliance engineer preparing a Rust module for **FIPS 140-3 validation via CMVP**.

The system already has:

* A clean crypto boundary (`pqc_crypto_module`)
* Approved-mode enforcement (runtime + optional compile-time)
* ML-DSA / ML-KEM / SHA3 isolation
* No fallback to non-approved algorithms
* Full test coverage (security, chaos, DoS, Byzantine, persistence)
* A draft `SECURITY_POLICY_DRAFT.md`

Now your task is to move from:

👉 **“audit-ready architecture”**
to
👉 **“pre-CMVP submission readiness”**

This does NOT mean certification yet.
It means the module is structured so a FIPS lab can evaluate it.

---

# 🎯 Objective

Produce all artifacts and structural guarantees required before engaging a FIPS lab and CMVP process.

---

# 📁 Target outputs

Create or complete:

```text
crates/pqc_crypto_module/
├── SECURITY_POLICY.md                (finalized, not draft)
├── DESIGN_DOCUMENT.md
├── FINITE_STATE_MODEL.md
├── KEY_MANAGEMENT.md
├── OPERATIONAL_GUIDANCE.md
├── SELF_TEST_DOCUMENTATION.md
├── NON_APPROVED_USAGE.md
└── build/
    ├── reproducible_build.md
    └── module_boundary_definition.md
```

---

# 🧾 Step 1 — Finalize Security Policy

Upgrade:

```text
SECURITY_POLICY_DRAFT.md → SECURITY_POLICY.md
```

Must include:

1. Module name and version
2. Cryptographic boundary definition
3. Approved algorithms:

   * ML-DSA (FIPS 204 aligned)
   * ML-KEM (FIPS 203 aligned)
   * SHA3-256 (FIPS 202)
4. Non-approved algorithms (legacy section)
5. Roles:

   * User
   * Crypto Officer
6. Services:

   * sign, verify, encapsulate, decapsulate, hash
7. Approved mode definition
8. Self-test behavior
9. Error state behavior (fail-closed)
10. Key lifecycle summary
11. Zeroization behavior
12. Physical/logical assumptions (software-only module)
13. Statement:

```text
This module is prepared for FIPS 140-3 evaluation and is not currently validated.
```

---

# 🧠 Step 2 — Design Document

Create:

```text
DESIGN_DOCUMENT.md
```

Include:

* Architecture diagram (textual)
* Module boundaries (what is inside vs outside)
* API entry points
* Internal components:

  * mldsa.rs
  * mlkem.rs
  * hashing.rs
  * rng.rs
  * approved_mode.rs
* Data flow:

  * input → validation → crypto → output
* No external crypto calls allowed outside module

---

# 🔄 Step 3 — Finite State Model

Create:

```text
FINITE_STATE_MODEL.md
```

Define:

```text
Uninitialized
→ SelfTesting
→ Approved
→ Error
```

Include:

* allowed transitions
* forbidden transitions
* behavior in Error (must fail closed)
* requirement that crypto operations only allowed in Approved

---

# 🔑 Step 4 — Key Management Document

Create:

```text
KEY_MANAGEMENT.md
```

Include:

* key generation (ML-DSA / ML-KEM)
* key storage (in-memory only unless explicitly extended)
* key usage
* key destruction (Zeroize)
* no persistent private key storage unless explicitly defined
* shared secret handling

---

# 🧪 Step 5 — Self-Test Documentation

Create:

```text
SELF_TEST_DOCUMENTATION.md
```

Describe:

* Known Answer Tests (KAT)
* what is tested:

  * ML-DSA sign/verify
  * ML-KEM encaps/decaps
  * SHA3 hashing
  * RNG sanity
* when tests run:

  * during `initialize_approved_mode()`
* failure behavior:

  * module enters Error state
  * no crypto allowed

---

# 🚫 Step 6 — Non-Approved Usage Document

Create:

```text
NON_APPROVED_USAGE.md
```

Include:

* list of legacy algorithms:

  * Ed25519
  * SHA-256
  * HMAC-SHA256
* statement:

```text
These are outside the approved boundary and disabled in Approved mode.
```

* how they are gated:

  * runtime guard
  * feature flag (`approved-only`)

---

# ⚙️ Step 7 — Reproducible Build

Create:

```text
build/reproducible_build.md
```

Include:

* Rust version
* Cargo.lock usage
* deterministic builds
* dependency pinning
* how to rebuild identical binary

---

# 🧱 Step 8 — Module Boundary Definition

Create:

```text
build/module_boundary_definition.md
```

Define:

* what files are inside crypto boundary
* what is outside
* how boundary is enforced:

  * code structure
  * boundary tests
  * dependency control

---

# 🧪 Step 9 — Add compliance tests

Add:

```text
tests/fips_readiness.rs
```

Tests:

* module cannot operate before Approved mode
* module fails after self-test failure
* legacy APIs fail in Approved mode
* no approved API exposes legacy types
* module state transitions valid

---

# ⚠️ Step 10 — Hard rules

* No silent fallback
* No panic-based crypto failure
* All errors explicit
* Approved mode strictly enforced
* Documentation must match implementation exactly

---

# 🧾 Final output format

Report:

1. Documents created
2. Missing sections (if any)
3. Whether module boundary is fully defined
4. Whether Approved mode behavior is documented and enforced
5. Whether all required artifacts exist for lab review
6. Any remaining gap before contacting lab
7. Final statement:

```text
The pqc_crypto_module is prepared for pre-CMVP review and ready to be submitted to a FIPS 140-3 accredited laboratory for evaluation.
```

---

## 🧠 Mindset

You are not coding features.

You are preparing a **cryptographic module for formal validation**.

Clarity, determinism, and auditability are more important than performance.
