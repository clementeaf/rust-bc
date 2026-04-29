# Cerulean Ledger — Technical Whitepaper

**Version 0.1 — April 2026**

---

## Abstract

Cerulean Ledger is a post-quantum-ready Layer 1 blockchain and cryptocurrency platform built in Rust. It combines a BFT consensus engine, native token economics with deflationary pressure, and NIST FIPS 204/203 post-quantum cryptography from day one. This document describes the security model, tokenomics, consensus mechanism, account model, and known limitations.

---

## 1. Post-Quantum Security Model

### 1.1 Threat Model

Quantum computers capable of breaking RSA-2048 and ECDSA are projected within 10–15 years (NIST PQC timeline). Cerulean Ledger adopts a **harvest-now-decrypt-later** defensive posture: all on-chain signatures must remain secure against adversaries who record today's traffic for future quantum decryption.

### 1.2 Algorithms

| Operation | Algorithm | Standard | Security Level |
|---|---|---|---|
| Digital signatures | ML-DSA-65 | FIPS 204 | NIST Level 3 |
| Key encapsulation | ML-KEM-768 | FIPS 203 | NIST Level 3 |
| Hashing | SHA3-256 | FIPS 202 | 128-bit |
| Legacy signatures | Ed25519 | RFC 8032 | ~128-bit classical |

### 1.3 Crypto Module Boundary

All cryptographic operations are isolated in `pqc_crypto_module`, a standalone Rust crate with:

- **Approved-mode state machine**: `Uninitialized → SelfTesting → Approved → Error`
- **KAT self-tests** at startup for all three algorithms
- **ZeroizeOnDrop** on all private key types with `mlock` memory protection
- **No classical fallback** in approved mode — Ed25519/SHA-256 only available via explicit `legacy::*` imports

Production code has **zero direct imports** of raw crypto crates (`sha2`, `ed25519_dalek`, `pqcrypto_mldsa`). Enforced by a boundary test that fails if any production file bypasses the module.

### 1.4 Dual-Signing Migration

For networks transitioning from Ed25519 to ML-DSA-65:

1. **Transition phase**: blocks carry both primary and secondary signatures. `DUAL_SIGN_VERIFY_MODE=either` — either signature validates.
2. **Strict phase**: `DUAL_SIGN_VERIFY_MODE=both` — both signatures required.
3. **Post-migration**: `REQUIRE_PQC_SIGNATURES=true` — classical signatures rejected.

### 1.5 Address Derivation

```
address = hex(SHA-256(public_key)[0..20])
```

40-character hex address, algorithm-agnostic. Works with Ed25519 (32-byte pk) and ML-DSA-65 (1952-byte pk).

---

## 2. NOTA Tokenomics

### 2.1 Supply

| Parameter | Value |
|---|---|
| Maximum supply | 100,000,000 NOTA |
| Realizable emission | ~20,370,000 NOTA (integer halving) |
| Initial block reward | 50 NOTA |
| Halving interval | 210,000 blocks |
| Halving eras | 6 (50 → 25 → 12 → 6 → 3 → 1 → 0) |
| Block time target | 15 seconds |
| Time to full emission | ~6 years at target block time |

The difference between theoretical maximum (100M) and realizable emission (~20.37M) is intentional — `MAX_SUPPLY` serves as a hard ceiling that `capped_block_reward()` enforces, while integer division in the halving schedule determines actual emission.

### 2.2 Fee Model (EIP-1559 Inspired)

- **Base fee**: dynamically adjusted per block based on utilization
- **Target utilization**: 50% of `MAX_TXS_PER_BLOCK` (500)
- **Adjustment**: ±12.5% per block (factor of 8)
- **Minimum base fee**: 1 NOTA (absolute floor)
- **Fee split**: 80% burned / 20% to block proposer
- **Minimum transaction fee**: 1 NOTA

The burn mechanism creates deflationary pressure proportional to network usage. At high utilization, more tokens are burned than minted, creating net deflation.

### 2.3 Epoch System

- **Epoch length**: 11,520 blocks (~2 days at 15s/block)
- Epoch fees are tracked for future staking reward distribution
- Epoch boundary resets fee accumulators

### 2.4 Storage Deposits

State writes require a deposit proportional to data size. Deposits are refunded on deletion and adjusted on updates. This prevents state bloat by making storage economically costly.

### 2.5 Validator Economics

| Parameter | Value |
|---|---|
| Minimum proposer stake | 1,000 NOTA |
| Minimum governance stake | 100 NOTA |
| Validator income | Block reward + 20% of tx fees |
| Slashing | Equivocation detection + penalty manager |

Annual yield decreases as total staked increases, incentivizing early participation while maintaining security through minimum stake requirements.

---

## 3. Consensus

### 3.1 BFT Protocol

HotStuff-inspired 4-phase BFT with pipelining:

1. **Prepare** — leader proposes block, validators vote
2. **Pre-Commit** — leader aggregates votes into QC
3. **Commit** — validators acknowledge QC
4. **Decide** — block finalized

Properties:
- **Safety**: no two honest nodes decide different blocks for the same round
- **Liveness**: progress with up to f Byzantine faults (n ≥ 3f + 1)
- **Finality**: single-round finality (no probabilistic confirmation)

### 3.2 Validator Selection (DPoS)

- Stake-weighted committee election from top-N candidates
- Proportional leader rotation based on stake
- Minimum 4 validators for BFT viability

### 3.3 Equivocation Detection

- `EquivocationDetector` tracks proposals per `(height, slot, proposer)`
- Conflicting proposals generate `EquivocationProof`
- Proofs gossiped and deduplicated across network
- Offending proposers quarantined

### 3.4 Slashing

