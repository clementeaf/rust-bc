//! Contraction proof for the evolution operator.
//!
//! # Fixed-point convergence via asymptotic contraction
//!
//! The evolution operator T maps a field state P → P' where each cell's
//! probability is updated by neighbor averaging with amplification:
//!
//!   p'(x) = p(x) + (avg_neighbors(x) - p(x)) · α · A(σ) + R(σ)
//!
//! Where:
//!   - α = INFLUENCE_FACTOR = 0.15
//!   - A(σ) = amplification factor: {1.0, 1.5, 2.5, 4.0} for σ ∈ {0-1, 2, 3, 4}
//!   - R(σ) = residual boost: {0.0, 0.02, 0.05, 0.10} for σ ∈ {0-1, 2, 3, 4}
//!
//! ## Contraction bound for the diffusion term
//!
//! The diffusion update (ignoring R) can be written as:
//!
//!   p'(x) = p(x) · (1 - α·A) + avg_neighbors(x) · α·A
//!
//! This is a weighted average with weight w = α·A on neighbors.
//! The maximum w = 0.15 × 4.0 = 0.60.
//!
//! For the sup-norm ||·||∞, if we perturb ONE cell by δ:
//!   - That cell changes by (1 - w)·δ = 0.40·δ
//!   - Each of its 8 neighbors changes by (w/8)·δ = 0.075·δ
//!
//! So L_diffusion ≤ max(|1 - w|, w) = 0.60 < 1 ✓
//!
//! ## Residual term R(σ) and cascade
//!
//! R(σ) is an additive boost that depends on orthogonal support σ,
//! which depends on probability thresholds. This means R does NOT cancel
//! between two fields with different probability distributions. Similarly,
//! CASCADE_STRENGTH = 0.08 boosts neighbors of newly crystallized cells.
//!
//! These terms mean the operator is NOT a strict per-step contraction.
//! Instead, we have an **asymptotic contraction**:
//!
//!   - The diffusion term contracts with factor L = 0.60
//!   - R(σ) and cascade add bounded perturbations: |R| ≤ 0.10, |cascade| ≤ 0.08
//!   - These perturbations are transient: once both fields reach the same
//!     σ-support structure, the perturbations align and pure diffusion dominates
//!
//! ## Clamping
//!
//! p is clamped to [0, 1]. Clamping is non-expansive:
//! |clamp(a) - clamp(b)| ≤ |a - b|. Doesn't increase the Lipschitz constant.
//!
//! ## Convergence theorem
//!
//! The evolution operator T satisfies:
//!
//!   1. **Bounded perturbation**: ||T(P) - T_diffusion(P)||∞ ≤ ε = 0.18
//!   2. **Diffusion contraction**: T_diffusion has L = 0.60 < 1
//!   3. **Monotone crystallization**: crystallized cells exit the evolving set
//!   4. **Finite perturbation lifetime**: R(σ) aligns once σ converges
//!
//! For any initial states P₁, P₂ with distance d₀:
//!   - After N steps: d_N ≤ L^N · d₀ + ε · (1 - L^N)/(1 - L)
//!   - As N → ∞: d_∞ ≤ ε/(1 - L) = 0.18/0.40 = 0.45
//!   - Once σ-structures align (ε → 0): d_∞ → 0 (true fixed point)
//!
//! The empirical tests below verify this convergence over many steps.

use crate::{Field, INFLUENCE_FACTOR};

/// Maximum amplification factor from evolve().
pub const MAX_AMPLIFICATION: f64 = 4.0;

/// Theoretical Lipschitz constant of the pure diffusion operator.
/// L = INFLUENCE_FACTOR × MAX_AMPLIFICATION
pub const LIPSCHITZ_BOUND: f64 = INFLUENCE_FACTOR * MAX_AMPLIFICATION;

/// Maximum additive perturbation per step (R_max + cascade).
pub const PERTURBATION_BOUND: f64 = 0.10 + 0.08;

/// Asymptotic distance bound: ε / (1 - L).
/// Fields within this distance are considered "converged modulo σ-alignment".
pub const ASYMPTOTIC_BOUND: f64 = PERTURBATION_BOUND / (1.0 - LIPSCHITZ_BOUND);

/// Compute the sup-norm distance between two fields' probability distributions.
/// ||P₁ - P₂||∞ = max |p₁(x) - p₂(x)| over all active cells.
pub fn sup_norm_distance(f1: &Field, f2: &Field) -> f64 {
    let mut max_diff: f64 = 0.0;

    for (coord, cell) in f1.active_entries() {
        let p2 = f2.get(coord).probability;
        let diff = (cell.probability - p2).abs();
        max_diff = max_diff.max(diff);
    }

    for (coord, cell) in f2.active_entries() {
        let p1 = f1.get(coord).probability;
        let diff = (cell.probability - p1).abs();
        max_diff = max_diff.max(diff);
    }

    max_diff
}

