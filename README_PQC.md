# Cerulean Ledger — Post-Quantum SegWit Architecture

Technical reference for the PQC/SegWit subsystem in rust-bc.

---

## 1. Overview

Cerulean Ledger is a blockchain node written in Rust with native support for post-quantum cryptography. The primary signature algorithm is **ML-DSA-65** (NIST FIPS 204), which produces signatures of 3309 bytes and public keys of 1952 bytes — roughly 50x larger than Ed25519.

**Problem:** Naively embedding PQC signatures inside transactions makes blocks ~50x heavier, slowing propagation, increasing storage, and raising verification costs.

**Solution:** A Segregated Witness architecture adapted for PQC that separates executable data from cryptographic proofs, enabling:

- Compact block propagation (84.7% bandwidth reduction)
- Parallel signature verification (2.7x speedup)
- Witness pruning for non-archival nodes
- Weight-based fees proportional to real cost

---

## 2. Architecture

### Transaction Model

```
TxCore (~150–200 bytes)          TxWitness (~5.3 KB for ML-DSA)
├── from                         ├── signature (3309 bytes)
├── to                           ├── public_key (1952 bytes)
├── amount                       └── signature_scheme (Ed25519 | MlDsa65)
├── fee
├── nonce
├── chain_id
├── timestamp
└── kind
```

`TxCore` holds everything that affects state. `TxWitness` holds the cryptographic proof. They are linked by position: `witnesses[i]` proves `tx_cores[i]`.

### Block Model

```
SegWitBlock:
  header:        CompactBlockHeader (height, hash, parent_hash, timestamp, proposer)
  tx_cores:      Vec<TxCore>
  witnesses:     Vec<TxWitness>
  tx_root:       [u8; 32]    ← Merkle root of serialized tx_cores
  witness_root:  [u8; 32]    ← Merkle root of serialized witnesses
```

Both roots are independently verifiable. Light clients can verify `tx_root` without downloading witnesses.

### Validation Pipeline

```
structure → roots → fees → signatures
```

Each stage gates the next. Failures in cheap stages prevent expensive work.

---

## 3. Scalability

### Verification Cache

Avoids re-verifying expensive ML-DSA signatures already seen in mempool.

- Key: `SHA-256(serialize(core) || serialize(witness))` — binds both halves
- FIFO eviction at configurable `max_entries`
- Never caches invalid signatures

### Parallel Verification (rayon)

Signatures verified in parallel across CPU cores. Cache is snapshotted for lock-free read during the parallel phase; newly verified pairs are inserted sequentially afterwards.

**Measured:** 2.7x speedup at 200 transactions (75ms → 28ms, Ed25519).

### Compact Block Propagation

Replaces full objects with 8-byte short IDs (`first_8_bytes(SHA3-256(serialized))`). Peers reconstruct blocks from mempool, requesting only missing objects.

**Measured:** 84.7% size reduction (11,584 → 1,771 bytes at 20 transactions).

### Witness Pruning

Non-archival nodes discard witnesses after `pruning_depth` confirmations. `witness_root` is preserved as a cryptographic commitment that witnesses existed and were valid.

**Storage savings:** ~97% for blocks with ML-DSA witnesses (5.3 KB/tx → ~200 bytes/tx after pruning).

---

## 4. Security

### Cryptographic Primitives

| Algorithm | Standard | Use |
|---|---|---|
| ML-DSA-65 | FIPS 204 | Primary transaction signatures |
| Ed25519 | RFC 8032 | Legacy signatures (pre-activation) |
| SHA-256 | FIPS 180-4 | Merkle roots, cache keys |
| SHA3-256 | FIPS 202 | Short IDs for compact blocks |

ML-DSA-65 implementation validated against ACVP test vectors (13 sigVer + 5 sigGen + 5 keyGen vectors, `cargo test --features acvp-tests`).

### Signing Payload

The signing payload includes all state-affecting fields:

```json
{
  "chain_id": 1,
  "kind": {"type": "Transfer", "from": "...", "to": "...", "amount": 100},
  "nonce": 0,
  "fee": 5000,
  "timestamp": 1700000000
}
```

For SegWitPqcV1 blocks, the payload is additionally prefixed with:

```
"RUST_BC_SEGWIT_PQC_V1_TX" || 0x01 || canonical_json
```

This domain separator prevents cross-version replay.

### Protections

| Attack | Mitigation |
|---|---|
| Quantum key recovery | ML-DSA-65 (NIST Level 3, ~143-bit classical security) |
| Cross-chain replay | `chain_id` in signed payload |
| Cross-version replay | Domain separator + version byte in payload |
| Witness swapping | Position binding: `witnesses[i]` must verify `tx_cores[i]`; cache key binds both |
| Field tampering | All fields in signing payload; `tx_root` changes on any modification |
| Cache bypass | Roots validated before cache is consulted |
| Signature malleability | Size validation: Ed25519 = 64 bytes, ML-DSA = 3309 bytes; reject mismatches |

---

## 5. Economic Model

Fees are proportional to real resource cost:

```
weight = serialized_size(core) × 4 + serialized_size(witness) × 1
fee_required = BASE_TX_FEE + weight × FEE_PER_WEIGHT_UNIT
```