- `PenaltyManager` with configurable duration/permanent/escalation policies
- Deterministic expiration at `start_height + duration`
- Anti-double-slash prevents re-penalizing the same offense
- Reputation tracking for progressive penalty escalation

---

## 4. Account Model

### 4.1 State

```rust
struct AccountState {
    balance: u64,       // NOTA token balance
    nonce: u64,         // Replay protection counter
    code_hash: Option,  // Smart contract identifier
}
```

Accounts are created implicitly on first credit (no registration needed). Default state: zero balance, zero nonce.

### 4.2 Transaction Types

| Type | Description | Signature Required |
|---|---|---|
| Transfer | Native value transfer | Yes |
| Coinbase | Block reward (protocol-only) | No |

### 4.3 Domain Separation

Every transaction includes `chain_id` in the signing payload:

```json
{
  "chain_id": 9999,
  "kind": { "Transfer": { "from": "...", "to": "...", "amount": 100 } },
  "nonce": 0,
  "fee": 5,
  "timestamp": 1714300000
}
```

| Chain ID | Network |
|---|---|
| 1 | Mainnet |
| 9999 | Testnet |
| 9998 | Devnet |
| 0 | Legacy/any (backwards compat) |

Transactions with `chain_id ≠ 0` are rejected on networks with a different ID, preventing cross-network replay.

### 4.4 Fee-Ordered Mempool

- BTreeMap ordered by fee descending (highest priority first)
- Configurable pool size (default: 10,000) and per-sender limit (default: 64)
- Eviction: lowest-fee tx replaced when pool is full and incoming fee is higher
- Dedup via known-ID set (including evicted)
- Minimum fee enforcement at admission

---

## 5. Block Production Pipeline

```
Mempool (fee-ordered)
    │
    ▼
drain_top(max_txs)  ─── highest fee first
    │
    ▼
execute_transfer()  ─── nonce check, balance debit, fee split
    │
    ▼
apply_block_rewards() ── mint reward to proposer
    │
    ▼
persist_block()  ────── write to BlockStore + transactions
    │
    ▼
announce_native_block() ── gossip to peers
```

Failed transactions are recorded in the block result but don't prevent other transactions from executing. Only successful transactions are persisted.

---

## 6. Network Architecture

### 6.1 P2P Layer

- TCP-based gossip protocol with TLS (optional mTLS)
- Alive messages for peer liveness detection
- Pull-based state sync for catching up
- Anchor peers for cross-organization discovery

### 6.2 Message Types (Crypto-specific)

| Message | Purpose |
|---|---|
| `NativeTransferGossip` | Propagate new transfer to peer mempools |
| `NativeBlockAnnounce` | Broadcast produced block to network |

### 6.3 API Endpoints

| Method | Path | Description |
|---|---|---|
| GET | `/accounts/{address}` | Balance + nonce |
| POST | `/transfer` | Submit native transfer |
| GET | `/mempool/stats` | Pool size + base fee |
| POST | `/faucet/drip` | Request testnet tokens |
| GET | `/faucet/status` | Faucet availability |

---

## 7. Limitations and Future Work

### 7.1 Current Limitations

- **No smart contract gas metering**: native transfers only; EVM layer exists but isn't integrated with the native account model
- **Single-threaded block production**: `produce_block()` executes transfers sequentially (parallel execution exists for Fabric-style transactions but not for native transfers)
- **No state trie**: account state is a flat HashMap; no Merkle proof for light client state verification of balances
- **No SPV**: light client verifies block headers but cannot prove individual account balances
- **No persistent mempool**: mempool is in-memory only; lost on restart
- **Signature not verified at mempool admission**: `verify_tx_signature()` exists but isn't enforced at the API layer (left to block producer)
- **Genesis allocations are code-level**: no genesis.json file; allocations are defined in Rust structs

### 7.2 Future Work

| Priority | Feature | Description |
|---|---|---|
| High | State trie (sparse Merkle) | Enable balance proofs for light clients |
| High | Gas metering for native txs | Prevent compute-heavy transactions |
| High | Mempool signature enforcement | Reject invalid signatures before queuing |
| Medium | HD wallets (BIP-39/44) | Mnemonic seed phrases, derivation paths |
| Medium | EVM integration with account model | Unified balance for native + EVM transactions |
| Medium | Persistent mempool | Survive node restarts |
| Low | State snapshots for fast sync | Download state at height N instead of replaying from genesis |
| Low | Light client balance proofs | SPV-style verification for mobile/IoT |
| Research | zkSNARK privacy layer | Private transfers with zero-knowledge proofs |

---

## 8. Security Audit Status

- **17 adversarial tests** covering: double spend, replay, nonce gaps, mempool spam, fee sniping, signature flood, economic invariants
- **Supply invariant verified**: `verify_supply_invariant()` proves `minted ≤ MAX_SUPPLY` and `burned ≤ minted`
- **Emission projection tested**: total emission converges to 20.37M NOTA over 6 halving eras
- **Crypto boundary**: 100% compliance — zero direct crypto imports in production code
- **FIPS pre-lab audit**: 0 CRITICAL, 0 HIGH, 0 MEDIUM findings open

---

## References

1. NIST FIPS 204 — Module-Lattice-Based Digital Signature Standard (ML-DSA)
2. NIST FIPS 203 — Module-Lattice-Based Key-Encapsulation Mechanism Standard (ML-KEM)
3. NIST FIPS 202 — SHA-3 Standard
4. EIP-1559 — Fee market change for ETH 1.0 chain
5. Yin et al., "HotStuff: BFT Consensus with Linearity and Responsiveness", PODC 2019
6. Bitcoin whitepaper — halving schedule and supply cap model

---

*Cerulean Ledger — Post-quantum cryptocurrency infrastructure for a future-proof digital economy.*
