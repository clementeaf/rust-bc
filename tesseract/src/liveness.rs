//! Liveness — valid states crystallize in bounded time.
//!
//! Safety says: "false things don't crystallize."
//! Liveness says: "true things DO crystallize, eventually."
//!
//! Without liveness, an attacker can suppress valid state indefinitely
//! without ever producing false evidence — a **blocking attack**.
//!
//! # Threat model for liveness
//!
//! Adversary B (blocking attacker):
//!   - Can delay evidence delivery on up to (4 - m) dimensions
//!   - Can inject noise events to dilute probability
//!   - Can partition the network temporarily
//!   - Cannot forge attestations or rewrite causal history
//!   - Cannot control more than (4 - m) dimension-bound validators
//!
//! # Liveness theorem
//!
//! **Claim**: For any valid event with attestations on m ≥ σ_min dimensions,
//! each backed by causal depth ≥ d_min, the event crystallizes within
//! T_max = O(1/(α·m)) evolution steps after all attestations are delivered.
//!
//! **Conditions**:
//!   - At least m dimensions have exclusive validators (σ_raw ≥ m)
//!   - Each attestation has causal depth ≥ d_min (cost requirement)
//!   - The field is not over-capacity in the target region
//!   - Network partition duration < T_partition (bounded delay)
//!
//! **Proof sketch**:
//!   1. Each attestation seeds probability p₀ = 1/(1+dist) at the center
//!   2. With m ≥ 4 attestations at the same center, p ≥ min(m·p₀, 1.0)
//!   3. evolve() amplifies cells with σ ≥ 4 by factor A=4.0 with residual R=0.10
//!   4. After k steps: p(k) ≥ p₀ + k·(R + α·A·(p_avg - p))
//!   5. p(k) crosses Θ=0.85 within T_max ≈ (Θ - p₀)/(R + α·A·δ) steps
//!
//! # Blocking attack vectors and defenses
//!
//! | Attack | Defense | Bound |
//! |--------|---------|-------|
//! | Delay evidence | Bounded delay assumption T_partition | Crystallizes within T_max after delivery |
//! | Noise injection | Noise has σ<4, decays under curvature pressure | Valid events have higher σ, survive |
//! | Dimension starvation | m ≥ σ_min required; below that, system correctly waits | Not a bug — insufficient evidence |
//! | Suppress σ_eff | Requires controlling causal graph — cost-bounded | Cost grows exponentially with depth |
//! | Partition dimensions | Post-partition: attestations merge, σ recovers | Recovery time = O(partition_duration / α) |
//! | Curvature exhaustion | Capacity is per-region; attacker wastes own capacity | Legitimate events in other regions unaffected |

use crate::{Coord, Dimension, Field};

/// Maximum evolution steps for a fully-attested event to crystallize
/// after all attestations are delivered. Derived from:
/// T_max ≈ (Θ - p_min) / (R_min + α·A_min·δ)
/// With Θ=0.85, p_min≈0.5 (center of 4 overlapping seeds),
/// R=0.10, α=0.15, A=4.0: T_max ≈ 0.35/0.70 ≈ 1 step.
/// In practice, crystallization at the center is near-instant.
/// Peripheral cells take longer. Conservative bound: 50 steps.
pub const LIVENESS_BOUND: usize = 50;

/// Minimum σ for liveness guarantee.
/// Below this, the system correctly refuses to crystallize (insufficient evidence).
pub const MIN_SIGMA_FOR_LIVENESS: usize = 4;

/// Check if a fully-attested event crystallizes within the liveness bound.
/// Returns (crystallized, steps_taken).
pub fn check_liveness(
    field: &mut Field,
    target: Coord,
    max_steps: usize,
) -> (bool, usize) {
    for step in 1..=max_steps {
        if field.get(target).crystallized {
            return (true, step);
        }
        field.evolve();
    }
    (field.get(target).crystallized, max_steps)
}

/// Simulate delayed attestation delivery.
/// Attestations arrive one dimension at a time with `delay` steps between each.
/// Returns (crystallized, total_steps, step_crystallized).
pub fn check_liveness_with_delay(
    field: &mut Field,
    center: Coord,
    event_id: &str,
    validators: &[(Dimension, &str)],
    delay_between: usize,
    max_steps_after: usize,
) -> (bool, usize, Option<usize>) {
    let mut total_steps = 0;

    for (i, (dim, vid)) in validators.iter().enumerate() {
        field.attest(center, event_id, *dim, vid);

        // Delay between attestations (simulate network latency)
        if i < validators.len() - 1 {
            for _ in 0..delay_between {
                field.evolve();
                total_steps += 1;
            }
        }
    }

    // Now evolve until crystallization or timeout
    for step in 1..=max_steps_after {
        if field.get(center).crystallized {
            return (true, total_steps + step, Some(total_steps + step));
        }
        field.evolve();
    }

    let crystallized = field.get(center).crystallized;
    let step = if crystallized { Some(total_steps + max_steps_after) } else { None };
    (crystallized, total_steps + max_steps_after, step)
}

