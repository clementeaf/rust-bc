//! Gravity — influence IS participation, computed not stored.
//!
//! Mass is not a number in a database. It is the count of causal events
//! connected to a participant. Like real gravity: mass IS the object,
//! not a label on it. You can't fake mass because mass IS the history
//! of participation in the causal graph.
//!
//! There is no registry to hack. There is no balance to forge.
//! Your mass is the answer to: "how many verified causal events
//! trace back to you?" That answer is computed, not stored.
//!
//! Properties:
//!   - **Computed, not stored**: mass is a pure function of the causal graph.
//!   - **Unforgeable**: faking mass = forging causal proofs = breaking SHA-256.
//!   - **Inverse-square**: influence decays with distance, preventing monopoly.
//!   - **Superposition**: multiple masses combine additively (like real gravity).
//!   - **No punishment**: inactivity doesn't destroy mass — others just grow.

use std::collections::HashMap;

use crate::causality::CausalGraph;
use crate::{Coord, Field, EPSILON};

/// Gravitational constant — controls how strongly mass curves the field.
const G_CONSTANT: f64 = 0.05;

/// Compute the mass of every participant directly from the causal graph.
/// Mass = number of events where the participant is the origin validator.
/// This is a pure function — no mutable state, no registry.
///
/// The returned map is ephemeral: it exists only for the duration of
/// the gravitational calculation. It cannot be tampered with because
/// it is recomputed from the graph every time.
pub fn compute_masses(graph: &CausalGraph) -> HashMap<Coord, f64> {
    let mut mass_by_origin: HashMap<Coord, f64> = HashMap::new();
    for event_id in graph.all_event_ids() {
        if let Some(event) = graph.event(event_id) {
            *mass_by_origin.entry(event.origin).or_default() += 1.0;
        }
    }
    mass_by_origin
}

/// Compute mass for a specific origin coordinate.
/// Pure function over the causal graph.
pub fn mass_at(graph: &CausalGraph, origin: Coord) -> f64 {
    graph.all_event_ids()
        .filter(|id| graph.event(id).map(|e| e.origin == origin).unwrap_or(false))
        .count() as f64
}

/// Gravitational influence from a mass source to a target coordinate.
/// F = G * m / (r² + 1)
/// Inverse-square with +1 to prevent singularity.
pub fn influence(source: Coord, source_mass: f64, target: Coord, field_size: usize) -> f64 {
    if source_mass <= 0.0 {
        return 0.0;
    }
    let dist = crate::distance(source, target, field_size);
    G_CONSTANT * source_mass / (dist * dist + 1.0)
}

/// Total gravitational pull at a coordinate from all mass sources.
/// Computed fresh from the causal graph — no cached state.
pub fn total_pull(graph: &CausalGraph, target: Coord, field_size: usize) -> f64 {
    let masses = compute_masses(graph);
    masses.iter()
        .map(|(source, mass)| influence(*source, *mass, target, field_size))
        .sum()
}

/// Relative weight of a coordinate — its fraction of total mass.
/// Range [0.0, 1.0].
pub fn relative_weight(graph: &CausalGraph, origin: Coord) -> f64 {
    let masses = compute_masses(graph);
    let total: f64 = masses.values().sum();
    if total == 0.0 {
        return 0.0;
    }
    masses.get(&origin).copied().unwrap_or(0.0) / total
}

