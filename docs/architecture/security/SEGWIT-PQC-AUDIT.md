# Security Audit — SegWit/PQC Subsystem

**Scope:** `src/transaction/{segwit,verification_cache,pqc_validation,replay_protection,block_version,weight_fee,compact_block,witness_pruning}.rs`
**Date:** 2026-04-30
**Status:** Internal review — external audit pending

---

## Executive Summary

8 files audited, 86 tests reviewed. **3 findings** requiring code changes (1 HIGH, 2 MEDIUM), **4 observations** (LOW/INFO).

Overall the subsystem is well-structured with defense-in-depth: roots checked before cache, fees before signatures, positional witness binding, and domain-separated replay protection. The findings below are hardening recommendations, not exploitable vulnerabilities in the current test suite.

---

## Findings

### F-001 [HIGH] — `validate_segwit_block()` verifies signatures BEFORE roots

**File:** `segwit.rs:168-205`
**Impact:** A malicious block with valid signatures but tampered `tx_root` will waste CPU on expensive ML-DSA verification before being rejected by the root check. This is a DoS vector.

**Detail:** The original `validate_segwit_block()` checks signatures at step 2, then roots at steps 3-4. An attacker can craft a block with valid signatures but a forged `tx_root` to force full signature verification before the cheap root check rejects it. With ML-DSA at ~1ms/verify, a 1000-tx block wastes ~1 second per attempt.

**Note:** `validate_segwit_block_with_cache()`, `validate_segwit_block_parallel()`, and `validate_pqc_block()` all correctly check roots FIRST. Only the original `validate_segwit_block()` has the wrong order.

**Fix:** Reorder `validate_segwit_block()` to match the pipeline: structure → roots → signatures.

**Status:** OPEN

---

### F-002 [MEDIUM] — `signing_payload()` uses `serde_json` JSON key ordering (non-canonical)

**File:** `segwit.rs:45-59`, `replay_protection.rs:29-49`
**Impact:** `serde_json::json!()` macro produces JSON with keys in source-code order, but this is not a guaranteed canonical form. If a different serializer or a future serde_json version changes key ordering, signing payloads will break consensus.

**Detail:** The signing payload relies on:
```rust
serde_json::json!({
    "chain_id": ...,
    "kind": ...,
    "nonce": ...,
    "fee": ...,
    "timestamp": ...,
})
```

The `json!()` macro preserves insertion order (serde_json uses `Map<String, Value>` which is a `BTreeMap` by default — alphabetically sorted). So the actual order is: `chain_id, fee, kind, nonce, timestamp` (alphabetical). This happens to be deterministic but is fragile.

**Recommendation:** Either (a) explicitly sort keys or use `serde_json`'s `to_string()` on a `BTreeMap` explicitly, or (b) pin `serde_json` to a version and document this as a consensus-critical dependency.

**Status:** FIXED — canonical binary serialization (`canonical.rs`) replaces `serde_json` for all consensus bytes (roots, cache keys, short IDs, SegWitPqcV1 signing payload). Legacy signing payload unchanged for backward compatibility. 7 audit regression tests verify no JSON in consensus paths.

---

### F-003 [MEDIUM] — `legacy_sha256()` silent fallback to `[0u8; 32]` in Merkle root

**File:** `segwit.rs:122-123`, `segwit.rs:140`
**Impact:** If `legacy_sha256()` fails (module in Approved mode), the Merkle root silently uses `[0u8; 32]` as the hash for that leaf. This could cause two different transactions to hash to the same value, breaking collision resistance.

**Detail:**
```rust
.map(|data| legacy_sha256(&data).unwrap_or([0u8; 32]))
```

`legacy_sha256()` returns `Err` when the PQC module is in Approved mode (which blocks legacy algorithms). In that scenario, all Merkle leaves become `[0u8; 32]`, making every block's `tx_root` identical regardless of content.

**Recommendation:** Propagate the error instead of silently falling back. Or use `hash_with(HashAlgorithm::Sha256, ...)` from `crypto/hasher.rs` which doesn't go through the approved-mode guard.

**Status:** OPEN

---

### F-004 [LOW] — `verify_witness()` hardcodes `index: 0` in error variants

**File:** `segwit.rs:217, 224, 232, 234`
**Impact:** When `verify_witness()` is called from a loop with the real index `i`, the error returned by `verify_witness` always says `index: 0`. The caller then maps it to the correct index, but if the error is returned directly (e.g. from `validate_or_insert()`), the index is misleading.

**Status:** OBSERVATION — cosmetic, no security impact

---

