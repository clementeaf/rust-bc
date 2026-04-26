# Tesseract — Limitations & Resolution Status

> Originally discovered via E2E conflict testing (`tests/e2e.rs`, tests 12–16).
> Last updated: 2025-04-25.

---

## 1. Executive Summary

Three critical limitations were identified through adversarial E2E testing. All three have been addressed with minimal, deterministic mechanisms:

| # | Limitation | Status | Resolution |
|---|-----------|--------|------------|
| 1 | Record divergence after split-brain | **CLOSED** | Evidence-carrying sync + `resolve()` |
| 2 | Influence records local-only | **CLOSED** | Boundary protocol now carries influences; `resolve()` merges as union |
| 3 | Cross-node double-spend | **CLOSED** | Spent-nonce tracking + deterministic hash-based resolution |

The system has evolved from **distributed aggregation with local finality** to **deterministic convergence with cross-node conflict resolution**.

---

## 2. Limitation 1: Record Divergence After Split-Brain Merge — CLOSED

### Original Problem

Two partitioned nodes could independently crystallize different events at the same coordinate. After reconnection, the boundary sync propagated `crystallized=true` but not the evidence that justified it. Both nodes agreed the coordinate was crystallized but disagreed on what happened there.

### Resolution

1. **Evidence roots** (`Cell.evidence_root: [u8; 32]`, `Cell.evidence_count: u32`): Each cell now carries a deterministic SHA-256 hash over all its evidence (influences + attestations, sorted). Identical evidence sets produce identical roots regardless of insertion order.

2. **Evidence-carrying boundary sync**: The wire protocol was extended from `{coord, p, k}` to `{coord, p, k, er, ec, infs}`, carrying influence records alongside probability and crystallization state.

3. **`resolve()` function**: A pure, deterministic merge function that combines two cells into one. Properties: idempotent, commutative, associative. Resolution order: `crystallized` (true > false) > `probability` (higher) > `evidence_count` (higher) > `evidence_root` (byte-order tiebreak). Evidence from both sides is merged as a set union.

### Verification

- `split_brain_conflicting_crystallizations_diverge_records` — now asserts both nodes converge to identical records containing evidence from both parties.
- `resolve_commutative`, `resolve_associative`, `resolve_idempotent` — verify algebraic properties.
- `resolve_split_brain_produces_unified_state` — verifies evidence from both sides is present in merged cell.

---

## 3. Limitation 2: Influence Records Local-Only — CLOSED

### Original Problem

The boundary sync wire protocol only transmitted `{coord, probability, crystallized}`. Influence records, attestations, and all provenance data were local to each node. Sigma-independence could not be verified cross-node.

### Resolution

The boundary protocol now includes influence records (`infs` array) in every boundary cell. On receipt, the `resolve()` function merges influences as a set union (deduplicating by event_id). Evidence roots are recomputed after merge, ensuring both nodes converge to the same root when they have the same evidence set.

### Verification

- `influences_sync_via_boundary_after_resolve` — two isolated nodes seed different events at the same coordinate. After reconnection, both nodes have both parties' influences.
- `divergent_nodes_have_different_evidence_roots` — proves roots detect divergence.
- `converged_nodes_have_same_evidence_roots` — proves roots match after convergence.

---

## 4. Limitation 3: Cross-Node Double-Spend — CLOSED

### Original Problem

The conservation layer (`ConservedField`) was entirely local. Two partitioned nodes starting from the same genesis could independently process transfers from the same account, each succeeding locally. Combined, more value was spent than existed (e.g., 800 + 900 = 1700 from a 1000 balance).

### Resolution

1. **Spent-nonce tracking**: `ConservedField` now maintains a `spent_nonces: HashMap<(Coord, u64), [u8; 32]>` mapping each `(source_coord, nonce)` to the transaction hash that claimed it.

2. **`check_remote_transfer()`**: During evidence sync, remote transactions are checked against the local spent-nonce map. If the same `(coord, nonce)` was claimed by a different transaction hash, a `ConservationError::DoubleSpend` is returned.

3. **Deterministic resolution**: When a conflict is detected, the transaction with the lexicographically lower hash wins. Both nodes arrive at the same winner independently — no coordination required.

4. **`resolve_double_spend()`**: The losing transaction can be reverted and replaced with the winning one, restoring conservation invariants.

### Verification

- `double_spend_across_partition_detected_and_resolved` — two partitioned fields process conflicting transfers. Cross-check detects the double-spend. Both sides agree on the winner.
- `double_spend_deterministic_resolution` — verifies both sides independently pick the same winner.
- `spent_nonces_tracked_after_transfer` — verifies nonce tracking is populated.
- `unknown_nonce_accepted` — verifies non-conflicting remotes are accepted.

---

## 5. Remaining Considerations

### What Tesseract Can Now Affirm

- **Deterministic convergence**: Two nodes with conflicting state converge to identical state after boundary exchange. The `resolve()` function is commutative, associative, and idempotent — merge order does not matter.
- **Evidence-verified crystallization**: Crystallization claims now carry their evidence. Receiving nodes can inspect the influence set that justified crystallization.
- **Double-spend detection**: Conflicting monetary transfers from the same source are detected on reconnection and resolved deterministically. No coordinator needed.
- **All original properties retained**: Single-node correctness, self-healing, crystallization, and conservation invariants are unchanged.

### What Tesseract Does Not Yet Provide

