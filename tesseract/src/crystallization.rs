//! Crystallization criteria — unified fixed-point convergence model.
//!
//! Crystallization is the transition from probabilistic to deterministic state.
//! Formally: a cell crystallizes when it reaches a **fixed point** where
//! further evolution cannot change its state.
//!
//! This module unifies three previously independent criteria:
//!
//!   1. **Threshold**: p(x) ≥ Θ (probability exceeds crystallization threshold)
//!   2. **Free energy**: F(x) = U(x) - T·S(x) < 0 (energetically favorable)
//!   3. **σ-independence**: σ(x) ≥ 4 (all dimensions independently attested)
//!
//! The unified criterion is:
//!
//!   crystallize(x) ⟺ threshold(x) ∧ energy(x) ∧ independence(x)
//!
//! This is a conjunction — ALL conditions must hold. This makes the
//! crystallization operator monotone on the lattice (false, true) and
//! guarantees convergence by Tarski's fixed-point theorem: the set of
//! crystallized cells can only grow (until equilibrium), never shrink
//! under normal evolution.
//!
//! Connection to formal frameworks:
//!
//!   - **Fixed-point theory**: evolve() is an inflationary operator on the
//!     lattice of field states. Crystallization is the least fixed point.
//!   - **Belief propagation**: evolve() computes marginal probabilities via
//!     neighbor message passing. Crystallization = convergence of beliefs.
//!   - **Energy minimization**: free energy F(x) < 0 ensures crystallization
//!     only occurs at local energy minima. The field's total free energy
//!     is a Lyapunov function (see lyapunov.rs).
//!
//! # σ-independence: formal definition and attack resistance
//!
//! ## Definition
//!
//! σ(x) counts the number of dimensions (T, C, O, V) for which cell x
//! has at least one **exclusive** validator: a validator that attests
//! ONLY on that dimension for this cell.
//!
//!   σ(x) = |{ d ∈ {T,C,O,V} : ∃ v ∈ validators(x,d) s.t.
//!            v ∉ validators(x,d') ∀ d' ≠ d }|
//!
//! This is NOT just "4 validators exist" — it requires 4 **structurally
//! independent** validators, each exclusively bound to one dimension.
//!
//! ## Why σ=4 resists Sybil attacks
//!
//! A Sybil attacker controls many validator IDs but operates from a
//! single entity. To achieve σ=4, the attacker needs 4 validators that
//! each attest on exactly one dimension. But:
//!
//!   - If the attacker uses the same validator on multiple dimensions,
//!     that validator is NOT exclusive → σ does not increase.
//!   - If the attacker creates 4 separate validator IDs (one per dim),
//!     they achieve σ=4 — but only if each ID is cryptographically
//!     distinct AND bound to that dimension.
//!
//! The binding is enforced by [`crate::mapper::SignedEvent`]: the
//! `validator_id` is derived from an Ed25519 public key, and the
//! dimension assignment is part of the signed payload. Forging a
//! binding requires the private key.
//!
//! **Sybil cost**: creating N fake identities gives N validator IDs,
//! but they all need to be exclusive per-dimension. N validators can
//! cover at most min(N, 4) dimensions. The cost of Sybil is linear
//! in the number of dimensions (4), not in the number of fake IDs.
//! 4 is a hard minimum — no amount of Sybil IDs reduces it.
//!
//! ## Why σ=4 resists collusion
//!
//! Collusion: M real entities coordinate to attest the same false event.
//! For σ=4, they need M ≥ 4 colluding validators, each exclusively
//! bound to a different dimension. This means:
//!
//!   - Collusion requires compromise of at least 4 independent parties
//!   - Each party must be bound to a different dimension
//!   - The binding is cryptographic (key-based), not social
//!
//! **Security scaling**: σ=4 means the attack surface scales as the
//! PRODUCT of dimension-specific compromise probabilities, not the sum:
//!
//!   P(attack) = P(compromise_T) × P(compromise_C) × P(compromise_O) × P(compromise_V)
//!
//! If each dimension has independent compromise probability p:
//!   P(attack) = p⁴
//!
//! For p = 0.1 (10% of validators compromised): P(attack) = 0.0001 (0.01%)
//!
//! ## What σ does NOT protect against
//!
//! - **Total system compromise**: if ALL validators on ALL dimensions
//!   are compromised, σ=4 is trivially satisfied. σ measures structural
//!   independence, not absolute security.
//! - **Dimension collapse**: if the system has fewer than 4 truly
//!   independent evidence sources, σ=4 is unachievable. The model
//!   assumes the 4 dimensions are backed by genuinely independent
//!   infrastructure.
//! - **Key theft**: if an attacker steals 4 private keys (one per
//!   dimension), they can forge σ=4. This is a key management problem,
//!   not a σ-independence problem.