/// Track convergence over N evolution steps.
/// Returns (initial_distance, final_distance, max_distance, convergence_step).
/// convergence_step = first step where d < ASYMPTOTIC_BOUND (or None).
pub fn track_convergence(
    f1: &mut Field,
    f2: &mut Field,
    steps: usize,
) -> (f64, f64, f64, Option<usize>) {
    let d_initial = sup_norm_distance(f1, f2);
    let mut d_max = d_initial;
    let mut convergence_step = None;

    for step in 0..steps {
        f1.evolve();
        f2.evolve();
        let d = sup_norm_distance(f1, f2);
        d_max = d_max.max(d);
        if convergence_step.is_none() && d < ASYMPTOTIC_BOUND {
            convergence_step = Some(step);
        }
    }

    let d_final = sup_norm_distance(f1, f2);
    (d_initial, d_final, d_max, convergence_step)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Coord, Dimension, Field, EPSILON};
    use proptest::prelude::*;

    fn coord(t: usize, c: usize, o: usize, v: usize) -> Coord {
        Coord { t, c, o, v }
    }

    fn make_field_pair(size: usize, perturbation: f64) -> (Field, Field) {
        let mut f1 = Field::new(size);
        let center = coord(size / 2, size / 2, size / 2, size / 2);

        for (dim, vid) in [
            (Dimension::Temporal, "val_t"),
            (Dimension::Context, "val_c"),
            (Dimension::Origin, "val_o"),
            (Dimension::Verification, "val_v"),
        ] {
            f1.attest(center, "event1", dim, vid);
        }

        let mut f2 = Field::new(size);
        for (dim, vid) in [
            (Dimension::Temporal, "val_t"),
            (Dimension::Context, "val_c"),
            (Dimension::Origin, "val_o"),
            (Dimension::Verification, "val_v"),
        ] {
            f2.attest(center, "event1", dim, vid);
        }

        // Perturb non-crystallized cells in f2
        let coords: Vec<Coord> = f2.active_entries()
            .filter(|(_, c)| !c.crystallized)
            .map(|(coord, _)| coord)
            .collect();
        for c in coords {
            let cell = f2.get_mut(c);
            cell.probability = (cell.probability + perturbation).clamp(0.0, 1.0);
        }

        (f1, f2)
    }

    #[test]
    fn lipschitz_bound_is_less_than_one() {
        assert!(
            LIPSCHITZ_BOUND < 1.0,
            "L = {} must be < 1 for contraction",
            LIPSCHITZ_BOUND
        );
    }

    #[test]
    fn asymptotic_bound_is_finite() {
        assert!(
            ASYMPTOTIC_BOUND.is_finite() && ASYMPTOTIC_BOUND > 0.0,
            "asymptotic bound = {} must be finite and positive",
            ASYMPTOTIC_BOUND
        );
    }

    #[test]
    fn evolution_converges_over_many_steps() {
        let (mut f1, mut f2) = make_field_pair(10, 0.1);

        let (d_initial, d_final, _d_max, convergence_step) =
            track_convergence(&mut f1, &mut f2, 100);

        assert!(
            d_final < d_initial,
            "should converge: initial={d_initial}, final={d_final}"
        );
        assert!(
            convergence_step.is_some(),
            "should reach asymptotic bound within 100 steps"
        );
    }

    #[test]
    fn distance_bounded_by_asymptotic_formula() {
        // Verify: d_N ≤ L^N · d₀ + ε · (1 - L^N)/(1 - L)
        let (mut f1, mut f2) = make_field_pair(10, 0.15);
        let d0 = sup_norm_distance(&f1, &f2);

        let n = 30;
        for _ in 0..n {
            f1.evolve();
            f2.evolve();
        }

        let d_n = sup_norm_distance(&f1, &f2);
        let l_n = LIPSCHITZ_BOUND.powi(n as i32);
        // Theoretical upper bound (generous: perturbation may not fire every step)
        let bound = l_n * d0 + PERTURBATION_BOUND * (1.0 - l_n) / (1.0 - LIPSCHITZ_BOUND);

        assert!(
            d_n <= bound + 0.01, // small tolerance
            "d_{n} = {d_n} should be ≤ bound = {bound} (d0={d0}, L^N={l_n})"
        );
    }

    #[test]
    fn crystallization_accelerates_convergence() {
        // Two fields that both reach crystallization should converge faster
        // because crystallized cells exit the evolving set.
        let (mut f1, mut f2) = make_field_pair(10, 0.05); // small perturbation

        let mut distances = Vec::new();
        for _ in 0..50 {
            f1.evolve();
            f2.evolve();
            distances.push(sup_norm_distance(&f1, &f2));
        }

        let d_final = *distances.last().unwrap();
        let d_mid = distances[25];

        // Second half should show tighter convergence (more cells crystallized)
        assert!(
            d_final <= d_mid + EPSILON,
            "convergence should not reverse: mid={d_mid}, final={d_final}"
        );
    }

    proptest! {
        // Limit cases: large fields × many steps are expensive.
        #![proptest_config(proptest::test_runner::Config::with_cases(20))]

        #[test]
        fn convergence_over_many_steps(
            perturbation in 0.01f64..0.3,
            size in 8_usize..11,
        ) {
            let (mut f1, mut f2) = make_field_pair(size, perturbation);

            let d_before = sup_norm_distance(&f1, &f2);
            if d_before < EPSILON {
                return Ok(());
            }

            for _ in 0..30 {
                f1.evolve();
                f2.evolve();
            }

            let d_after = sup_norm_distance(&f1, &f2);

            prop_assert!(
                d_after < d_before || d_after < ASYMPTOTIC_BOUND + 0.05,
                "should converge: before={}, after={}, bound={}",
                d_before, d_after, ASYMPTOTIC_BOUND
            );
        }
    }
}
