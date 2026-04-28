You are a senior Rust blockchain consensus and Byzantine fault-tolerance auditor.

Your task is to close the last two elite gaps in the DLT equivocation system:

1. Persistent penalty/slashing state after restart
2. Retroactive equivocation detection after network partition healing

The system already has:

* PQC strict validation
* ML-DSA signatures
* equivocation proof construction
* gossip deduplication
* validator penalty/quarantine
* long partition deterministic convergence
* RocksDB persistence tests

---

## Target files

Prefer adding a dedicated file:

`tests/equivocation_persistence_partition.rs`

Modify implementation files only if needed:

* `src/consensus/equivocation.rs`
* persistent storage modules
* gossip/consensus integration

---

# Part 1 — Persistent penalty state

## Test name

`equivocation_penalty_survives_restart`

## Objective

If a validator equivocates and gets penalized, that penalty must survive node restart.

A validator must not be able to bypass quarantine/slashing by restarting.

## Flow

1. Start node with real persistent storage, preferably RocksDB.
2. Enable strict PQC config:

```env
REQUIRE_PQC_SIGNATURES=true
TLS_PQC_KEM=true
DUAL_SIGN_VERIFY_MODE=both
HASH_ALGORITHM=sha3-256
SIGNING_ALGORITHM=ml-dsa-65
```

3. Create validator/proposer with ML-DSA key.
4. Create two different valid blocks for the same consensus position:

```rust
(position = height + slot + proposer_id)
```

5. Sign both blocks correctly.
6. Submit both to node.
7. Assert equivocation proof is created.
8. Assert proposer is penalized:

```rust
assert!(node.equivocation_detector().is_penalized(proposer_id));
```

9. Fully shut down node.
10. Drop in-memory handles.
11. Restart node from the same storage directory.
12. Assert immediately after restart:

```rust
assert!(restored_node.equivocation_detector().is_penalized(proposer_id));
assert_eq!(
    restored_node.equivocation_detector().proof_count_for(proposer_id),
    1
);
```

13. Have the penalized validator submit a new future valid block.
14. Assert it is rejected.

Failure condition:

```rust
panic!("Equivocation penalty was lost after restart");
```

---

# Part 2 — Cross-partition retroactive equivocation detection

## Test name

`equivocation_across_partition_detected_after_healing`

## Objective

If a Byzantine validator sends block A to partition 1 and conflicting block B to partition 2, equivocation may not be detectable during the partition.

It must be detected after the partition heals.

## Flow

1. Start 6 honest nodes + 1 Byzantine proposer.
2. Split honest nodes:

```text
Group A: nodes 0, 1, 2
Group B: nodes 3, 4, 5
```

3. Byzantine proposer creates:

```text
block_A: valid ML-DSA block at position P
block_B: different valid ML-DSA block at same position P
```

4. Send `block_A` only to Group A.
5. Send `block_B` only to Group B.
6. During partition, assert:

```rust
assert_group_has_seen_only_block(group_a, block_A);
assert_group_has_seen_only_block(group_b, block_B);
assert_no_equivocation_detected_yet_across_groups();
```

7. Heal the partition.
8. Run sync/gossip rounds.
9. Assert all honest nodes eventually detect equivocation:

```rust
assert_all_honest_nodes_have_equivocation_proof(proposer_id, position);
assert_all_honest_nodes_penalized(proposer_id);
assert_no_remaining_forks_in_honest_nodes(&cluster);
assert_chain_valid_under_strict_pqc_policy(&cluster);
```

10. Assert deterministic convergence:

```rust
assert_all_honest_nodes_have_same_state_hash(&cluster);
assert_all_honest_nodes_have_same_canonical_tip(&cluster);
```

---

## Critical requirements

* Do NOT use invalid signatures.
* Both conflicting blocks must be individually valid.
* Detection must be based on conflict of same proposer + same position + different hash.
* The equivocation proof must be created after healing, not only during direct submission.
* Proof must propagate to all honest nodes.
* Penalization must become consistent across all honest nodes.
* Canonical chain must not retain both conflicting blocks.
* Restart must not erase penalty/proof state.

---

## Storage requirement

If equivocation proofs/penalties are currently in-memory only, implement persistence.

Suggested persisted structures:

```rust
EquivocationProofRecord {
    proposer_id,
    position,
    block_hash_a,
    block_hash_b,
    signature_a,
    signature_b,
    algorithm,
    detected_at_height,
}

PenalizedValidatorRecord {
    proposer_id,
    reason,
    proof_hash,
    penalty_start_height,
    penalty_until_height,
}
```

Use existing RocksDB/storage patterns if available.

---

## Mandatory negative tests

Add these if not already covered:

### `restart_does_not_create_false_penalty`

1. Persist normal non-equivocating chain.
2. Restart node.
3. Assert no validators are penalized.

### `cross_partition_same_block_duplicate_not_equivocation`

1. Same valid block appears in both partitions.
2. Heal network.
3. Assert no equivocation proof is created.

---

## Diagnostics on failure

Print:

* proposer_id
* position
* block_hash_A
* block_hash_B
* which nodes saw which block
* proof count by node
* penalty state by node
* canonical tip by node
* state hash by node
* storage path for persistent test
* whether proof was loaded from disk or memory

---

## Final output format

Report:

1. Tests added
2. Files modified
3. Whether penalty survived restart
4. Whether future blocks from penalized validator were rejected
5. Whether cross-partition equivocation was detected after healing
6. Whether proof propagated to all honest nodes
7. Whether honest nodes converged after healing
8. Any bug found
9. Exact cargo test command used

Be strict. These tests should fail if equivocation penalties are memory-only or if cross-partition equivocation can survive healing undetected.
