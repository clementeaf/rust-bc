You are a senior Rust distributed systems auditor.

Your task is to strengthen the existing `tests/chaos_network.rs` harness by adding a **deterministic convergence test after a long network partition**.

The current partition test only verifies that the network makes progress after healing.
That is not enough.

You must prove that after a long partition and healing:

* all honest nodes converge to the **exact same canonical chain**
* all honest nodes have the **exact same final state hash**
* no invalid PQC block entered the final state
* no fork remains accepted by honest nodes
* convergence is deterministic across repeated seeded runs

---

## Target file

Modify or extend:

`tests/chaos_network.rs`

---

## Scenario to implement

Add a new test named:

`long_partition_heals_to_identical_state_hash`

---

## Required setup

Use:

* 6 honest nodes
* PQC strict mode enabled:

  * `REQUIRE_PQC_SIGNATURES=true`
  * `TLS_PQC_KEM=true`
  * `DUAL_SIGN_VERIFY_MODE=both`
  * `HASH_ALGORITHM=sha3-256`
* deterministic seeded RNG

---

## Test flow

1. Start 6 honest nodes.
2. Let them produce and exchange valid blocks for a few rounds.
3. Split the network into two partitions:

   * Group A: nodes 0, 1, 2
   * Group B: nodes 3, 4, 5
4. Keep the partition active for a long period:

   * at least 25–50 rounds
5. Allow both partitions to produce valid blocks independently.
6. Heal the network.
7. Run additional sync rounds:

   * at least 25–50 rounds
8. Assert deterministic convergence.

---

## Mandatory assertions

At the end of the test, assert:

```rust
assert_all_honest_nodes_have_same_state_hash(&cluster);
assert_all_honest_nodes_have_same_canonical_tip(&cluster);
assert_no_invalid_pqc_blocks_in_any_honest_chain(&cluster);
assert_no_remaining_forks_in_honest_nodes(&cluster);
assert_chain_valid_under_strict_pqc_policy(&cluster);
```

If these helper functions do not exist, implement them.

---

## Determinism requirement

Run the same scenario with multiple fixed seeds:

```rust
let seeds = [1, 42, 1337, 9001, 123456789];
```

For each seed:

* run the full long-partition scenario
* collect final state hashes
* verify all honest nodes converge internally
* verify repeated runs with the same seed produce the same final state hash

---

## Important

Do NOT only assert that the chain height increased.
That is insufficient.

A valid result requires exact equality of:

* state hash
* canonical tip hash
* accepted block set or canonical chain path

---

## Failure diagnostics

If convergence fails, print:

* seed
* node id
* canonical tip
* chain height
* state hash
* fork count
* last 10 block hashes
* rejected block counts by reason

---

## Final output

After implementation, report:

1. New test added
2. Helper functions added
3. Whether deterministic convergence passed
4. Any fork or divergence found
5. Exact command used to run the test

Be strict. This test should fail if the protocol only makes progress but does not deterministically converge.
