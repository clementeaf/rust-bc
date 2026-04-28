You are a senior Rust blockchain consensus auditor.

Your task is to extend the DLT test suite with **Byzantine equivocation detection**.

The system already rejects invalid PQC signatures, downgrade attempts, replay, malformed blocks, corrupted storage, and Crypto-DoS floods.

Now we must test a harder case:

A malicious validator signs two different valid PQC blocks for the same height/slot/round.

Both signatures are valid.
Both blocks may be structurally valid.
The attack is not cryptographic forgery — it is **equivocation**.

---

## Objective

Prove that the DLT detects and penalizes a validator that produces conflicting valid blocks for the same consensus position.

A consensus position should include whichever fields the protocol uses, for example:

```rust
(height, round, slot, epoch, proposer_id)
```

Use the actual fields available in the codebase.

---

## Target file

Create:

`tests/byzantine_equivocation.rs`

or extend:

`tests/chaos_network.rs`

Prefer a dedicated file if cleaner.

---

## Required strict config

```env
REQUIRE_PQC_SIGNATURES=true
TLS_PQC_KEM=true
DUAL_SIGN_VERIFY_MODE=both
HASH_ALGORITHM=sha3-256
SIGNING_ALGORITHM=ml-dsa-65
```

---

## Tests to add

### 1. `detects_two_valid_blocks_same_height_same_proposer`

Flow:

1. Create one validator/proposer with a valid ML-DSA key.
2. Produce block A at height H.
3. Produce block B at the same height H from the same proposer, but with different:

   * transactions OR
   * previous hash OR
   * timestamp/nonce
4. Sign both blocks correctly with ML-DSA.
5. Submit both to an honest node.
6. Assert:

```rust
assert_eqivocation_detected(proposer_id, height_or_slot);
assert_only_one_block_accepted_for_position(height_or_slot, proposer_id);
assert_validator_penalized_or_quarantined(proposer_id);
assert_no_fork_created_from_equivocation();
```

---

### 2. `equivocation_proof_is_constructed_from_two_valid_signatures`

Flow:

1. Create two conflicting blocks.
2. Verify both signatures are individually valid.
3. Submit both.
4. Assert the node creates an equivocation proof containing:

```rust
proposer_id
position
block_hash_a
block_hash_b
signature_a
signature_b
```

5. Assert proof validation succeeds.

---

### 3. `equivocation_proof_survives_gossip_and_is_deduplicated`

Flow:

1. Node A detects equivocation.
2. Node A gossips equivocation proof.
3. Nodes B/C receive the proof.
4. Assert:

   * proof accepted once
   * duplicate proof ignored
   * malicious proposer penalized consistently across honest nodes

---

### 4. `equivocating_validator_cannot_produce_future_blocks_until_penalty_expires`

Flow:

1. Validator equivocates.
2. Validator tries to produce a future valid block at height H+1.
3. Honest nodes reject it while penalty/quarantine is active.

If the protocol does not yet have slashing/quarantine:

* implement minimal test-visible penalty state
* or mark as critical missing guarantee

---

### 5. `different_proposers_same_height_is_not_equivocation`

Flow:

1. Proposer A creates valid block at height H.
2. Proposer B creates valid block at same height H.
3. Both are valid according to fork-choice rules.
4. Assert this is NOT treated as equivocation.

---

### 6. `same_proposer_same_block_duplicate_is_not_equivocation`

Flow:

1. Same proposer sends same block twice.
2. Assert:

   * duplicate is deduplicated
   * no equivocation proof created
   * proposer is not penalized

---

## Required implementation behavior

The system should maintain an index similar to:

```rust
seen_proposals: HashMap<(ProposerId, ConsensusPosition), BlockHash>
```

When a new valid proposal arrives:

```rust
if same proposer + same position + different block hash {
    create equivocation proof
    penalize proposer
    reject conflicting block or quarantine both, according to protocol rules
}
```

Important:

* Equivocation detection must happen after cheap structural checks.
* But it must not require accepting both blocks into canonical state.
* It should use valid signatures as evidence.
* Invalid signatures should not create equivocation proofs.

---

## Mandatory assertions

At minimum:

```rust
assert_eq!(invalid_block_acceptance_count(), 0);
assert_eq!(equivocation_proof_count_for(proposer_id), 1);
assert_validator_penalized_or_quarantined(proposer_id);
assert_no_remaining_forks_in_honest_nodes(&cluster);
assert_chain_valid_under_strict_pqc_policy(&cluster);
```

---

## Failure diagnostics

On failure, print:

* proposer_id
* height/slot/round
* block_hash_a
* block_hash_b
* whether signature A verified
* whether signature B verified
* accepted block hash for that position
* equivocation proof count
* penalty/quarantine state
* canonical tip/state hash of all honest nodes

---

## Important rules

* Do NOT simulate invalid signatures. This test is about two valid conflicting signatures.
* Do NOT classify different proposers at same height as equivocation.
* Do NOT classify duplicate delivery of same block as equivocation.
* Do NOT allow equivocation to create canonical fork divergence.
* Do NOT rely on wall-clock timing unless unavoidable.
* Use deterministic seeded randomness.

---

## Final output format

Report:

1. Tests added
2. Files modified
3. Whether equivocation was detected
4. Whether proof was created and validated
5. Whether proposer was penalized/quarantined
6. Whether honest nodes remained convergent
7. Any Byzantine consensus vulnerability found
8. Exact cargo test command used

Be strict. This test should fail if a validator can sign two valid conflicting blocks for the same consensus position without detection.