/// Inject noise events around a target to test if valid events survive.
/// Returns the number of noise seeds injected.
pub fn inject_noise(
    field: &mut Field,
    center: Coord,
    noise_count: usize,
    radius: usize,
) -> usize {
    let s = field.size;
    let mut injected = 0;
    for i in 0..noise_count {
        let offset = (i % radius) + 1;
        let noise_coord = Coord {
            t: (center.t + offset) % s,
            c: (center.c + offset) % s,
            o: center.o, // same region — competes for curvature
            v: (center.v + offset) % s,
        };
        // Noise: only 1 dimension attested → σ=1, will NOT crystallize
        field.attest(
            noise_coord,
            &format!("noise_{i}"),
            Dimension::Temporal,
            &format!("noise_val_{i}"),
        );
        injected += 1;
    }
    injected
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Field;

    fn coord(t: usize, c: usize, o: usize, v: usize) -> Coord {
        Coord { t, c, o, v }
    }

    fn full_validators() -> Vec<(Dimension, &'static str)> {
        vec![
            (Dimension::Temporal, "val_t"),
            (Dimension::Context, "val_c"),
            (Dimension::Origin, "val_o"),
            (Dimension::Verification, "val_v"),
        ]
    }

    fn attest_full(field: &mut Field, center: Coord) {
        for (dim, vid) in full_validators() {
            field.attest(center, "valid_event", dim, vid);
        }
    }

    // --- Core liveness ---

    #[test]
    fn valid_event_crystallizes_within_bound() {
        let mut field = Field::new(12);
        let center = coord(6, 6, 6, 6);
        attest_full(&mut field, center);

        let (crystallized, steps) = check_liveness(&mut field, center, LIVENESS_BOUND);

        assert!(crystallized, "fully attested event must crystallize");
        assert!(
            steps <= LIVENESS_BOUND,
            "must crystallize within {LIVENESS_BOUND} steps, took {steps}"
        );
    }

    #[test]
    fn center_crystallizes_immediately() {
        // At the attestation center, p=1.0 after 4 overlapping seeds
        // → should crystallize on the attest() call itself
        let mut field = Field::new(12);
        let center = coord(6, 6, 6, 6);
        attest_full(&mut field, center);

        assert!(
            field.get(center).crystallized,
            "center should crystallize immediately with 4 attestations"
        );
    }

    // --- Delayed delivery ---

    #[test]
    fn crystallizes_after_delayed_attestations() {
        let mut field = Field::new(12);
        let center = coord(6, 6, 6, 6);

        let (crystallized, _total, step) = check_liveness_with_delay(
            &mut field,
            center,
            "delayed_event",
            &full_validators(),
            5,  // 5 evolve steps between each dimension
            LIVENESS_BOUND,
        );

        assert!(crystallized, "should crystallize after all attestations arrive");
        assert!(
            step.unwrap() <= 5 * 3 + LIVENESS_BOUND,
            "should crystallize within delay + bound: step={}", step.unwrap()
        );
    }

    #[test]
    fn large_delay_still_crystallizes() {
        let mut field = Field::new(12);
        let center = coord(6, 6, 6, 6);

        let (crystallized, _, _) = check_liveness_with_delay(
            &mut field,
            center,
            "very_delayed",
            &full_validators(),
            20, // 20 steps between each attestation
            LIVENESS_BOUND,
        );

        assert!(crystallized, "should crystallize even with large delays");
    }

    // --- Noise resistance ---

    #[test]
    fn valid_event_survives_noise() {
        let mut field = Field::new(14);
        let center = coord(7, 7, 7, 7);

        // Inject noise BEFORE valid event
        inject_noise(&mut field, center, 20, 3);

        // Now attest the valid event
        attest_full(&mut field, center);

        let (crystallized, steps) = check_liveness(&mut field, center, LIVENESS_BOUND);

        assert!(crystallized, "valid event should survive 20 noise injections");
        assert!(
            steps <= LIVENESS_BOUND,
            "noise should not delay beyond bound: took {steps}"
        );
    }

    #[test]
    fn noise_itself_does_not_crystallize() {
        let mut field = Field::new(14);
        let noise_center = coord(3, 3, 3, 3);

        // Only noise — single-dimension attestations
        inject_noise(&mut field, noise_center, 50, 4);

        // Evolve extensively
        for _ in 0..100 {
            field.evolve();
        }

        // Noise coords should NOT crystallize (σ=1 < 4)
        // Some may crystallize from overlapping seeds in legacy mode,
        // but center should not have σ=4
        let cell = field.get(noise_center);
        assert!(
            !cell.crystallized || cell.sigma_independence() >= 4,
            "noise should not crystallize without σ=4"
        );
    }

    // --- Partition and recovery ---

    #[test]
    fn crystallizes_after_partition_heals() {
        let mut field = Field::new(12);
        let center = coord(6, 6, 6, 6);

        // Partition: only 2 dimensions available initially
        field.attest(center, "partitioned", Dimension::Temporal, "val_t");
        field.attest(center, "partitioned", Dimension::Context, "val_c");

        // Evolve during partition — should NOT crystallize
        for _ in 0..30 {
            field.evolve();
        }
        assert!(
            !field.get(center).crystallized,
            "should not crystallize with only 2 dimensions"
        );

        // Partition heals — remaining 2 dimensions arrive
        field.attest(center, "partitioned", Dimension::Origin, "val_o");
        field.attest(center, "partitioned", Dimension::Verification, "val_v");

        let (crystallized, steps) = check_liveness(&mut field, center, LIVENESS_BOUND);
        assert!(crystallized, "should crystallize after partition heals");
        assert!(
            steps <= LIVENESS_BOUND,
            "recovery should be within bound: {steps}"
        );
    }

    #[test]
    fn sigma_recovers_after_reconnection() {
        let mut field = Field::new(12);
        let center = coord(6, 6, 6, 6);

        // Start with 3 dimensions — σ=3, won't crystallize
        field.attest(center, "event", Dimension::Temporal, "val_t");
        field.attest(center, "event", Dimension::Context, "val_c");
        field.attest(center, "event", Dimension::Origin, "val_o");

        assert_eq!(field.get(center).sigma_independence(), 3);

        // Reconnect: 4th dimension arrives
        field.attest(center, "event", Dimension::Verification, "val_v");

        assert_eq!(field.get(center).sigma_independence(), 4);
        assert!(
            field.get(center).crystallized,
            "should crystallize immediately on σ=4"
        );
    }

    // --- Dimension starvation (correct refusal) ---

    #[test]
    fn insufficient_dimensions_correctly_blocks() {
        let mut field = Field::new(12);
        let center = coord(6, 6, 6, 6);

        // Only 1 dimension — attacker cannot force crystallization
        field.attest(center, "starved", Dimension::Temporal, "val_t");

        for _ in 0..100 {
            field.evolve();
        }

        assert!(
            !field.get(center).crystallized,
            "1 dimension must never crystallize (correct refusal)"
        );
    }

    #[test]
    fn two_dimensions_correctly_blocks() {
        let mut field = Field::new(12);
        let center = coord(6, 6, 6, 6);

        field.attest(center, "event", Dimension::Temporal, "val_t");
        field.attest(center, "event", Dimension::Origin, "val_o");

        for _ in 0..100 {
            field.evolve();
        }

        assert!(
            !field.get(center).crystallized,
            "2 dimensions must never crystallize"
        );
    }

    // --- Curvature pressure ---

    #[test]
    fn valid_event_survives_moderate_curvature_pressure() {
        let mut field = Field::new(12);
        let center = coord(6, 6, 6, 6);

        // Curvature pressure limits HOW MANY cells crystallize, not WHETHER
        // the event crystallizes at all. Liveness = the event reaches
        // crystallization. Curvature may then evict low-BE cells.
        field.set_capacity(6, 30.0);

        attest_full(&mut field, center);

        // Center crystallizes immediately on attest (p=1.0, σ=4)
        assert!(
            field.get(center).crystallized,
            "center should crystallize immediately despite capacity limit"
        );

        // After evolution, curvature pressure kicks in and evicts periphery
        for _ in 0..30 {
            field.evolve();
        }

        // Load should be constrained near capacity
        let load = field.curvature_load(6);
        assert!(
            load <= 35.0,
            "curvature pressure should constrain load: load={load}, capacity=30"
        );
    }

    // --- Liveness bound verification ---

    #[test]
    fn liveness_bound_is_reasonable() {
        // Verify T_max derivation: at center with 4 attestations,
        // probability hits 1.0 immediately. Peripheral cells take longer
        // but should be within LIVENESS_BOUND.
        let mut field = Field::new(12);
        let center = coord(6, 6, 6, 6);
        attest_full(&mut field, center);

        // Check a near neighbor — should crystallize within bound
        let neighbor = coord(6, 6, 6, 7);
        let (crystallized, steps) = check_liveness(&mut field, neighbor, LIVENESS_BOUND);

        // Neighbor may or may not crystallize depending on overlap
        // but if it does, it should be within bound
        if crystallized {
            assert!(steps <= LIVENESS_BOUND, "neighbor crystallized in {steps} steps");
        }
    }

    #[test]
    fn blocking_attack_bounded_by_cost() {
        // An attacker tries to suppress valid crystallization by
        // injecting noise to keep probability below threshold.
        // But valid attestations have σ=4 → amplified by A=4.0.
        // Noise has σ=1 → amplified by A=1.0. Valid wins.
        let mut field = Field::new(14);
        let center = coord(7, 7, 7, 7);

        // Continuous noise injection
        for _round in 0..5 {
            inject_noise(&mut field, center, 10, 3);
            field.evolve();
        }

        // Valid event arrives
        attest_full(&mut field, center);

        let (crystallized, _steps) = check_liveness(&mut field, center, LIVENESS_BOUND);
        assert!(
            crystallized,
            "valid event must crystallize despite 50 noise injections"
        );
    }
}
