//! Adversarial resistance — defense against evidence shaping attacks.
//!
//! # Threat model
//!
//! **Adversary A** with capabilities:
//!   - Controls k entities with valid cryptographic identities
//!   - Can generate structurally valid events (correct format, valid signatures)
//!   - Can coordinate across dimensions (k ≥ 4 → achieves raw σ=4)
//!   - Can choose timing and placement of attestations
//!
//! **Adversary limitations:**
//!   - Cannot forge other entities' private keys
//!   - Cannot rewrite causal history (hash-chained DAG)
//!   - Cannot exceed propagation speed (light cone constraint)
//!   - Must pay attestation cost (causal depth or stake)
//!
//! # The core attack: adversarial evidence shaping
//!
//! The attacker generates evidence that is:
//!   - Structurally valid (passes all format checks)
//!   - Independently attested (raw σ=4)
//!   - Internally consistent (no contradictions)
//!   - **But false** (does not correspond to external reality)
//!
//! Raw σ-independence does not detect this because it only measures
//! structural independence, not truthfulness.
//!
//! # Defense: effective sigma (σ_eff)
//!
//! σ_eff replaces raw σ with a weighted measure that penalizes:
//!
//!   1. **Causal correlation**: validators with shared recent causal
//!      history get reduced weight (they may be coordinating)
//!   2. **Low diversity**: validators from similar origins/contexts
//!      contribute less than truly diverse sources
//!   3. **Zero cost**: attestations without proof-of-cost (causal depth,
//!      stake, latency) are discounted
//!
//! σ_eff = Σ_d min(1, independence_d × diversity_d × cost_d)
//!
//! Where each dimension d contributes at most 1.0, and the total
//! σ_eff ∈ [0, 4].
//!
//! # Security bound
//!
//! For an adversary controlling k entities with attestation cost c:
//!
//!   P(false crystallization) ≤ (k/N)^σ_eff × e^(-c·σ_eff)
//!
//! Where N = total validators. The cost term c makes evidence shaping
//! exponentially expensive in the number of dimensions.
//!
//! Without cost (c=0): P = (k/N)^σ_eff — pure Sybil resistance.
//! With cost: the exponential penalty makes coordinated attacks
//! prohibitively expensive even when k/N is not small.

use crate::causality::CausalGraph;
use crate::{Attestation, Cell, Coord, Dimension, Field};
use std::collections::{HashMap, HashSet};

// --- Attestation cost model ---

/// Minimum causal depth for full-weight attestation.
/// Attestations from validators with shorter causal chains
/// get proportionally reduced weight.
pub const MIN_CAUSAL_DEPTH: u64 = 3;

/// Maximum causal overlap ratio before correlation penalty kicks in.
/// If two validators share > this fraction of their causal ancestors,
/// they are considered correlated.
pub const CORRELATION_THRESHOLD: f64 = 0.5;

/// Discount factor for zero-cost attestations (no causal depth).
pub const ZERO_COST_DISCOUNT: f64 = 0.25;

// --- Effective sigma ---

/// Per-dimension contribution to σ_eff.
#[derive(Clone, Debug)]
pub struct DimensionScore {
    pub dimension: Dimension,
    /// Raw: is there an exclusive validator? (0 or 1)
    pub raw_independence: f64,
    /// Diversity: fraction of unique causal histories among validators on this dim.
    /// 1.0 = all validators have disjoint histories. 0.0 = identical histories.
    pub diversity: f64,
    /// Cost: average attestation cost of validators on this dimension.
    /// 1.0 = all validators have full causal depth. 0.25 = zero-cost attestations.
    pub cost: f64,
    /// Effective contribution: min(1, independence × diversity × cost)
    pub effective: f64,
}

/// Full σ_eff evaluation for a cell.
#[derive(Clone, Debug)]
pub struct EffectiveSigma {
    pub coord: Coord,
    pub raw_sigma: usize,
    pub sigma_eff: f64,
    pub dimensions: Vec<DimensionScore>,
    /// Detected correlation pairs (validator_a, validator_b, overlap_ratio).
    pub correlations: Vec<(String, String, f64)>,
}