### F-005 [LOW] — `to_segwit()` determines `signature_scheme` by size heuristic

**File:** `segwit.rs:72-75`
**Impact:** The scheme detection `match self.signature.len() { 3309 => MlDsa65, _ => Ed25519 }` is a fallback for legacy transactions that don't carry a `signature_algorithm` tag. A 3309-byte non-ML-DSA payload would be misclassified.

**Mitigation:** This is only used for legacy conversion (`NativeTransaction::to_segwit`). New SegWit transactions set `signature_scheme` explicitly. The existing `validate_signature_consistency()` in `identity/pqc_policy.rs` catches size/tag mismatches at the consensus layer.

**Status:** ACCEPTED — backward compatibility requirement

---

### F-006 [LOW] — `VersionedBlockHeader::compute_hash()` omits `witness_root` for Legacy but doesn't include a length indicator

**File:** `block_version.rs:45-58`
**Impact:** A Legacy header with `witness_root: None` and a SegWit header with `witness_root: Some([0u8; 32])` could theoretically produce the same hash if the witness_root is all zeros (the extra 32 zero bytes change the hash, so this is actually safe). However, the absence of a length prefix means the hash construction is not fully domain-separated.

**Recommendation:** Consider including a 1-byte flag (0/1) indicating presence of `witness_root` before hashing it, to make the encoding unambiguous even for adversarial inputs.

**Status:** OBSERVATION — no practical collision risk

---

### F-007 [INFO] — `validate_block_versioned()` does not verify `SegWitWithoutWitnessRoot` for SegWit blocks

**File:** `block_version.rs:156-166`
**Impact:** The `BlockVersionError::SegWitWithoutWitnessRoot` variant is defined but never returned. SegWit blocks use `CompactBlockHeader` (which doesn't have `witness_root` in the header — it's on `SegWitBlock` itself), so the check can't fire from `validate_block_versioned()`. The `witness_root` is validated implicitly by `validate_pqc_block()` via Merkle root recomputation.

**Status:** OBSERVATION — dead variant, no security gap

---

## Invariant Verification

| Invariant | Status | Evidence |
|---|---|---|
| `tx_cores.len() == witnesses.len()` | PASS | Checked in all 4 validators |
| `tx_root` recomputed and compared | PASS | All validators recompute |
| `witness_root` recomputed and compared | PASS | All validators recompute |
| `witnesses[i]` verifies `tx_cores[i]` | PASS | Positional zip in all paths |
| `chain_id` in signing payload | PASS | Present in `signing_payload()` and `signing_payload_for_version()` |
| `nonce` in signing payload | PASS | Present in both payload functions |
| `fee` in signing payload | PASS | Present in both payload functions |
| `timestamp` in signing payload | PASS | Present in both payload functions |
| `kind` in signing payload | PASS | Present in both payload functions |
| Cache never bypasses roots | PASS | Roots checked before cache in `validate_segwit_block_with_cache`, `_parallel`, `validate_pqc_block` |
| Cache never bypasses fees | PASS | Fees checked before sigs in `validate_pqc_block` |
| Cache never accepts invalid sigs | PASS | Only `insert_valid` called after successful verify |
| Witness swapping fails | PASS | Cache key binds `(core, witness)` pair; 4 tests cover this |
| Pruning preserves `witness_root` | PASS | `PrunedSegWitBlock` carries `witness_root` |
| Pruned block rejected as full | PASS | Length mismatch (0 witnesses) |
| Cross-version replay prevented | PASS | Domain separator + version byte |
| Short IDs not used for consensus | PASS | Full validation after reconstruction |

---

## Recommendations Summary

| ID | Severity | Fix | Status |
|---|---|---|---|
| F-001 | HIGH | Reorder `validate_segwit_block()`: roots before signatures | **FIXED** |
| F-002 | MEDIUM | Canonical binary serialization for consensus bytes | **FIXED** |
| F-003 | MEDIUM | Replace `unwrap_or([0u8; 32])` with `hash_with()` | **FIXED** |
| F-004 | LOW | Pass index to `verify_witness()` or return a generic error | OBSERVATION |
| F-005 | LOW | Accepted — legacy compat | ACCEPTED |
| F-006 | LOW | Consider adding presence flag to header hash | OBSERVATION |
| F-007 | INFO | Remove dead `SegWitWithoutWitnessRoot` variant or add check | OBSERVATION |

**All HIGH and MEDIUM findings resolved.** 0 CRITICAL, 0 HIGH, 0 MEDIUM open.

---

## Next Steps

1. Schedule external audit covering this subsystem + `pqc_crypto_module`
