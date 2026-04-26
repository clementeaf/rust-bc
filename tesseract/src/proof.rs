//! Cryptographic primitives that make rule violations mathematically impossible.
//!
//! These are not checks — they are algebraic structures where invalid states
//! cannot be represented. Like how gravity isn't "enforced" by a validator,
//! these commitments are properties of the math itself.
//!
//! ## Pedersen Commitments (Conservation)
//!
//! A Pedersen commitment hides a value while preserving additive homomorphism:
//!   C(v, r) = v·G + r·H
//!
//! Key property: C(a, r1) + C(b, r2) = C(a+b, r1+r2)
//! This means: if sum(input_commits) == sum(output_commits), then
//! sum(input_values) == sum(output_values) — WITHOUT revealing the values.
//!
//! Breaking this requires solving the discrete logarithm problem on Curve25519.
//! 128-bit security level — computationally infeasible with known algorithms.
//!
//! ## Hash Chain Seals (Entropy)
//!
//! Each crystallization event produces: S(n) = SHA-256(S(n-1) || evidence)
//! Reversing requires inverting SHA-256 — preimage resistance.
//! The seal IS the history. You can't rewrite one link without
//! invalidating everything after it.

use sha2::{Digest, Sha256};

// --- Pedersen Commitment Scheme on Ristretto255 ---
//
// Using curve25519-dalek's RistrettoPoint for production-grade commitments.
// The Ristretto group provides a prime-order group from Curve25519,
// eliminating cofactor pitfalls.
//
// G = RISTRETTO_BASEPOINT_POINT (standard generator)
// H = hash-derived generator (nothing-up-my-sleeve, independent of G)
// Commitment: C = value·G + blinding·H
//
// Security: finding log_G(H) is the elliptic curve discrete log problem.
// 128-bit security — breaking requires ~2^128 operations.

use curve25519_dalek::constants::RISTRETTO_BASEPOINT_POINT;
use curve25519_dalek::ristretto::{CompressedRistretto, RistrettoPoint};
use curve25519_dalek::scalar::Scalar;

/// Generator G: the standard Ristretto basepoint.
const G: RistrettoPoint = RISTRETTO_BASEPOINT_POINT;

/// Generator H: derived from hashing to ensure nobody knows log_G(H).
/// This is a nothing-up-my-sleeve construction: H = hash_to_point("tesseract_pedersen_H").
fn generator_h() -> RistrettoPoint {
    use sha2::Sha512;
    let hash = Sha512::digest(b"tesseract_pedersen_H");
    let bytes: [u8; 64] = hash.into();
    RistrettoPoint::from_uniform_bytes(&bytes)
}

/// A Pedersen commitment: C = value·G + blinding·H.
/// The commitment hides the value but preserves addition.
#[derive(Clone, Debug)]
pub struct Commitment {
    /// The commitment point on the Ristretto group.
    point: RistrettoPoint,
}

impl PartialEq for Commitment {
    fn eq(&self, other: &Self) -> bool {
        self.point.compress() == other.point.compress()
    }
}

impl Eq for Commitment {}

impl Commitment {
    /// Create a commitment to a value with a blinding factor.
    /// C = value·G + blinding·H
    pub fn commit(value: u64, blinding: u64) -> Self {
        let v = Scalar::from(value);
        let r = Scalar::from(blinding);
        let point = v * G + r * generator_h();
        Self { point }
    }

    /// Verify a commitment opening: does C == commit(value, blinding)?
    pub fn verify(&self, value: u64, blinding: u64) -> bool {
        let expected = Self::commit(value, blinding);
        *self == expected
    }

    /// Homomorphic addition: C(a,r1) + C(b,r2) = C(a+b, r1+r2).
    /// This is the CORE of conservation — addition is a group operation,
    /// not a runtime check.
    pub fn add(&self, other: &Commitment) -> Commitment {
        Commitment {
            point: self.point + other.point,
        }
    }

    /// The identity element (commitment to zero with zero blinding).
    pub fn zero() -> Self {
        Self::commit(0, 0)
    }

    /// Compressed point bytes (32 bytes) for serialization/comparison.
    pub fn compressed(&self) -> CompressedRistretto {
        self.point.compress()
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
        Self {
            commitment,
            value,
            blinding,
        }
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
/// If the commitments balance, the values MUST balance (or ECDLP is broken).
pub fn verify_conservation(inputs: &[Commitment], outputs: &[Commitment]) -> bool {
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
        let mut parent_hashes: Vec<[u8; 32]> = parent_proofs.iter().map(|p| p.hash).collect();
        parent_hashes.sort();
        for h in &parent_hashes {
            hasher.update(h);
        }
        hasher.update(origin);
        hasher.update(data);
        Self {
            hash: hasher.finalize().into(),
        }
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

    // --- Pedersen Commitment tests (now on Ristretto255) ---

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
        assert_eq!(
            sum_ab, direct,
            "C(300,10) + C(200,20) should equal C(500,30)"
        );
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

    #[test]
    fn commitment_zero_is_identity() {
        let c = Commitment::commit(42, 7);
        let z = Commitment::zero();
        let sum = z.add(&c);
        assert_eq!(sum, c, "zero + C should equal C");
    }

    #[test]
    fn balance_proof_roundtrip() {
        let bp = BalanceProof::new(1000, 555);
        assert!(bp.is_valid());
        assert_eq!(bp.value(), 1000);
        assert_eq!(bp.blinding(), 555);
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
        assert_ne!(s0.hash, s1.hash);
        assert_eq!(s1.depth, s0.depth + 1);
    }

    // --- Causal Proof tests ---

    #[test]
    fn causal_proof_encodes_ancestry() {
        let genesis = CausalProof::genesis(b"node_a", b"first_event");
        let child = CausalProof::new(&[&genesis], b"node_b", b"second_event");

        let alt_genesis = CausalProof::genesis(b"node_a", b"different_event");
        let alt_child = CausalProof::new(&[&alt_genesis], b"node_b", b"second_event");

        assert_ne!(
            child, alt_child,
            "different ancestry must produce different proof"
        );
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