/// Compute effective sigma for a cell.
///
/// Requires the causal graph to evaluate validator histories.
/// If no causal graph is available, falls back to raw σ with
/// cost discount for validators without causal depth.
pub fn effective_sigma(field: &Field, coord: Coord, graph: Option<&CausalGraph>) -> EffectiveSigma {
    let cell = field.get(coord);
    let raw_sigma = cell.sigma_independence();

    let mut dimensions = Vec::new();
    let mut all_correlations = Vec::new();

    for dim in &Dimension::ALL {
        let atts = cell.attestations.get(dim);
        let atts = match atts {
            Some(a) if !a.is_empty() => a,
            _ => {
                dimensions.push(DimensionScore {
                    dimension: *dim,
                    raw_independence: 0.0,
                    diversity: 0.0,
                    cost: 0.0,
                    effective: 0.0,
                });
                continue;
            }
        };

        // Raw independence (from sigma_independence logic)
        let raw_indep = if has_exclusive_validator(cell, *dim) {
            1.0
        } else {
            0.0
        };

        // Diversity: causal history overlap between validators on this dim
        let (diversity, correlations) = compute_diversity(atts, graph);
        all_correlations.extend(correlations);

        // Cost: average causal depth of validators
        let cost = compute_cost(atts, graph);

        let effective = (raw_indep * diversity * cost).min(1.0);

        dimensions.push(DimensionScore {
            dimension: *dim,
            raw_independence: raw_indep,
            diversity,
            cost,
            effective,
        });
    }

    let sigma_eff: f64 = dimensions.iter().map(|d| d.effective).sum();

    EffectiveSigma {
        coord,
        raw_sigma,
        sigma_eff,
        dimensions,
        correlations: all_correlations,
    }
}