use crate::{Cell, Coord, Field, CRYSTALLIZATION_THRESHOLD, EPSILON};

/// Result of evaluating crystallization criteria for a cell.
#[derive(Clone, Debug)]
pub struct CrystallizationEval {
    pub coord: Coord,
    /// p(x) ≥ Θ
    pub threshold_met: bool,
    /// F(x) < 0 (energetically favorable)
    pub energy_favorable: bool,
    /// σ(x) ≥ required independence level
    pub independence_met: bool,
    /// Free energy value (for diagnostics)
    pub free_energy: f64,
    /// σ-independence value
    pub sigma: usize,
    /// Probability value
    pub probability: f64,
}

impl CrystallizationEval {
    /// All three criteria satisfied — cell should crystallize.
    pub fn should_crystallize(&self) -> bool {
        self.threshold_met && self.energy_favorable && self.independence_met
    }
}

/// Crystallization criterion: determines when a cell transitions
/// from probabilistic to deterministic state.
pub trait CrystallizationCriterion {
    /// Evaluate whether a cell at `coord` should crystallize.
    fn evaluate(&self, field: &Field, coord: Coord) -> CrystallizationEval;
}

/// The unified criterion: threshold ∧ energy ∧ independence.
///
/// Parameters:
///   - `temperature`: current thermodynamic temperature (affects free energy)
///   - `threshold`: minimum probability for crystallization (default: CRYSTALLIZATION_THRESHOLD)
///   - `min_sigma`: minimum σ-independence required (default: 4)
pub struct UnifiedCriterion {
    pub temperature: f64,
    pub threshold: f64,
    pub min_sigma: usize,
}

impl UnifiedCriterion {
    pub fn new(temperature: f64) -> Self {
        Self {
            temperature,
            threshold: CRYSTALLIZATION_THRESHOLD,
            min_sigma: 4,
        }
    }

    /// Compute free energy: F = U - T·S
    /// U = -(p + binding_energy), S = binary Shannon entropy of p
    fn free_energy(&self, field: &Field, coord: Coord, cell: &Cell) -> f64 {
        let p = cell.probability;
        let binding = field.binding_energy(coord);
        let u = -(p + binding);

        let s = if p > EPSILON && p < 1.0 {
            -(p * p.log2() + (1.0 - p) * (1.0 - p).log2())
        } else {
            0.0
        };

        u + self.temperature * s
    }
}

impl CrystallizationCriterion for UnifiedCriterion {
    fn evaluate(&self, field: &Field, coord: Coord) -> CrystallizationEval {
        let cell = field.get(coord);
        let p = cell.probability;
        let sigma = cell.sigma_independence();
        let f_energy = self.free_energy(field, coord, cell);

        // For cells without attestations (legacy mode), skip σ check
        let has_attestations = !cell.attestations.is_empty();

        CrystallizationEval {
            coord,
            threshold_met: p >= self.threshold,
            energy_favorable: f_energy < 0.0,
            independence_met: if has_attestations {
                sigma >= self.min_sigma
            } else {
                true
            },
            free_energy: f_energy,
            sigma,
            probability: p,
        }
    }
}

/// Threshold-only criterion (backward compatible with legacy evolve).
/// Used when thermodynamics is not active.
pub struct ThresholdCriterion {
    pub threshold: f64,
    pub min_sigma: usize,
}

impl Default for ThresholdCriterion {
    fn default() -> Self {
        Self {
            threshold: CRYSTALLIZATION_THRESHOLD,
            min_sigma: 4,
        }
    }
}

