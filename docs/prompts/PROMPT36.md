You are a senior Rust cryptography and distributed systems engineer.

The current blockchain demo is now E2E functional with:

* transaction submit
* tx_id query
* committed state
* non-zero Ed25519 orderer_signature
* smart contract install
* kv_store set/get working
* world state returning hex value correctly

However, the validated demo used Ed25519.

Your task is to validate and harden the PQC path using ML-DSA-65.

## OBJECTIVE

Make the E2E demo run successfully with:

SIGNING_ALGORITHM=ml-dsa-65

The final output must prove:

1. transaction is signed with ML-DSA-65
2. signature is non-zero
3. signature length matches expected ML-DSA-65 signature size
4. block verifies successfully
5. tx is committed
6. tx can be queried by tx_id
7. smart contract set/get still works
8. no fallback to Ed25519 happens silently

## STRICT RULES

* Do not redesign the system.
* Do not replace the existing signing abstraction.
* Do not mock ML-DSA-65.
* Do not silently fallback to Ed25519.
* Fail loudly if ML-DSA-65 is unavailable.
* Keep changes minimal.
* Add explicit logs proving which signing algorithm is active.

## TASKS

### STEP 1 — Algorithm selection audit

Find where SIGNING_ALGORITHM is read.
Ensure accepted values are explicit:

* ed25519
* ml-dsa-65

Any unknown value must return an error.

### STEP 2 — ML-DSA-65 signer wiring

Ensure the orderer signer actually uses ML-DSA-65 when SIGNING_ALGORITHM=ml-dsa-65.

Add logs:

* active signing algorithm
* public key length
* signature length

### STEP 3 — Verification

Ensure block verification uses the matching algorithm.
Do not verify ML-DSA-65 blocks with Ed25519 verifier.

### STEP 4 — E2E test

Add or update an E2E test that runs with:

SIGNING_ALGORITHM=ml-dsa-65

It must submit:

* tx_id: demo-tx-pqc
* contract: kv_store
* operation: set("pqc", "hello")

Then query:

* GET /tx/demo-tx-pqc
* get("pqc")

### STEP 5 — Output proof

Print final demo evidence:

* algorithm: ml-dsa-65
* tx_id
* block_height
* committed: true
* signature_len
* signature_non_zero: true
* verification_valid: true
* contract_get_result: 68656c6c6f

## OUTPUT FORMAT

Return:

1. Files changed
2. Code diff summary
3. Commands to run
4. Expected output
5. Any failing issue with exact cause

Start by auditing the signing path.