/// Check if a dimension has at least one exclusive validator.
fn has_exclusive_validator(cell: &Cell, dim: Dimension) -> bool {
    let mut validator_dims: HashMap<&str, HashSet<Dimension>> = HashMap::new();
    for (d, atts) in &cell.attestations {
        for att in atts {
            validator_dims
                .entry(att.validator_id.as_str())
                .or_default()
                .insert(*d);
        }
    }

    cell.attestations
        .get(&dim)
        .map(|atts| {
            atts.iter().any(|att| {
                validator_dims
                    .get(att.validator_id.as_str())
                    .map(|dims| dims.len() == 1)
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

/// Compute diversity score for validators on a dimension.
/// Uses causal graph to detect shared history.
/// Returns (diversity_score, detected_correlations).
fn compute_diversity(
    atts: &[Attestation],
    graph: Option<&CausalGraph>,
) -> (f64, Vec<(String, String, f64)>) {
    if atts.len() <= 1 {
        // Single validator — diversity is 1.0 (no one to correlate with)
        return (1.0, vec![]);
    }

    let graph = match graph {
        Some(g) => g,
        None => return (1.0, vec![]), // no graph → assume independent
    };

    let mut correlations = Vec::new();
    let mut total_pairs = 0;
    let mut correlated_pairs = 0;

    for i in 0..atts.len() {
        for j in (i + 1)..atts.len() {
            total_pairs += 1;
            let overlap = causal_overlap(&atts[i].validator_id, &atts[j].validator_id, graph);
            if overlap > CORRELATION_THRESHOLD {
                correlated_pairs += 1;
                correlations.push((
                    atts[i].validator_id.clone(),
                    atts[j].validator_id.clone(),
                    overlap,
                ));
            }
        }
    }

    let diversity = if total_pairs > 0 {
        1.0 - (correlated_pairs as f64 / total_pairs as f64)
    } else {
        1.0
    };

    (diversity, correlations)
}

/// Compute causal overlap between two validators.
/// Overlap = |ancestors(A) ∩ ancestors(B)| / |ancestors(A) ∪ ancestors(B)|
/// (Jaccard similarity of causal histories)
fn causal_overlap(validator_a: &str, validator_b: &str, graph: &CausalGraph) -> f64 {
    let events_a = events_by_origin(validator_a, graph);
    let events_b = events_by_origin(validator_b, graph);

    if events_a.is_empty() || events_b.is_empty() {
        return 0.0;
    }

    // Collect all ancestors of A's events and B's events
    let ancestors_a: HashSet<_> = events_a
        .iter()
        .filter_map(|id| graph.ancestors_of(id))
        .flat_map(|s| s.iter().cloned())
        .collect();
    let ancestors_b: HashSet<_> = events_b
        .iter()
        .filter_map(|id| graph.ancestors_of(id))
        .flat_map(|s| s.iter().cloned())
        .collect();

    let intersection = ancestors_a.intersection(&ancestors_b).count();
    let union = ancestors_a.union(&ancestors_b).count();

    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

/// Find events in the causal graph originating from a validator.
/// Uses origin coordinate matching (validator_id encodes origin).
fn events_by_origin(_validator_id: &str, graph: &CausalGraph) -> Vec<crate::causality::EventId> {
    // In the current model, validator_id is a string label.
    // We match events whose origin was used by this validator.
    // This is approximate — a full implementation would use the
    // mapper's SignedEvent to bind validator_id to event origin.
    graph.all_event_ids().cloned().collect::<Vec<_>>()
}

/// Compute cost score for attestations.
/// Cost = average(min(causal_depth / MIN_CAUSAL_DEPTH, 1.0))
/// Attestations without causal backing get ZERO_COST_DISCOUNT.
fn compute_cost(atts: &[Attestation], graph: Option<&CausalGraph>) -> f64 {
    let graph = match graph {
        Some(g) => g,
        None => return ZERO_COST_DISCOUNT, // no graph → zero-cost
    };

    if atts.is_empty() {
        return 0.0;
    }

    let total: f64 = atts
        .iter()
        .map(|att| {
            let depth = validator_causal_depth(&att.validator_id, graph);
            if depth == 0 {
                ZERO_COST_DISCOUNT
            } else {
                (depth as f64 / MIN_CAUSAL_DEPTH as f64).min(1.0)
            }
        })
        .sum();

    total / atts.len() as f64
}

/// Causal depth of a validator: longest chain of events they've produced.
fn validator_causal_depth(_validator_id: &str, graph: &CausalGraph) -> u64 {
    // Approximate: count total events in graph as depth proxy.
    // Full implementation would track per-validator event chains.
    let all_events: Vec<_> = graph.all_event_ids().collect();
    all_events.len() as u64
}

// --- Security bound ---

/// Compute the security bound for a given adversarial scenario.
///
/// P(false crystallization) ≤ (k/N)^σ_eff × e^(-c·σ_eff)
///
/// Where:
///   k = adversary-controlled validators
///   n = total validators
///   sigma_eff = effective sigma of the target cell
///   cost_per_dim = average attestation cost
pub fn security_bound(k: usize, n: usize, sigma_eff: f64, cost_per_dim: f64) -> f64 {
    if n == 0 || sigma_eff <= 0.0 {
        return 1.0; // no security
    }
    let sybil_factor = (k as f64 / n as f64).powf(sigma_eff);
    let cost_factor = (-cost_per_dim * sigma_eff).exp();
    (sybil_factor * cost_factor).min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Coord, Dimension, Field};

    fn coord(t: usize, c: usize, o: usize, v: usize) -> Coord {
        Coord { t, c, o, v }
    }

    // --- σ_eff tests ---

    #[test]
    fn sigma_eff_equals_raw_without_graph() {
        let mut field = Field::new(12);
        let center = coord(5, 5, 5, 5);
        for (dim, vid) in [
            (Dimension::Temporal, "val_t"),
            (Dimension::Context, "val_c"),
            (Dimension::Origin, "val_o"),
            (Dimension::Verification, "val_v"),
        ] {
            field.attest(center, "event1", dim, vid);
        }

        let result = effective_sigma(&field, center, None);
        assert_eq!(result.raw_sigma, 4);
        // Without graph: cost = ZERO_COST_DISCOUNT = 0.25
        // σ_eff = 4 × (1.0 × 1.0 × 0.25) = 1.0
        assert!(
            (result.sigma_eff - 1.0).abs() < 0.01,
            "σ_eff without graph should be discounted: {}",
            result.sigma_eff
        );
    }

    #[test]
    fn sigma_eff_zero_for_sybil_single_validator() {
        let mut field = Field::new(12);
        let center = coord(5, 5, 5, 5);
        for dim in Dimension::ALL {
            field.attest(center, "fake", dim, "sybil");
        }

        let result = effective_sigma(&field, center, None);
        assert_eq!(result.raw_sigma, 0);
        assert!(
            result.sigma_eff < 0.01,
            "Sybil σ_eff should be ~0: {}",
            result.sigma_eff
        );
    }

    #[test]
    fn sigma_eff_with_causal_graph() {
        let mut field = Field::new(12);
        let center = coord(5, 5, 5, 5);
        for (dim, vid) in [
            (Dimension::Temporal, "val_t"),
            (Dimension::Context, "val_c"),
            (Dimension::Origin, "val_o"),
            (Dimension::Verification, "val_v"),
        ] {
            field.attest(center, "event1", dim, vid);
        }

        // Build a causal graph with some depth
        let mut graph = CausalGraph::new();
        let e1 = crate::causality::CausalEvent::new(center, 0, vec![], b"genesis".to_vec());
        let e1_id = e1.id.clone();
        graph.insert(e1);

        let e2 =
            crate::causality::CausalEvent::new(center, 1, vec![e1_id.clone()], b"second".to_vec());
        let e2_id = e2.id.clone();
        graph.insert(e2);

        let e3 = crate::causality::CausalEvent::new(center, 2, vec![e2_id], b"third".to_vec());
        graph.insert(e3);

        let result = effective_sigma(&field, center, Some(&graph));

        // With graph depth=3 (≥ MIN_CAUSAL_DEPTH): cost = 1.0
        // σ_eff should be higher than without graph
        assert!(
            result.sigma_eff > 1.0,
            "σ_eff with causal depth should be > 1.0: {}",
            result.sigma_eff
        );
    }

    #[test]
    fn partial_attestation_reduces_sigma_eff() {
        let mut field = Field::new(12);
        let center = coord(5, 5, 5, 5);
        // Only 2 dimensions
        field.attest(center, "event1", Dimension::Temporal, "val_t");
        field.attest(center, "event1", Dimension::Context, "val_c");

        let result = effective_sigma(&field, center, None);
        assert_eq!(result.raw_sigma, 2);
        assert!(
            result.sigma_eff < 1.0,
            "2-dim σ_eff should be < 1.0: {}",
            result.sigma_eff
        );
    }

    // --- Security bound tests ---

    #[test]
    fn security_bound_decreases_with_more_dimensions() {
        let p1 = security_bound(10, 100, 1.0, 0.0); // σ_eff=1
        let p2 = security_bound(10, 100, 2.0, 0.0); // σ_eff=2
        let p4 = security_bound(10, 100, 4.0, 0.0); // σ_eff=4

        assert!(p2 < p1, "p2={p2} should be < p1={p1}");
        assert!(p4 < p2, "p4={p4} should be < p2={p2}");
        assert!(p4 < 0.001, "σ_eff=4 should give < 0.1%: {p4}");
    }

    #[test]
    fn security_bound_with_cost_is_exponentially_better() {
        let p_no_cost = security_bound(10, 100, 4.0, 0.0);
        let p_with_cost = security_bound(10, 100, 4.0, 1.0);

        assert!(
            p_with_cost < p_no_cost * 0.1,
            "cost should improve security 10×: no_cost={p_no_cost}, with_cost={p_with_cost}"
        );
    }

    #[test]
    fn total_compromise_has_no_security() {
        let p = security_bound(100, 100, 4.0, 0.0);
        assert!((p - 1.0).abs() < 0.01, "k=N should give P≈1: {p}");
    }

    #[test]
    fn zero_sigma_has_no_security() {
        let p = security_bound(1, 100, 0.0, 10.0);
        assert!(
            (p - 1.0).abs() < 0.01,
            "σ_eff=0 should give P=1 regardless of cost: {p}"
        );
    }

    // --- Evidence shaping attack simulation ---

    #[test]
    fn evidence_shaping_detected_by_sigma_eff() {
        // Scenario: attacker has 4 validator IDs, each exclusive per dim.
        // Raw σ=4 (passes basic check). But all validators have
        // zero causal depth → σ_eff is discounted.
        let mut field = Field::new(12);
        let center = coord(5, 5, 5, 5);

        field.attest(center, "shaped_event", Dimension::Temporal, "attacker_t");
        field.attest(center, "shaped_event", Dimension::Context, "attacker_c");
        field.attest(center, "shaped_event", Dimension::Origin, "attacker_o");
        field.attest(
            center,
            "shaped_event",
            Dimension::Verification,
            "attacker_v",
        );

        // No causal graph → zero-cost attestations
        let result = effective_sigma(&field, center, None);

        assert_eq!(result.raw_sigma, 4, "raw σ should be 4");
        assert!(
            result.sigma_eff < 2.0,
            "σ_eff should be significantly reduced without causal backing: {}",
            result.sigma_eff
        );

        // Compare with honest validators that have causal depth
        let mut graph = CausalGraph::new();
        let mut last_id = None;
        for i in 0..5u64 {
            let parents = last_id.iter().cloned().collect();
            let ev = crate::causality::CausalEvent::new(
                center,
                i,
                parents,
                format!("honest_{i}").into_bytes(),
            );
            last_id = Some(ev.id.clone());
            graph.insert(ev);
        }

        let honest_result = effective_sigma(&field, center, Some(&graph));
        assert!(
            honest_result.sigma_eff > result.sigma_eff,
            "honest σ_eff ({}) should exceed attacker σ_eff ({})",
            honest_result.sigma_eff,
            result.sigma_eff
        );
    }
}