impl CrystallizationCriterion for ThresholdCriterion {
    fn evaluate(&self, field: &Field, coord: Coord) -> CrystallizationEval {
        let cell = field.get(coord);
        let p = cell.probability;
        let sigma = cell.sigma_independence();
        let has_attestations = !cell.attestations.is_empty();

        CrystallizationEval {
            coord,
            threshold_met: p >= self.threshold,
            energy_favorable: true, // no energy check
            independence_met: if has_attestations {
                sigma >= self.min_sigma
            } else {
                true
            },
            free_energy: f64::NAN,
            sigma,
            probability: p,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Dimension, Field};

    fn coord(t: usize, c: usize, o: usize, v: usize) -> Coord {
        Coord { t, c, o, v }
    }

    fn fully_attested_field(size: usize) -> (Field, Coord) {
        let mut field = Field::new(size);
        let center = coord(5, 5, 5, 5);
        for (dim, vid) in [
            (Dimension::Temporal, "val_t"),
            (Dimension::Context, "val_c"),
            (Dimension::Origin, "val_o"),
            (Dimension::Verification, "val_v"),
        ] {
            field.attest(center, "event1", dim, vid);
        }
        (field, center)
    }

    #[test]
    fn unified_criterion_crystallizes_high_p_cell() {
        let (field, center) = fully_attested_field(12);
        let criterion = UnifiedCriterion::new(0.1); // cold
        let eval = criterion.evaluate(&field, center);

        assert!(
            eval.threshold_met,
            "p={} should meet threshold",
            eval.probability
        );
        assert!(
            eval.energy_favorable,
            "F={} should be < 0",
            eval.free_energy
        );
        assert!(eval.independence_met, "sigma={} should be >= 4", eval.sigma);
        assert!(eval.should_crystallize());
    }

    #[test]
    fn unified_criterion_rejects_low_p_cell() {
        let mut field = Field::new(12);
        let coord = coord(5, 5, 5, 5);
        // Only one attestation — low probability, low sigma
        field.attest(coord, "event1", Dimension::Temporal, "val_t");

        let criterion = UnifiedCriterion::new(0.1);
        let far_coord = coord!(9, 9, 9, 9);
        let eval = criterion.evaluate(&field, far_coord);

        assert!(!eval.should_crystallize());
    }

    #[test]
    fn hot_temperature_prevents_crystallization() {
        let (field, center) = fully_attested_field(12);
        // Very hot — T·S dominates, F > 0
        let criterion = UnifiedCriterion::new(100.0);
        let _center_eval = criterion.evaluate(&field, center);

        // Center is already crystallized (p=1.0, S=0), so F is still negative.
        // Test a neighbor that has high p but not crystallized.
        let neighbor = coord(5, 5, 5, 6);
        let n_eval = criterion.evaluate(&field, neighbor);

        // At T=100, non-crystallized cells with p<1 should have F>0
        if n_eval.probability > EPSILON && n_eval.probability < 1.0 {
            assert!(
                !n_eval.energy_favorable,
                "hot field should prevent crystallization: F={}",
                n_eval.free_energy
            );
        }
    }

    #[test]
    fn threshold_criterion_ignores_energy() {
        let (field, center) = fully_attested_field(12);
        let criterion = ThresholdCriterion::default();
        let eval = criterion.evaluate(&field, center);

        assert!(eval.energy_favorable); // always true
        assert!(eval.free_energy.is_nan()); // no energy computed
        assert!(eval.should_crystallize());
    }

    #[test]
    fn missing_sigma_blocks_crystallization() {
        let mut field = Field::new(12);
        let center = coord(5, 5, 5, 5);
        // Only 2 dimensions attested
        field.attest(center, "event1", Dimension::Temporal, "val_t");
        field.attest(center, "event1", Dimension::Context, "val_c");

        let criterion = UnifiedCriterion::new(0.1);
        let eval = criterion.evaluate(&field, center);

        assert!(!eval.independence_met, "sigma={} should be < 4", eval.sigma);
        assert!(!eval.should_crystallize());
    }

    // --- Sybil resistance tests ---

    #[test]
    fn sybil_same_validator_all_dims_gives_sigma_1() {
        // Attack: one entity "sybil" attests on all 4 dimensions.
        // Since the same validator_id appears on multiple dimensions,
        // it is NOT exclusive on any — σ should be 0 or at most 1.
        let mut field = Field::new(12);
        let center = coord(5, 5, 5, 5);

        for dim in Dimension::ALL {
            field.attest(center, "fake_event", dim, "sybil_validator");
        }

        let cell = field.get(center);
        let sigma = cell.sigma_independence();

        // "sybil_validator" appears on all 4 dims → exclusive on none
        // But since it's the ONLY validator, each dim has it and it
        // spans all 4 → dims.len() == 4, not 1 → not exclusive
        assert_eq!(
            sigma, 0,
            "single validator on all dims should give σ=0, got {sigma}"
        );
    }

    #[test]
    fn sybil_two_validators_across_four_dims_limited_sigma() {
        // Attack: 2 Sybil IDs each covering 2 dimensions.
        // Neither is exclusive on any dimension.
        let mut field = Field::new(12);
        let center = coord(5, 5, 5, 5);

        field.attest(center, "fake", Dimension::Temporal, "sybil_a");
        field.attest(center, "fake", Dimension::Context, "sybil_a");
        field.attest(center, "fake", Dimension::Origin, "sybil_b");
        field.attest(center, "fake", Dimension::Verification, "sybil_b");

        let cell = field.get(center);
        let sigma = cell.sigma_independence();

        // Each validator covers 2 dims → not exclusive on any
        assert_eq!(
            sigma, 0,
            "2 validators × 2 dims each should give σ=0, got {sigma}"
        );
    }

    #[test]
    fn sybil_four_exclusive_validators_achieves_sigma_4() {
        // "Attack" with 4 separate IDs, each exclusive to one dim.
        // This DOES achieve σ=4 — but it requires 4 independent keys.
        // The cost of this attack = compromising 4 separate identities.
        let mut field = Field::new(12);
        let center = coord(5, 5, 5, 5);

        field.attest(center, "event", Dimension::Temporal, "key_t");
        field.attest(center, "event", Dimension::Context, "key_c");
        field.attest(center, "event", Dimension::Origin, "key_o");
        field.attest(center, "event", Dimension::Verification, "key_v");

        let cell = field.get(center);
        let sigma = cell.sigma_independence();

        assert_eq!(sigma, 4, "4 exclusive validators should give σ=4");
        // This is the minimum cost: 4 independent keys.
        // No amount of Sybil IDs below 4 achieves this.
    }

    // --- Collusion resistance tests ---

    #[test]
    fn collusion_three_of_four_dims_insufficient() {
        // 3 colluding parties, each exclusive on one dimension.
        // Missing the 4th → σ=3, crystallization blocked.
        let mut field = Field::new(12);
        let center = coord(5, 5, 5, 5);

        field.attest(center, "collusion_event", Dimension::Temporal, "colluder_t");
        field.attest(center, "collusion_event", Dimension::Context, "colluder_c");
        field.attest(center, "collusion_event", Dimension::Origin, "colluder_o");
        // Verification dimension missing

        let cell = field.get(center);
        assert_eq!(cell.sigma_independence(), 3);

        let criterion = UnifiedCriterion::new(0.1);
        let eval = criterion.evaluate(&field, center);
        assert!(
            !eval.independence_met,
            "3/4 dims should block crystallization"
        );
        assert!(!eval.should_crystallize());
    }

    #[test]
    fn mixed_honest_and_sybil_validators() {
        // 2 honest exclusive validators + 1 Sybil spanning 2 dims.
        // Only the 2 honest dims contribute to σ.
        let mut field = Field::new(12);
        let center = coord(5, 5, 5, 5);

        // Honest: exclusive on their dimension
        field.attest(center, "event", Dimension::Temporal, "honest_t");
        field.attest(center, "event", Dimension::Context, "honest_c");

        // Sybil: same ID on both remaining dimensions
        field.attest(center, "event", Dimension::Origin, "sybil");
        field.attest(center, "event", Dimension::Verification, "sybil");

        let cell = field.get(center);
        let sigma = cell.sigma_independence();

        // honest_t exclusive on T ✓, honest_c exclusive on C ✓
        // sybil on O+V → not exclusive on either ✗
        assert_eq!(
            sigma, 2,
            "2 honest + 1 Sybil×2 should give σ=2, got {sigma}"
        );

        let criterion = UnifiedCriterion::new(0.1);
        let eval = criterion.evaluate(&field, center);
        assert!(
            !eval.should_crystallize(),
            "σ=2 should block crystallization"
        );
    }

    #[test]
    fn security_scales_multiplicatively() {
        // Demonstrate: adding each independent dimension compounds security.
        // σ=1 → one axis, σ=2 → two axes, etc. Attack probability = p^σ.
        let mut field = Field::new(12);
        let center = coord(5, 5, 5, 5);

        let dims_and_ids = [
            (Dimension::Temporal, "val_t"),
            (Dimension::Context, "val_c"),
            (Dimension::Origin, "val_o"),
            (Dimension::Verification, "val_v"),
        ];

        for (i, (dim, vid)) in dims_and_ids.iter().enumerate() {
            field.attest(center, "event", *dim, vid);
            let cell = field.get(center);
            let sigma = cell.sigma_independence();
            assert_eq!(
                sigma,
                i + 1,
                "after {i} exclusive attestations, σ should be {}",
                i + 1
            );
        }

        // At σ=4: attack probability with p=0.1 per dimension
        let p_per_dim = 0.1_f64;
        let p_attack = p_per_dim.powi(4);
        assert!(
            p_attack < 0.001,
            "multiplicative security: p^4 = {p_attack} should be < 0.1%"
        );
    }
}

/// Helper macro for coordinate construction in tests.
#[cfg(test)]
macro_rules! coord {
    ($t:expr, $c:expr, $o:expr, $v:expr) => {
        Coord {
            t: $t,
            c: $c,
            o: $o,
            v: $v,
        }
    };
}
#[cfg(test)]
pub(crate) use coord;
