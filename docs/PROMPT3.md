You are a senior Rust distributed systems reliability auditor.

Your task is to extend the existing chaos test suite to validate **crash recovery with real persistent storage**, not `MemoryStore`.

The current chaos tests validate crash/recovery behavior, but they use in-memory state.
That is insufficient for production-grade DLT guarantees.

You must prove that after a crash and restart:

* blocks are persisted correctly
* canonical tip is restored correctly
* PQC metadata is not lost
* hash algorithm metadata is not lost
* invalid history is not accepted after restart
* restarted node converges back to the honest cluster

---

## Target file

Modify or extend:

`tests/chaos_network.rs`

If needed, create a dedicated file:

`tests/persistent_crash_recovery.rs`

---

## New test name

Add:

`persistent_node_recovers_exact_state_after_crash`

---

## Required setup

Use:

* real persistent storage implementation if available
* temporary test directory
* 4 honest nodes
* strict PQC config:

```env
REQUIRE_PQC_SIGNATURES=true
TLS_PQC_KEM=true
DUAL_SIGN_VERIFY_MODE=both
HASH_ALGORITHM=sha3-256
```

Do NOT use `MemoryStore` for the crashed node.

---

## Test flow

1. Start 4 honest nodes.
2. At least one node must use real persistent storage.
3. Produce and sync valid PQC blocks for 10–20 rounds.
4. Capture for the persistent node before crash:

```rust
pre_crash_state_hash
pre_crash_canonical_tip
pre_crash_height
pre_crash_last_10_blocks
```

5. Fully shut down that node.
6. Drop all in-memory handles.
7. Recreate the node from the same persistent storage directory.
8. Assert immediately after restart:

```rust
assert_eq!(restored_state_hash, pre_crash_state_hash);
assert_eq!(restored_canonical_tip, pre_crash_canonical_tip);
assert_eq!(restored_height, pre_crash_height);
assert_eq!(restored_last_10_blocks, pre_crash_last_10_blocks);
```

9. Continue syncing with the cluster for 10–20 more rounds.
10. Assert the restarted node converges with all honest nodes.

---

## Mandatory assertions

Implement or reuse helpers:

```rust
assert_pqc_metadata_persisted(&node);
assert_hash_algorithm_metadata_persisted(&node);
assert_chain_valid_under_strict_pqc_policy(&cluster);
assert_all_honest_nodes_have_same_state_hash(&cluster);
assert_all_honest_nodes_have_same_canonical_tip(&cluster);
```

---

## Negative test

Add a second test:

`persistent_node_rejects_tampered_storage_after_restart`

Flow:

1. Create persistent node.
2. Produce valid PQC chain.
3. Shut down node.
4. Tamper with stored data manually:

   * corrupt signature bytes OR
   * change `signature_algorithm` from ML-DSA to Ed25519 OR
   * change `hash_algorithm` from Sha3_256 to Sha256
5. Restart node from same directory.
6. Assert one of these acceptable behaviors:

```rust
// preferred
restart fails with validation error

// acceptable
node starts but quarantines/rejects corrupted block and does not accept it into canonical chain
```

Failure condition:

```rust
panic!("Node accepted tampered persisted history");
```

---

## Determinism requirement

Run persistent crash recovery with seeds:

```rust
let seeds = [7, 99, 2024, 9001];
```

Each seed must produce stable recovery behavior.

---

## Failure diagnostics

On failure, print:

* seed
* storage path
* pre-crash state hash
* restored state hash
* pre-crash canonical tip
* restored canonical tip
* pre-crash height
* restored height
* last 10 block hashes before/after
* corrupted block id, if applicable
* validation error, if applicable

---

## Important rules

* Do NOT mock persistence.
* Do NOT use `MemoryStore` for the node being crash-tested.
* Do NOT only test that the node “starts”.
* Do NOT silently rebuild state from peers before asserting restored local state.
* First assertion must validate local restoration from disk.
* Then sync with peers.

---

## Final output format

Report:

1. Persistent storage implementation used
2. Tests added
3. Files modified
4. Whether exact restoration passed
5. Whether tampered storage was rejected
6. Any persistence bug found
7. Exact cargo test command used

Be strict. This should fail if storage loses PQC/hash metadata or rebuilds an invalid state.