- **Automatic state rollback**: Double-spend detection identifies the conflict and determines the winner, but automatic reversal of the losing transaction is available as an API (`resolve_double_spend()`) rather than triggered automatically during sync. A full integration into the sync loop would require the wallet layer to participate in boundary exchange.
- **Attestation sync**: The current evidence sync carries legacy influences. Dimension-bound attestations (the sigma-independence model) are not yet transmitted in the boundary protocol. This means sigma verification remains local for attestation-mode events.
- **Proof of non-existence**: A node can prove what it has seen, but cannot prove that no other node has seen something different. This is fundamental to AP systems and is not addressed.
- **Byzantine fault tolerance**: The `resolve()` function handles honest-but-partitioned nodes correctly. A deliberately Byzantine node that fabricates evidence roots or claims false crystallization is not yet detected. This would require signature-based attestation verification.

### Path to Full Consensus

The three closures move Tesseract from "aggregation with local finality" to "deterministic convergence with conflict resolution." The remaining distance to full consensus is:

1. **Attestation sync** — extend boundary protocol to carry `attestations` alongside `influences`. Recompute sigma-independence on the receiving side.
2. **Signed evidence** — bind attestations to cryptographic identities so that fabricated evidence can be detected and rejected.
3. **Automatic rollback integration** — wire `resolve_double_spend()` into the sync loop so that losing transactions are reverted without manual intervention.

These are incremental additions to the existing infrastructure, not architectural changes.

---

---

## 6. New Gaps Found via Adversarial Testing

Discovered via `tests/adversarial_convergence.rs` (23 tests, 7 attack categories).

### Adversarial Classification Summary

| Attack | Category | Status |
|--------|----------|--------|
| Sybil spam vs crystallized | Sybil | MITIGATED |
| Sybil spam tiebreak control | Sybil | MITIGATED |
| Sybil same dimension | Sybil | PARTIAL |
| Sybil same validator all dims | Sybil | MITIGATED |
| Replay transfer (nonce) | Replay | MITIGATED |
| Replay remote same hash | Replay | MITIGATED |
| Replay evidence in resolve | Replay | MITIGATED |
| Equivocation attestations | Equivocation | **CLOSED** |
| Equivocation visible after merge | Equivocation | PARTIAL |
| Prolonged 2-way partition | Partition | MITIGATED |
| 3-way partition merge order | Partition | MITIGATED |
| Byzantine inflated probability | Byzantine | **CLOSED** |
| Byzantine fabricated count | Byzantine | MITIGATED |
| Byzantine false crystallization | Byzantine | **CLOSED** |
| Arrival order evidence root | Order | MITIGATED |
| Arrival order resolve chain | Order | MITIGATED |
| Arrival order field seeds | Order | MITIGATED |
| resolve() properties (6 proptests) | Properties | MITIGATED |

### Gap 4: Equivocation — CLOSED

**Resolution:** `Cell::equivocating_validators()` detects validators that attest contradictory event_ids on the same dimension. `sigma_independence()` excludes equivocating validators from the computation. Attestations are preserved for audit but carry no weight for consensus.

**Test:** `equivocation_detected_and_excluded_from_sigma`

### Gap 5: Byzantine False Crystallization — CLOSED

**Resolution:** `resolve()` now re-verifies crystallization after merging evidence:
- Attestation model: requires `sigma_independence() >= 4` (with equivocation exclusion)
- Legacy model: requires `probability >= CRYSTALLIZATION_THRESHOLD`
- No evidence: `crystallized` forced to `false`

A Byzantine peer claiming `k=true` with empty or insufficient evidence is degraded to `k=false`.

**Test:** `byzantine_false_crystallization_rejected`

### Gap 6: Byzantine Inflated Probability — CLOSED

**Resolution:** `resolve()` now derives probability from actual evidence weights. The claimed probability is capped at the sum of influence/attestation weights (matching the accumulation logic in `seed_named()`). A Byzantine peer claiming `p=1.0` with weak evidence gets capped to the evidence-supported value.

**Test:** `byzantine_inflated_probability_capped`

---

## 7. Current Status

All 6 gaps (3 original + 3 adversarial) are **CLOSED**.

| Gap | Description | Status |
|-----|------------|--------|
| 1 | Record divergence after split-brain | CLOSED |
| 2 | Influence records local-only | CLOSED |
| 3 | Cross-node double-spend | CLOSED |
| 4 | Equivocation not detected | CLOSED |
| 5 | Byzantine false crystallization | CLOSED |
| 6 | Byzantine inflated probability | CLOSED |

### Remaining Partial Mitigations

- **Sybil same dimension:** 50 sybil validators on one dimension achieve sigma=1. This is correct behavior (sigma measures dimension diversity, not validator count), but a determined attacker with validators on all 4 dimensions can achieve sigma=4. Mitigation requires identity-binding (cryptographic validator registration).
- **Equivocation visible after merge:** Contradictory claims from the same actor coexist in the evidence union. They are excluded from sigma but not automatically removed. Full cleanup requires a garbage collection pass.

### Test Coverage

| Suite | Tests | Status |
|-------|-------|--------|
| `tests/evidence_sync.rs` | 24 | All pass |
| `tests/adversarial_convergence.rs` | 23 | All pass |
| `tests/e2e.rs` | 19 | All pass |
| Unit tests (`--lib`) | 220 | All pass |

---

*This document reflects the state after closing all 6 identified gaps. Test references: `tests/e2e.rs`, `tests/evidence_sync.rs`, `tests/adversarial_convergence.rs`.*