/// Apply gravitational curvature to the field.
/// Each active cell gets a probability boost proportional to the
/// gravitational pull at its location. Mass curves the field.
///
/// Returns number of cells affected.
pub fn apply_curvature(graph: &CausalGraph, field: &mut Field) -> usize {
    let masses = compute_masses(graph);
    if masses.is_empty() {
        return 0;
    }

    let size = field.size;
    let coords: Vec<Coord> = field.active_entries()
        .map(|(coord, _)| coord)
        .collect();

    let mut affected = 0;
    for coord in coords {
        let pull: f64 = masses.iter()
            .map(|(source, mass)| influence(*source, *mass, coord, size))
            .sum();

        if pull < EPSILON {
            continue;
        }

        let cell = field.get_mut(coord);
        if !cell.crystallized {
            let boost = pull.min(0.1); // cap per-step
            cell.probability = (cell.probability + boost).min(1.0);
            affected += 1;
        }
    }

    affected
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::causality::{CausalEvent, CausalGraph};
    use crate::Dimension;

    fn coord(t: usize, c: usize, o: usize, v: usize) -> Coord {
        Coord { t, c, o, v }
    }

    fn build_graph_with_events(origins: &[(Coord, &[u8])]) -> CausalGraph {
        let mut graph = CausalGraph::new();
        let mut last_id: Option<crate::causality::EventId> = None;
        for (i, (origin, data)) in origins.iter().enumerate() {
            let parents = match &last_id {
                Some(id) => vec![id.clone()],
                None => vec![],
            };
            let event = CausalEvent::new(*origin, i as u64, parents, data.to_vec());
            let id = event.id.clone();
            graph.insert(event);
            last_id = Some(id);
        }
        graph
    }

    #[test]
    fn mass_computed_from_graph() {
        let graph = build_graph_with_events(&[
            (coord(5, 5, 5, 5), b"e1"),
            (coord(5, 5, 5, 5), b"e2"), // same origin
            (coord(3, 3, 3, 3), b"e3"), // different origin
        ]);

        let masses = compute_masses(&graph);
        assert_eq!(*masses.get(&coord(5, 5, 5, 5)).unwrap(), 2.0);
        assert_eq!(*masses.get(&coord(3, 3, 3, 3)).unwrap(), 1.0);
    }

    #[test]
    fn mass_at_specific_origin() {
        let graph = build_graph_with_events(&[
            (coord(5, 5, 5, 5), b"e1"),
            (coord(5, 5, 5, 5), b"e2"),
            (coord(3, 3, 3, 3), b"e3"),
        ]);

        assert_eq!(mass_at(&graph, coord(5, 5, 5, 5)), 2.0);
        assert_eq!(mass_at(&graph, coord(3, 3, 3, 3)), 1.0);
        assert_eq!(mass_at(&graph, coord(9, 9, 9, 9)), 0.0);
    }

    #[test]
    fn influence_decays_with_distance() {
        let source = coord(5, 5, 5, 5);
        let mass = 10.0;
        let size = 20;

        let near = influence(source, mass, coord(5, 5, 5, 5), size);
        let mid = influence(source, mass, coord(7, 5, 5, 5), size);
        let far = influence(source, mass, coord(10, 5, 5, 5), size);

        assert!(near > mid, "near ({near}) > mid ({mid})");
        assert!(mid > far, "mid ({mid}) > far ({far})");
    }

    #[test]
    fn relative_weight_proportional_to_participation() {
        // Alice: 3 events, Bob: 1 event
        let mut graph = CausalGraph::new();
        let g = CausalEvent::new(coord(5, 5, 5, 5), 0, vec![], b"g".to_vec());
        let gid = g.id.clone();
        graph.insert(g);

        let a1 = CausalEvent::new(coord(5, 5, 5, 5), 1, vec![gid.clone()], b"a1".to_vec());
        let a1id = a1.id.clone();
        graph.insert(a1);

        let a2 = CausalEvent::new(coord(5, 5, 5, 5), 2, vec![a1id.clone()], b"a2".to_vec());
        let a2id = a2.id.clone();
        graph.insert(a2);

        let b1 = CausalEvent::new(coord(3, 3, 3, 3), 3, vec![a2id], b"b1".to_vec());
        graph.insert(b1);

        // Alice's origin has 3 events, Bob's has 1
        let alice_w = relative_weight(&graph, coord(5, 5, 5, 5));
        let bob_w = relative_weight(&graph, coord(3, 3, 3, 3));

        assert!((alice_w - 0.75).abs() < 0.01, "alice ~75%, got {alice_w}");
        assert!((bob_w - 0.25).abs() < 0.01, "bob ~25%, got {bob_w}");
    }

    #[test]
    fn superposition_of_masses() {
        let graph = build_graph_with_events(&[
            (coord(3, 5, 5, 5), b"a1"),
            (coord(7, 5, 5, 5), b"b1"),
        ]);

        let midpoint = coord(5, 5, 5, 5);
        let total = total_pull(&graph, midpoint, 15);

        let masses = compute_masses(&graph);
        let pull_a = influence(coord(3, 5, 5, 5), *masses.get(&coord(3, 5, 5, 5)).unwrap(), midpoint, 15);
        let pull_b = influence(coord(7, 5, 5, 5), *masses.get(&coord(7, 5, 5, 5)).unwrap(), midpoint, 15);

        assert!(
            (total - (pull_a + pull_b)).abs() < 1e-10,
            "superposition: {total} vs {pull_a} + {pull_b}"
        );
    }

    #[test]
    fn empty_graph_no_gravity() {
        let graph = CausalGraph::new();
        let pull = total_pull(&graph, coord(5, 5, 5, 5), 10);
        assert_eq!(pull, 0.0);
        assert_eq!(relative_weight(&graph, coord(5, 5, 5, 5)), 0.0);
    }

    #[test]
    fn curvature_boosts_field_probability() {
        let mut field = Field::new(15);
        let center = coord(7, 7, 7, 7);
        field.attest(center, "ev1", Dimension::Temporal, "val_t");

        // Build graph with many events at center
        let mut graph = CausalGraph::new();
        let mut last_id = None;
        for i in 0..20u64 {
            let parents = last_id.iter().cloned().collect();
            let ev = CausalEvent::new(center, i, parents, format!("e{i}").into_bytes());
            last_id = Some(ev.id.clone());
            graph.insert(ev);
        }

        let affected = apply_curvature(&graph, &mut field);
        assert!(affected > 0, "should affect cells near high-mass region");
    }
}
