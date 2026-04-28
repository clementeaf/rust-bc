You are a senior Rust security engineer specialized in cryptographic DoS resistance.

Your task is to extend the DLT test suite with adversarial **invalid-message flood / cryptographic DoS tests**.

The system already has:

* PQC enforcement
* ML-DSA signatures
* ML-KEM TLS
* SHA3-256 support
* chaos network tests
* persistent crash recovery tests

Now we need to prove that malicious peers cannot saturate the node by forcing expensive PQC verification on unlimited invalid messages.

---

## Target file

Create:

`tests/crypto_dos_flood.rs`

or extend:

`tests/chaos_network.rs`

Prefer a dedicated file if cleaner.

---

## Main objective

Verify that under a large flood of invalid blocks/messages:

* honest nodes remain responsive
* invalid messages are rejected
* valid blocks still get accepted
* CPU-expensive verification is bounded
* repeated invalid payloads are cached/rate-limited/dropped early
* one malicious peer cannot monopolize validation resources

---

## Required production config

Use strict PQC mode:

```env
REQUIRE_PQC_SIGNATURES=true
TLS_PQC_KEM=true
DUAL_SIGN_VERIFY_MODE=both
HASH_ALGORITHM=sha3-256
SIGNING_ALGORITHM=ml-dsa-65
```

---

## Tests to add

### 1. `invalid_pqc_signature_flood_does_not_halt_valid_progress`

Flow:

1. Start 4 honest nodes.
2. Start 2 malicious peers.
3. Malicious peers send at least 10,000 invalid PQC-signed blocks/messages.
4. Honest nodes simultaneously produce valid blocks.
5. Assert:

```rust
assert_valid_blocks_accepted_during_flood(&cluster);
assert_no_invalid_blocks_accepted(&cluster);
assert_honest_nodes_remain_responsive(&cluster);
assert_rejection_count_at_least(&cluster, 10_000);
```

Failure condition:

```rust
panic!("Crypto DoS: valid progress halted under invalid PQC flood");
```

---

### 2. `duplicate_invalid_signature_is_dropped_before_reverification`

Purpose:

A repeated invalid block should not trigger expensive ML-DSA verification every time.

Flow:

1. Create one invalid PQC block.
2. Send the exact same invalid block 5,000 times.
3. Instrument verification count.
4. Assert:

```rust
assert!(pqc_verify_calls <= EXPECTED_SMALL_BOUND);
```

Suggested bound:

```rust
const EXPECTED_SMALL_BOUND: usize = 5;
```

If the system currently does not track verification count, add a test-only instrumentation layer.

---

### 3. `malicious_peer_is_rate_limited_or_quarantined`

Flow:

1. One peer sends invalid messages continuously.
2. After threshold is reached, node should:

   * rate-limit the peer OR
   * quarantine the peer OR
   * drop further messages before crypto verification

Assert:

```rust
assert_peer_penalized(malicious_peer_id);
assert_subsequent_messages_dropped_early(malicious_peer_id);
```

If peer scoring does not exist, implement minimal test-visible peer penalty mechanism.

---

### 4. `mixed_valid_and_invalid_load_preserves_fairness`

Flow:

1. 3 honest peers send valid blocks.
2. 2 malicious peers send invalid blocks at high rate.
3. Run for multiple rounds.
4. Assert:

```rust
assert_all_honest_peers_made_progress(&cluster);
assert_valid_block_latency_below_threshold(&cluster);
assert_invalid_blocks_never_enter_canonical_chain(&cluster);
```

Use relaxed thresholds to avoid flaky tests.

---

## Early rejection requirements

Before expensive PQC verification, the node should reject or deprioritize messages when possible using cheap checks:

* malformed structure
* impossible signature length
* algorithm tag mismatch
* duplicate block hash
* stale height / replay
* peer already rate-limited

Add tests proving cheap rejection happens before ML-DSA verification.

---

## Required helpers/instrumentation

Add test-only counters if missing:

```rust
pqc_verify_call_count()
cheap_rejection_count()
rate_limited_peer_count()
accepted_valid_block_count()
invalid_block_acceptance_count()
```

Use `#[cfg(test)]` or a test feature flag.

---

## Performance guardrails

These tests should not be overly flaky.

Use:

* deterministic seeded RNG
* relaxed time thresholds
* counters over wall-clock timing where possible
* `tokio::time::timeout` only as a final guard

Suggested timeout:

```rust
tokio::time::timeout(Duration::from_secs(10), flood_future)
```

---

## Mandatory assertions

At minimum:

```rust
assert_eq!(invalid_block_acceptance_count(), 0);
assert!(accepted_valid_block_count() > 0);
assert!(cheap_rejection_count() > 0);
assert!(pqc_verify_call_count() < total_invalid_messages);
assert_all_honest_nodes_have_same_state_hash(&cluster);
```

---

## Final output format

Report:

1. Tests added
2. Files modified
3. Whether invalid flood was rejected
4. Whether valid progress continued
5. Whether duplicate invalid messages avoided repeated PQC verification
6. Whether peer rate-limiting/quarantine exists
7. Any DoS vulnerability found
8. Exact cargo test command used

Be adversarial. The test should fail if invalid messages can force unbounded ML-DSA verification.
