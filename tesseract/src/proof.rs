//! Cryptographic primitives that make rule violations mathematically impossible.
//!
//! These are not checks — they are algebraic structures where invalid states
//! cannot be represented. Like how gravity isn't "enforced" by a validator,
//! these commitments are properties of the math itself.
//!
//! ## Pedersen Commitments (Conservation)
//!
//! A Pedersen commitment hides a value while preserving additive homomorphism:
//!   C(v, r) = v·G + r·H  (mod p)
//!
//! Key property: C(a, r1) + C(b, r2) = C(a+b, r1+r2)
//! This means: if sum(input_commits) == sum(output_commits), then
//! sum(input_values) == sum(output_values) — WITHOUT revealing the values.
//!
//! Breaking this requires solving the discrete logarithm problem.
//! Not "hard" — mathematically impossible with current knowledge.
//!
//! ## Hash Chain Seals (Entropy)
//!
//! Each crystallization event produces: S(n) = SHA-256(S(n-1) || evidence)
//! Reversing requires inverting SHA-256 — preimage resistance.
//! The seal IS the history. You can't rewrite one link without
//! invalidating everything after it.

use sha2::{Digest, Sha256};

// --- Pedersen Commitment Scheme (mod p) ---
//
// Using a safe prime for demonstration. In production, use curve25519.
// The math is identical — only the group changes.
//
// p = safe prime (p = 2q + 1 where q is also prime)
// g, h = generators of the subgroup of order q
// Commitment: C = g^value * h^blinding mod p
//
// Security: finding `log_g(h)` is the discrete log problem.
// Without it, you cannot open a commitment to a different value.

/// Safe prime for the commitment scheme.
/// p = 2 * q + 1 where q is prime. Small for prototype — production uses 256-bit.
const PRIME: u128 = 1_000_000_007_000_000_003; // ~60 bits, safe prime
const GENERATOR_G: u128 = 5;
const GENERATOR_H: u128 = 7;
/// Order of the subgroup (q = (p-1)/2).
const ORDER: u128 = (PRIME - 1) / 2;

/// Modular exponentiation: base^exp mod modulus.
fn mod_pow(mut base: u128, mut exp: u128, modulus: u128) -> u128 {
    if modulus == 1 { return 0; }
    let mut result: u128 = 1;
    base %= modulus;
    while exp > 0 {
        if exp % 2 == 1 {
            result = mod_mul(result, base, modulus);
        }
        exp /= 2;
        base = mod_mul(base, base, modulus);
    }
    result
}

/// Modular multiplication avoiding overflow.
fn mod_mul(a: u128, b: u128, m: u128) -> u128 {
    ((a as u128) * (b as u128)) % m
}

/// A Pedersen commitment: C = g^value * h^blinding mod p.
/// The commitment hides the value but preserves addition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Commitment {
    /// The commitment value (g^v * h^r mod p).
    point: u128,
}

impl Commitment {
    /// Create a commitment to a value with a blinding factor.
    /// The blinding factor hides the value — without it, you'd know the value.
    pub fn commit(value: u64, blinding: u64) -> Self {
        let gv = mod_pow(GENERATOR_G, value as u128, PRIME);
        let hr = mod_pow(GENERATOR_H, blinding as u128, PRIME);
        let point = mod_mul(gv, hr, PRIME);
        Self { point }
    }

    /// Verify a commitment opening: does C == commit(value, blinding)?
    pub fn verify(&self, value: u64, blinding: u64) -> bool {
        let expected = Self::commit(value, blinding);
        self.point == expected.point
    }

    /// Homomorphic addition: C(a,r1) + C(b,r2) = C(a+b, r1+r2).
    /// This is the CORE of conservation — addition is a group operation,
    /// not a runtime check.
    pub fn add(&self, other: &Commitment) -> Commitment {
        Commitment {
            point: mod_mul(self.point, other.point, PRIME),
        }
    }

    /// The identity element (commitment to zero with zero blinding).
    pub fn zero() -> Self {
        Self::commit(0, 0)
    }

    /// Raw commitment value (for comparison).
    pub fn raw(&self) -> u128 {
        self.point
    }
}

/// A balance proof: value + commitment + blinding.
/// The commitment can be verified without revealing the value to others.
/// Only the holder knows (value, blinding). Everyone can verify the commitment.
#[derive(Clone, Debug)]
pub struct BalanceProof {
    /// The Pedersen commitment (public).
    pub commitment: Commitment,
    /// The actual value (private to holder).
    value: u64,
    /// The blinding factor (private to holder).
    blinding: u64,
}

impl BalanceProof {
    /// Create a new balance proof.
    pub fn new(value: u64, blinding: u64) -> Self {
        let commitment = Commitment::commit(value, blinding);
        Self { commitment, value, blinding }
    }

    /// The hidden value (only accessible to the holder).
    pub fn value(&self) -> u64 {
        self.value
    }

