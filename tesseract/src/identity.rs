//! Identity and Sybil resistance through geometric weight.
//!
//! A participant's "region" is not assigned — it emerges from their
//! interactions with other participants. An identity with no real
//! interactions has an empty region with zero geometric weight.
//!
//! Sybil resistance: creating 1000 fake identities gives you 1000
//! empty regions. Empty regions have no crystallizations, no orbital
//! overlap, no binding energy. They are geometrically meaningless.
//!
//! Your weight in the field = how much the space has crystallized
//! around your coordinate through REAL interactions with others.
//!
//! **Cryptographic binding:** Identity spoofing is prevented by
//! [`mapper::SignedEvent`], which derives `org` from the signer's
//! Ed25519 public key. An attacker cannot claim another identity's
//! org without possessing their private key. Combined with geometric
//! weight, Sybil resistance is both cryptographic AND geometric.

use crate::Field;

/// Geometric weight of a participant in the field.
/// This is NOT a balance. It's a measure of how real and active
/// this identity is — how much the space has deformed around it
/// through interactions with other real participants.
#[derive(Debug, Clone)]
pub struct GeometricWeight {
    /// Number of crystallized cells in this identity's region.
    pub crystallized_cells: usize,
    /// Average binding energy of those cells.
    pub avg_binding_energy: f64,
    /// Number of distinct external influences (other identities
    /// that contributed probability to this region).
    pub external_influences: usize,
    /// Composite weight: the product of all factors.
    /// A Sybil identity scores 0 on all three → weight = 0.
    pub weight: f64,
}

/// Measure the geometric weight of an identity in the field.
/// The identity's region is defined by its o-axis coordinate.
pub fn geometric_weight(field: &Field, region: usize) -> GeometricWeight {
    let mut crystallized_cells = 0_usize;
    let mut total_crystal_neighbors = 0.0_f64;
    let mut external_event_ids = std::collections::HashSet::new();

    for (coord, cell) in field.active_entries() {
        if coord.o != region {
            continue;
        }
        if !cell.crystallized {
            continue;
        }

        crystallized_cells += 1;

        // Structural support: how many crystallized neighbors anchor this cell
        let cn = field
            .neighbors(coord)
            .iter()
            .filter(|n| field.get(**n).crystallized)
            .count() as f64;
        total_crystal_neighbors += cn / 8.0;

        // Count distinct event sources (diversity of interactions)
        for inf in &cell.influences {
            external_event_ids.insert(inf.event_id.clone());
        }
    }

    let avg_binding_energy = if crystallized_cells > 0 {
        total_crystal_neighbors / crystallized_cells as f64
    } else {
        0.0
    };

    let external_influences = external_event_ids.len();

    // Composite weight: crystallized density × structural support × source diversity.
    // A Sybil with no real interactions has 0 external influences → weight = 0.
    let weight = crystallized_cells as f64 * avg_binding_energy * external_influences as f64;

    GeometricWeight {
        crystallized_cells,
        avg_binding_energy,
        external_influences,
        weight,
    }
}

/// Check if an identity has enough geometric weight to seed
/// a deformation of a given cost.
///
/// This is NOT a lock or pre-validation. It's a QUERY.
/// The field still accepts any seed. But the application layer
/// can use this to warn: "this identity has no weight — the
/// deformation will likely decay under curvature pressure."
pub fn can_sustain_deformation(field: &Field, region: usize, cost: f64) -> bool {
    let w = geometric_weight(field, region);
    w.weight >= cost
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;

    #[test]
    fn new_identity_has_zero_weight() {
        let field = Field::new(8);
        let w = geometric_weight(&field, 5);
        assert_eq!(w.crystallized_cells, 0);
        assert_eq!(w.weight, 0.0);
    }

    #[test]
    fn active_identity_has_weight() {
        let mut field = Field::new(16);
        // Simulate real activity: events from NEARBY but DISTINCT sources.
        // Seeds must overlap (within SEED_RADIUS) for emergent crystallization,
        // but come from different event IDs for source diversity.
        field.seed_named(
            Coord {
                t: 5,
                c: 5,
                o: 5,
                v: 5,
            },
            "alice-tx1",
        );
        field.seed_named(
            Coord {
                t: 7,
                c: 5,
                o: 5,
                v: 5,
            },
            "bob-pays-alice",
        );
        field.seed_named(
            Coord {
                t: 5,
                c: 7,
                o: 5,
                v: 5,
            },
            "carol-pays-alice",
        );
        field.seed_named(
            Coord {
                t: 5,
                c: 5,
                o: 7,
                v: 5,
            },
            "dave-pays-alice",
        );
        evolve_to_equilibrium(&mut field, 10);

        let w = geometric_weight(&field, 5);
        assert!(
            w.crystallized_cells > 0,
            "Active identity should have crystallizations"
        );
        assert!(w.external_influences > 0, "Should have external influences");
        assert!(w.weight > 0.0, "Composite weight should be positive");
    }

    #[test]
    fn sybil_identity_has_no_weight() {
        let mut field = Field::new(16);

        // Real identity: Alice (region 3) interacts with Bob (region 4)
        field.seed_named(
            Coord {
                t: 2,
                c: 3,
                o: 3,
                v: 3,
            },
            "alice-real-tx",
        );
        field.seed_named(
            Coord {
                t: 3,
                c: 3,
                o: 3,
                v: 3,
            },
            "bob-interacts-alice",
        );
        field.seed_named(
            Coord {
                t: 4,
                c: 3,
                o: 3,
                v: 3,
            },
            "carol-pays-alice",
        );
        evolve_to_equilibrium(&mut field, 10);

        // Sybil: "fake-org" at region 12 — far away, no one interacts with it
        let sybil_w = geometric_weight(&field, 12);
        let real_w = geometric_weight(&field, 3);

        assert!(
            real_w.weight > sybil_w.weight,
            "Real identity ({:.2}) should outweigh Sybil ({:.2})",
            real_w.weight,
            sybil_w.weight
        );
    }

    #[test]
    fn sybil_flood_no_weight() {
        let mut field = Field::new(16);

        // Real activity concentrated in region 5
        field.seed_named(
            Coord {
                t: 3,
                c: 5,
                o: 5,
                v: 3,
            },
            "legit-tx-1",
        );
        field.seed_named(
            Coord {
                t: 4,
                c: 5,
                o: 5,
                v: 3,
            },
            "legit-tx-2",
        );
        field.seed_named(
            Coord {
                t: 5,
                c: 5,
                o: 5,
                v: 3,
            },
            "legit-tx-3",
        );
        evolve_to_equilibrium(&mut field, 10);

        let real = geometric_weight(&field, 5);

        // Sybil creates 10 fake identities — none interact with real users
        let sybil_total: f64 = (0..10)
            .map(|i| geometric_weight(&field, 10 + i).weight)
            .sum();

        assert!(
            real.weight > sybil_total,
            "One real identity ({:.2}) should outweigh 10 Sybils ({:.2})",
            real.weight,
            sybil_total
        );
    }
}