| Constant | Value | Rationale |
|---|---|---|
| `CORE_MULTIPLIER` | 4 | Core is stored permanently, re-executed by all nodes |
| `WITNESS_MULTIPLIER` | 1 | Witness verified once, pruneable |
| `BASE_TX_FEE` | 1 | Minimum floor |
| `FEE_PER_WEIGHT_UNIT` | 1 | Linear scaling |

**Result:** ML-DSA transactions naturally cost ~19x more than Ed25519 (required fee ~19,268 vs ~991) due to witness size, not an artificial surcharge.

---

## 6. Consensus Versioning

```rust
enum BlockVersion {
    Legacy = 0,      // Pre-SegWit: no witnesses, no weight fees
    SegWitPqcV1 = 1, // Dual Merkle roots, witnesses required, weight fees
}
```

Activation is height-based:

```
height < activation_height  → only Legacy allowed
height >= activation_height → only SegWitPqcV1 allowed
```

The version byte is included in the block header hash, making version changes detectable and non-malleable.

---

## 7. Official Validation

```rust
pub fn validate_pqc_block(
    block: &SegWitBlock,
    cache: &mut VerificationCache,
    config: &PqcValidationConfig,
) -> Result<(), PqcBlockError>
```

Configuration:

```rust
PqcValidationConfig {
    enforce_fees: bool,      // Weight-based fee check
    use_cache: bool,         // Read/write verification cache
    parallel_verify: bool,   // Rayon parallel signatures
}
```

Consensus invariants (documented in `docs/pqc-consensus-invariants.md`):

- Cache never bypasses roots or fees
- Cache never accepts invalid signatures
- Roots are always recomputed and compared
- Pipeline order is mandatory: structure → roots → fees → signatures

---

## 8. Threat Model Summary

| Threat | Status | Mechanism |
|---|---|---|
| Quantum computing (Shor's algorithm) | Mitigated | ML-DSA-65 lattice-based signatures |
| Replay attack (cross-chain) | Mitigated | `chain_id` in signing payload |
| Replay attack (cross-version) | Mitigated | Domain separator + version byte |
| Witness swapping | Mitigated | Positional binding + dual Merkle roots |
| Transaction tampering | Mitigated | Full payload signed, tx_root commitment |
| Cache poisoning | Mitigated | Only valid signatures cached; roots checked first |
| Network bandwidth (large PQC sigs) | Mitigated | Compact blocks (84.7% reduction) |
| Storage growth | Mitigated | Witness pruning after depth threshold |
| CPU cost (ML-DSA verify) | Mitigated | Verification cache + parallel verify (2.7x) |

---

## 9. Current Limitations

The following are known limitations as of this version:

- **External audit pending.** No third-party security audit has been performed on the SegWit/PQC subsystem.
- **ACVP vectors are library-generated.** Full NIST-sourced ACVP vectors are optional; current tests use vectors generated from `pqcrypto-mldsa`.
- **P2P propagation not integrated.** Compact block protocol exists as a model but is not wired into the network layer.
- **Dynamic fee market not implemented.** Fees are static (weight × rate). No EIP-1559-style congestion pricing for the SegWit layer.
- **No L2 / rollup support.** The SegWit model is L1-only.
- **ML-DSA determinism.** `pqcrypto-mldsa` uses randomized signing; deterministic mode (hedged) is not yet enforced.
- **Single PQC algorithm.** Only ML-DSA-65 is supported. No algorithm agility framework for future NIST selections.

---

## 10. Roadmap

| Priority | Item |
|---|---|
| High | External security audit of PQC/SegWit subsystem |
| High | Integration of compact block protocol into P2P layer |
| Medium | Mempool priority queue with weight-based ordering |
| Medium | Dynamic fee market (congestion-responsive) |
| Medium | Full NIST ACVP vector suite (official download) |
| Low | Algorithm agility (ML-DSA-44, ML-DSA-87, SLH-DSA) |
| Low | Cross-chain bridge with PQC signature verification |
| Low | L2 scaling (validity proofs over SegWit blocks) |

---

## Test Coverage

| Module | Tests | Focus |
|---|---|---|
| `segwit.rs` | 7 | Core model, roots, signing, tampering |
| `verification_cache.rs` | 16 | Cache hit/miss, eviction, parallel |
| `compact_block.rs` | 12 | Short IDs, reconstruction, collision safety |
| `witness_pruning.rs` | 7 | Depth rules, root preservation |
| `weight_fee.rs` | 7 | Weight model, Ed25519 vs ML-DSA cost |
| `pqc_validation.rs` | 11 | Unified pipeline, config flags |
| `block_version.rs` | 10 | Version routing, activation rules |
| `replay_protection.rs` | 9 | Domain separation, cross-version replay |
| `mldsa65_acvp` | 7 | ACVP vectors, size validation |
| **Total** | **86** | |

All tests pass: `cargo test --lib` (1616 tests) + `cargo test --features acvp-tests --test mldsa65_acvp` (7 tests).

---

## Source Files

```
src/transaction/
├── segwit.rs              # TxCore, TxWitness, Merkle roots, validation
├── verification_cache.rs  # Signature cache, parallel block validation
├── compact_block.rs       # Compact propagation, ShortId, mempool reconstruction
├── witness_pruning.rs     # PrunedSegWitBlock, depth-based pruning
├── weight_fee.rs          # Weight calculation, fee model
├── pqc_validation.rs      # Unified pipeline (validate_pqc_block)
├── block_version.rs       # BlockVersion, AnyBlock, activation routing
└── replay_protection.rs   # Domain separator, versioned signing payload
```