    /// The blinding factor (only accessible to the holder).
    pub fn blinding(&self) -> u64 {
        self.blinding
    }

    /// Verify this proof is internally consistent.
    pub fn is_valid(&self) -> bool {
        self.commitment.verify(self.value, self.blinding)
    }
}

/// Verify that a transfer is balanced using ONLY commitments.
/// sum(input_commitments) == sum(output_commitments)
///
/// This is not a check — it's an algebraic identity.
/// If the commitments balance, the values MUST balance (or discrete log is broken).
pub fn verify_conservation(
    inputs: &[Commitment],
    outputs: &[Commitment],
) -> bool {
    let sum_in = inputs.iter().fold(Commitment::zero(), |acc, c| acc.add(c));
    let sum_out = outputs.iter().fold(Commitment::zero(), |acc, c| acc.add(c));
    sum_in == sum_out
}

// --- Hash Chain Seal (Entropy) ---

/// A crystallization seal: an append-only hash chain.
/// Each seal incorporates all previous history.
/// Reversing = inverting SHA-256 = impossible.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Seal {
    /// Current hash of the chain.
    hash: [u8; 32],
    /// Number of links in the chain.
    pub depth: u64,
}

impl Seal {
    /// Genesis seal — the first link.
    pub fn genesis(data: &[u8]) -> Self {
        let hash = Sha256::digest(data).into();
        Self { hash, depth: 0 }
    }

    /// Extend the chain with new evidence.
    /// S(n) = SHA-256(S(n-1) || evidence)
    /// This is irreversible: you cannot produce S(n-1) from S(n).
    pub fn extend(&self, evidence: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(self.hash);
        hasher.update(evidence);
        Self {
            hash: hasher.finalize().into(),
            depth: self.depth + 1,
        }
    }

    /// Verify that `child` is a valid extension of `self` with `evidence`.
    pub fn verify_extension(&self, evidence: &[u8], child: &Seal) -> bool {
        let expected = self.extend(evidence);
        expected == *child && child.depth == self.depth + 1
    }

    /// The raw hash (for external verification).
    pub fn hash(&self) -> &[u8; 32] {
        &self.hash
    }

    /// Hex representation.
    pub fn hex(&self) -> String {
        hex::encode(self.hash)
    }
}

// --- Causal Proof (Causality) ---

/// A causal proof: the event hash incorporates parent hashes.
/// You cannot construct a valid event without knowing the parents' content.
/// The hash IS the causal history.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CausalProof {
    /// Hash of (parent_hashes || origin || data).
    /// This hash encodes the ENTIRE causal ancestry.
    hash: [u8; 32],
}

impl CausalProof {
    /// Create a causal proof from parent proofs and event data.
    /// The resulting hash depends on ALL ancestors transitively.
    pub fn new(parent_proofs: &[&CausalProof], origin: &[u8], data: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        // Sorted parent hashes ensure deterministic proof regardless of order
        let mut parent_hashes: Vec<[u8; 32]> = parent_proofs.iter()
            .map(|p| p.hash)
            .collect();
        parent_hashes.sort();
        for h in &parent_hashes {
            hasher.update(h);
        }
        hasher.update(origin);
        hasher.update(data);
        Self { hash: hasher.finalize().into() }
    }

    /// Genesis proof — no parents.
    pub fn genesis(origin: &[u8], data: &[u8]) -> Self {
        Self::new(&[], origin, data)
    }

    /// Verify this proof was constructed from the given parents and data.
    pub fn verify(&self, parent_proofs: &[&CausalProof], origin: &[u8], data: &[u8]) -> bool {
        let expected = Self::new(parent_proofs, origin, data);
        self.hash == expected.hash
    }

    pub fn hash(&self) -> &[u8; 32] {
        &self.hash
    }

    pub fn short(&self) -> String {
        hex::encode(&self.hash[..4])
    }
}

