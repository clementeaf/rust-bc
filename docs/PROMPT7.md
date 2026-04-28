You are a senior Rust blockchain protocol economist and consensus security auditor.

Your task is to extend the DLT with explicit **slashing / penalty economics and expiration rules**.

The system already has:

* PQC strict validation
* ML-DSA signatures
* equivocation detection
* equivocation proofs
* gossip propagation
* persistent penalty state
* cross-partition retroactive detection
* deterministic convergence
* persistent crash recovery
* Crypto-DoS protection

Now we need to define and test the lifecycle of validator punishment.

---

## Objective

A validator that equivocates must have a clearly defined penalty lifecycle:

* when penalty starts
* how long it lasts
* whether it is temporary or permanent
* whether stake/reputation is reduced
* whether future proposals are rejected
* whether penalty expiration is deterministic
* whether penalty survives restart
* whether expired penalties cannot be abused

---

## Target files

Prefer adding:

`tests/slashing_penalty_lifecycle.rs`

Modify implementation only if needed:

* `src/consensus/equivocation.rs`
* `src/consensus/slashing.rs`
* `src/consensus/validator_set.rs`
* persistent storage modules

---

# Required data structures

If missing, implement something similar to:

```rust
pub enum PenaltyReason {
    Equivocation,
    InvalidPqcSignatureFlood,
    ProtocolViolation,
}

pub enum PenaltyStatus {
    Active,
    Expired,
    Permanent,
}

pub struct PenaltyRecord {
    pub validator_id: ValidatorId,
    pub reason: PenaltyReason,
    pub proof_hash: Option<BlockHash>,
    pub start_height: u64,
    pub until_height: Option<u64>,
    pub slashed_amount: Option<u128>,
    pub reputation_delta: i64,
    pub status: PenaltyStatus,
}
```

Use the project’s existing types where available.

---

# Policy requirement

Define an explicit policy, for example:

```rust
pub struct PenaltyPolicy {
    pub equivocation_penalty_duration_blocks: u64,
    pub equivocation_is_permanent: bool,
    pub slash_percent_bps: u16,
    pub min_slash_amount: u128,
    pub reputation_penalty: i64,
}
```

Recommended initial defaults:

```rust
equivocation_penalty_duration_blocks = 10_000
equivocation_is_permanent = false
slash_percent_bps = 500 // 5%
min_slash_amount = 1
reputation_penalty = -100
```

If your DLT does not yet have stake accounting, implement reputation-only slashing first and mark stake slashing as future integration.

---

# Tests to add

## 1. `equivocation_creates_active_penalty_record`

Flow:

1. Validator equivocates with two valid ML-DSA blocks.
2. Node creates equivocation proof.
3. Assert penalty record exists:

```rust
assert_eq!(record.reason, PenaltyReason::Equivocation);
assert_eq!(record.status, PenaltyStatus::Active);
assert_eq!(record.start_height, detection_height);
assert_eq!(record.until_height, Some(detection_height + policy.equivocation_penalty_duration_blocks));
assert_eq!(record.reputation_delta, policy.reputation_penalty);
```

---

## 2. `active_penalty_rejects_validator_proposals`

Flow:

1. Penalize validator.
2. Validator submits valid future block while penalty is active.
3. Assert rejected with reason:

```rust
RejectReason::ValidatorPenalized
```

---

## 3. `penalty_expires_at_deterministic_height`

Flow:

1. Penalize validator at height H.
2. Try at H + duration - 1 → rejected.
3. Try at H + duration → accepted or re-enabled depending on policy.
4. Assert all nodes agree on expiration height.

---

## 4. `permanent_penalty_never_expires`

Flow:

1. Set:

```rust
equivocation_is_permanent = true
```

2. Penalize validator.
3. Advance many heights.
4. Assert validator remains penalized.

---

## 5. `penalty_lifecycle_survives_restart`

Flow:

1. Penalize validator.
2. Persist penalty record.
3. Restart node.
4. Assert active status restored.
5. Advance to expiration height.
6. Assert expiration works after restart.

---

## 6. `expired_penalty_does_not_delete_historical_proof`

Flow:

1. Penalize validator.
2. Advance beyond expiration.
3. Validator may be allowed again.
4. Assert equivocation proof remains available historically.

---

## 7. `repeated_equivocation_extends_or_escalates_penalty`

Flow:

1. Validator equivocates once.
2. Validator equivocates again while already penalized.
3. Assert one of the explicit policy behaviors:

Acceptable options:

* extend penalty duration
* escalate to permanent penalty
* increase reputation penalty
* create second penalty record

The behavior must be deterministic and documented.

---

## 8. `slashing_reputation_delta_is_applied_once`

Flow:

1. Same equivocation proof is gossiped multiple times.
2. Assert reputation penalty is applied only once.
3. Duplicate proof must not double-slash.

---

## 9. `honest_validator_never_penalized_by_expiration_logic`

Flow:

1. Honest validator produces normal valid blocks.
2. Advance many heights.
3. Restart node.
4. Assert no penalty record exists.

---

# Mandatory assertions

At minimum:

```rust
assert_penalty_record_exists(validator_id);
assert_penalty_status(validator_id, PenaltyStatus::Active);
assert_validator_rejected_while_penalized(validator_id);
assert_penalty_expiration_height_is_deterministic(validator_id);
assert_proof_still_exists_after_penalty_expiration(validator_id);
assert_no_duplicate_slashing_for_same_proof(validator_id);
```

---

# Consensus integration requirement

Penalty status must be checked before accepting proposals:

```rust
if penalty_manager.is_active_penalty(proposer_id, current_height) {
    return Err(RejectReason::ValidatorPenalized);
}
```

This check must happen before expensive operations where possible.

---

# Persistence requirement

Penalty records must persist to disk.

On restart:

* active penalties restore as active
* expired penalties restore as expired or become expired deterministically based on current height
* permanent penalties restore as permanent
* proof history remains available

---

# Failure diagnostics

On failure print:

* validator_id
* reason
* proof_hash
* start_height
* until_height
* current_height
* status
* reputation before/after
* proposal accept/reject reason
* storage path
* duplicate proof count

---

# Final output format

Report:

1. Penalty policy implemented
2. Tests added
3. Files modified
4. Whether active penalties reject proposals
5. Whether expiration is deterministic
6. Whether permanent penalties remain active
7. Whether penalty lifecycle survives restart
8. Whether duplicate proofs avoid double slashing
9. Whether any economic/penalty bug was found
10. Exact cargo test command used

Be strict. This should fail if penalties are implicit, memory-only, non-deterministic, or double-applied.
