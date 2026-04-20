# Changelog

## [Unreleased]

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
- Imbalance requires solving the discrete logarithm problem
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

### Added — Cryptographic Primitives (`proof.rs`)
- `Commitment`: Pedersen scheme over Z_p* with homomorphic addition
- `Seal`: append-only hash chain for irreversible crystallization
- `CausalProof`: ancestry-encoding hash with deterministic parent ordering
- `verify_conservation()`: algebraic balance verification