impl std::fmt::Display for CausalProof {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode(&self.hash[..8]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Pedersen Commitment tests ---

    #[test]
    fn commitment_hides_value() {
        let c1 = Commitment::commit(100, 42);
        let c2 = Commitment::commit(100, 99);
        // Same value, different blinding → different commitments
        assert_ne!(c1, c2);
    }

    #[test]
    fn commitment_binds_value() {
        let c = Commitment::commit(100, 42);
        assert!(c.verify(100, 42));
        assert!(!c.verify(101, 42)); // wrong value
        assert!(!c.verify(100, 43)); // wrong blinding
    }

    #[test]
    fn homomorphic_addition_works() {
        let a = Commitment::commit(300, 10);
        let b = Commitment::commit(200, 20);
        let sum_ab = a.add(&b);

        let direct = Commitment::commit(500, 30);
        assert_eq!(sum_ab, direct, "C(300,10) + C(200,20) should equal C(500,30)");
    }

    #[test]
    fn conservation_verified_algebraically() {
        // Transfer: 300 from A, 200 from B → 250 to C, 250 to D
        let in1 = Commitment::commit(300, 10);
        let in2 = Commitment::commit(200, 20);
        let out1 = Commitment::commit(250, 15);
        let out2 = Commitment::commit(250, 15);

        assert!(
            verify_conservation(&[in1, in2], &[out1, out2]),
            "balanced transfer should verify"
        );
    }

    #[test]
    fn imbalanced_transfer_fails_algebraically() {
        let in1 = Commitment::commit(100, 10);
        let out1 = Commitment::commit(200, 10); // more than input!

        assert!(
            !verify_conservation(&[in1], &[out1]),
            "imbalanced transfer must not verify"
        );
    }

    #[test]
    fn conservation_with_different_blindings() {
        // Same values but blindings must also balance
        let in1 = Commitment::commit(500, 100);
        let out1 = Commitment::commit(300, 60);
        let out2 = Commitment::commit(200, 40);

        assert!(
            verify_conservation(&[in1], &[out1, out2]),
            "balanced values AND blindings should verify"
        );
    }

    #[test]
    fn conservation_fails_if_blindings_mismatch() {
        // Values balance but blindings don't
        let in1 = Commitment::commit(500, 100);
        let out1 = Commitment::commit(300, 50);
        let out2 = Commitment::commit(200, 40); // 50+40=90 != 100

        assert!(
            !verify_conservation(&[in1], &[out1, out2]),
            "mismatched blindings should fail even with balanced values"
        );
    }

    // --- Seal tests ---

    #[test]
    fn seal_chain_is_ordered() {
        let s0 = Seal::genesis(b"block_0");
        let s1 = s0.extend(b"block_1");
        let s2 = s1.extend(b"block_2");

        assert_eq!(s0.depth, 0);
        assert_eq!(s1.depth, 1);
        assert_eq!(s2.depth, 2);
        assert_ne!(s0.hash, s1.hash);
        assert_ne!(s1.hash, s2.hash);
    }

    #[test]
    fn seal_extension_is_verifiable() {
        let s0 = Seal::genesis(b"origin");
        let s1 = s0.extend(b"evidence_1");

        assert!(s0.verify_extension(b"evidence_1", &s1));
        assert!(!s0.verify_extension(b"tampered", &s1));
    }

    #[test]
    fn seal_cannot_be_forged() {
        let s0 = Seal::genesis(b"origin");
        let s1 = s0.extend(b"real_evidence");

        // Attacker tries to create alternative history
        let fake = s0.extend(b"fake_evidence");
        assert_ne!(s1, fake, "different evidence must produce different seal");
    }

    #[test]
    fn seal_is_irreversible() {
        let s0 = Seal::genesis(b"origin");
        let s1 = s0.extend(b"evidence");
        // You cannot derive s0 from s1 — that would require inverting SHA-256.
        // We can only verify this property exists by demonstrating
        // that s0 and s1 are structurally independent values.
        assert_ne!(s0.hash, s1.hash);
        assert_eq!(s1.depth, s0.depth + 1);
    }

    // --- Causal Proof tests ---

    #[test]
    fn causal_proof_encodes_ancestry() {
        let genesis = CausalProof::genesis(b"node_a", b"first_event");
        let child = CausalProof::new(&[&genesis], b"node_b", b"second_event");

        // Child proof depends on genesis — different genesis → different child
        let alt_genesis = CausalProof::genesis(b"node_a", b"different_event");
        let alt_child = CausalProof::new(&[&alt_genesis], b"node_b", b"second_event");

        assert_ne!(child, alt_child, "different ancestry must produce different proof");
    }

    #[test]
    fn causal_proof_is_verifiable() {
        let g = CausalProof::genesis(b"origin", b"data");
        let c = CausalProof::new(&[&g], b"origin2", b"data2");

        assert!(c.verify(&[&g], b"origin2", b"data2"));
        assert!(!c.verify(&[&g], b"origin2", b"tampered"));
    }

    #[test]
    fn causal_proof_order_independent() {
        let a = CausalProof::genesis(b"a", b"da");
        let b = CausalProof::genesis(b"b", b"db");

        let merge1 = CausalProof::new(&[&a, &b], b"m", b"dm");
        let merge2 = CausalProof::new(&[&b, &a], b"m", b"dm");

        assert_eq!(merge1, merge2, "parent order should not affect proof");
    }

    #[test]
    fn causal_proof_cannot_be_forged_without_parents() {
        let real_parent = CausalProof::genesis(b"real", b"data");
        let child = CausalProof::new(&[&real_parent], b"c", b"cd");

        let fake_parent = CausalProof::genesis(b"fake", b"data");
        assert!(
            !child.verify(&[&fake_parent], b"c", b"cd"),
            "forged parent must not verify"
        );
    }
}
