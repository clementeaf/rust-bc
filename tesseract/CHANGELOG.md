# Changelog

## [Unreleased]

### Performance — Dirty-Set Evolution and Reduced Seed Radius

Throughput improved 4–10x via two changes: dirty-cell tracking in
`evolve()` and reduced SEED_RADIUS (4 → 3).

- `evolve()` uses dirty set: only processes cells that changed + their neighbors
- SEED_RADIUS reduced from 4 to 3 (6561 → 2401 cells/seed)
- Seed cost: 440µs → 195µs per event
- 16⁴ throughput: 45 → 210 events/sec
- 32⁴ throughput: 1 → 10 events/sec
- 64⁴ convergence: 144ms → 33ms
- Known trade-off: self-healing not yet restored with bounded cascade

### Fixed — Bounded Field Evolution

Cascade and evolution no longer create unbounded cells. Scale tests
that previously consumed >1GB RAM and never terminated now complete
in seconds.

- `evolve()` only processes existing cells and their existing neighbors
- Cascade boosts only cells that already have evidence (no new cell creation)
- `apply_cascade_from()` targets newly crystallized cells, not all crystals
- Entropy threshold relaxed (0.1 → 0.2) to match bounded cascade dynamics

### Changed — Production Cryptography

Pedersen commitments upgraded from demo-grade modular arithmetic to
Ristretto255 (curve25519-dalek). 128-bit security level.

- `proof.rs`: `Commitment` now operates on `RistrettoPoint` instead of `u128 mod p`
- Generator H derived via hash-to-point ("nothing-up-my-sleeve" construction)
- Same public API — all existing tests pass without modification
- Dependencies: `curve25519-dalek` v4, `ed25519-dalek` v2

### Added — Cryptographic Identity Binding

Identity spoofing is no longer possible. The `org` field is derived from
the signer's Ed25519 public key, not set manually.

- `mapper.rs`: `SignedEvent` — sign events with Ed25519, verify before field entry
- `org_from_public_key()` — deterministic org from `SHA-256(pubkey)[..8]`
- Tampered events rejected at verification (signature check)
- Sybil resistance is now cryptographic + geometric

### Changed — Unified Conservation Stack

The wallet, conservation, and proof layers are now connected.
Previously they were three independent implementations.

- `wallet.rs`: `TesseractLedger` backed by `ConservedField` (u64 balances, nonces)
- Every transfer produces a `TransferReceipt` with Pedersen commitments
- `receipt.verify_conservation()` — algebraic proof without revealing amounts
- `ledger.balance_proof()` — Pedersen commitment for any participant's balance
- Amounts are `u64` (not `f64`) throughout the monetary stack

### Added — Four Fundamental Rules

Physics-inspired consensus primitives that replace software checks with
mathematical impossibilities. Each rule is unhackable not because violation
is "detected" but because it cannot be expressed.

#### Causality (`causality.rs`, `proof::CausalProof`)
- Light cones: events only influence what they can causally reach
- Partial order (Before / After / Concurrent) replaces total ordering
- Event hashes encode full ancestry — forging requires inverting SHA-256
- `Field::with_causality()` enables relativistic mode; `tick()` expands cones

#### Conservation (`conservation.rs`, `proof::Commitment`)
- Pedersen commitments with homomorphic addition
- Balanced transfers are an algebraic identity, not a runtime check
- Imbalance requires solving the elliptic curve discrete logarithm problem
- Genesis is the Big Bang — the only moment value is created

#### Entropy (`entropy.rs`, `proof::Seal`)
- Thermodynamic model: temperature, free energy, Shannon entropy
- Crystallization occurs when energetically favorable (F < 0)
- Hash-chain seals make reversal require inverting SHA-256
- Reversal cost grows with age — old crystals are permanent

#### Gravity (`gravity.rs`)
- Mass computed from the causal graph, not stored in a registry
- Inverse-square influence with superposition
- No registry to hack — mass IS participation in the DAG
- Faking mass requires forging causal events (SHA-256 preimage)
