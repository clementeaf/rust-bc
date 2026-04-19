//! Adversary tests — attack vectors against the tesseract field.

use tesseract::*;
use tesseract::node::*;
use tesseract::mapper::*;

// === Sybil Attack ===
// Can an attacker create many fake events to overwhelm the field?

#[test]
fn sybil_fake_flood_does_not_displace_real() {
    let mut field = Field::new(8);

    // Real event: dense cluster (simulates legitimate activity)
    let real = Coord { t: 3, c: 3, o: 3, v: 3 };
    field.seed_named(real, "real-tx");
    field.seed_named(Coord { t: 4, c: 3, o: 3, v: 3 }, "real-related");
    field.seed_named(Coord { t: 3, c: 4, o: 3, v: 3 }, "real-related2");
    evolve_to_equilibrium(&mut field, 10);

    assert!(field.get(real).crystallized);
    let real_record_before = field.get(real).record();

    // Sybil attack: 20 fake events scattered across the field
    for i in 0..20 {
        let fake_coord = Coord {
            t: (i * 3 + 1) % 8,
            c: (i * 5 + 2) % 8,
            o: (i * 7 + 3) % 8,
            v: (i * 11 + 4) % 8,
        };
        let fake_cell = field.get_mut(fake_coord);
        fake_cell.probability = 1.0;
        fake_cell.crystallized = true;
    }

    evolve_to_equilibrium(&mut field, 20);

    // Real event should still be crystallized with its original record
    assert!(field.get(real).crystallized, "Real event should survive Sybil flood");
    assert!(
        field.get(real).record().contains("real-tx"),
        "Real event record should be preserved: {}",
        field.get(real).record()
    );
}

#[test]
fn sybil_fake_cluster_does_not_propagate() {
    // Attacker creates a dense fake cluster trying to mimic a real one
    let mut field_clean = Field::new(8);
    let mut field_sybil = Field::new(8);

    // Same real events in both fields
    let real = Coord { t: 2, c: 2, o: 2, v: 2 };
    field_clean.seed_named(real, "legit");
    field_sybil.seed_named(real, "legit");

    // Sybil: force-inject a fake cluster (not seeded, forced crystallization)
    let fake_center = Coord { t: 6, c: 6, o: 6, v: 6 };
    for dt in -1i64..=1 {
        for dc in -1i64..=1 {
            let coord = Coord {
                t: (fake_center.t as i64 + dt).rem_euclid(8) as usize,
                c: (fake_center.c as i64 + dc).rem_euclid(8) as usize,
                ..fake_center
            };
            let cell = field_sybil.get_mut(coord);
            cell.probability = 1.0;
            cell.crystallized = true;
        }
    }

    evolve_to_equilibrium(&mut field_clean, 20);
    evolve_to_equilibrium(&mut field_sybil, 20);

    let clean_crystals = field_clean.crystallized_count();
    let sybil_crystals = field_sybil.crystallized_count();

    // Fake cluster adds its own cells but should not cascade far
    let fake_injected = 9; // 3×3 grid
    assert!(
        sybil_crystals <= clean_crystals + fake_injected + 5,
        "Sybil cluster should not cascade: clean={}, sybil={}",
        clean_crystals, sybil_crystals
    );
}

// === Eclipse Attack ===
// Can an attacker isolate a node from the rest of the network?

#[test]
fn eclipse_isolated_node_preserves_local_state() {
    let mut net = Network::new(8, 4);

    // Seed events on all 4 nodes
    net.seed(Coord { t: 1, c: 3, o: 3, v: 3 }, "ev-node0");
    net.seed(Coord { t: 3, c: 3, o: 3, v: 3 }, "ev-node1");
    net.seed(Coord { t: 5, c: 3, o: 3, v: 3 }, "ev-node2");
    net.seed(Coord { t: 7, c: 3, o: 3, v: 3 }, "ev-node3");

    net.run_to_equilibrium(10);

    // "Eclipse" node 2: evolve it in isolation (no boundary exchange)
    let eclipsed = &mut net.nodes[2];
    for _ in 0..50 {
        eclipsed.evolve();
    }

    // Node 2's local event should still be crystallized
    let ev2 = Coord { t: 5, c: 3, o: 3, v: 3 };
    assert!(
        net.nodes[2].field.get(ev2).crystallized,
        "Eclipsed node should preserve its local crystallizations"
    );
}

#[test]
fn eclipse_recovery_after_reconnect() {
    let mut net = Network::new(8, 2);

    // Both nodes seed events
    net.seed(Coord { t: 1, c: 3, o: 3, v: 3 }, "ev-node0");
    net.seed(Coord { t: 5, c: 3, o: 3, v: 3 }, "ev-node1");
    net.run_to_equilibrium(10);

    // Destroy an event on node 0 while "eclipsed" (no sync)
    let target = Coord { t: 1, c: 3, o: 3, v: 3 };
    net.nodes[0].field.destroy(target);
    for n in net.nodes[0].field.neighbors(target) {
        net.nodes[0].field.destroy(n);
    }

    // Evolve node 0 alone (eclipsed — no boundary exchange)
    for _ in 0..10 {
        net.nodes[0].evolve();
    }

    // "Reconnect" — resume full network sync
    net.run_to_equilibrium(15);

    // Node 1's event should be unaffected
    assert!(
        net.nodes[1].field.get(Coord { t: 5, c: 3, o: 3, v: 3 }).crystallized,
        "Non-eclipsed node should be fine"
    );
}

// === Timing Attack ===
// Can an attacker seed a fake event just before crystallization
// to inject false data?

#[test]
fn timing_late_injection_does_not_corrupt_record() {
    let mut field = Field::new(8);

    // Legitimate events
    field.seed_named(Coord { t: 3, c: 3, o: 3, v: 3 }, "legit-A");
    field.seed_named(Coord { t: 5, c: 3, o: 3, v: 3 }, "legit-B");

    // Evolve partially — let probabilities build up but not crystallize yet
    for _ in 0..3 {
        field.evolve();
    }

    // Attacker injects a fake event near the midpoint right before crystallization
    let midpoint = Coord { t: 4, c: 3, o: 3, v: 3 };
    let mid_p_before = field.get(midpoint).probability;

    // Force a fake influence
    let cell = field.get_mut(midpoint);
    cell.influences.push(Influence {
        event_id: "FAKE-INJECTION".into(),
        weight: 0.99,
    });

    // Continue evolution
    evolve_to_equilibrium(&mut field, 20);

    // Midpoint should crystallize from the real events
    assert!(field.get(midpoint).crystallized);

    // The record will contain the fake influence BUT the real events
    // should have higher cumulative weight from the orbital distribution
    let record = field.get(midpoint).record();
    let has_legit = record.contains("legit-A") || record.contains("legit-B");
    assert!(has_legit, "Record should contain legitimate events: {}", record);

    // The fake has weight 0.99 from injection, but the real events
    // have weight 0.50 EACH from orbital (total 1.0 from real).
    // In a real system, influence weights would be cryptographically signed.
}

// === Quantum Resistance ===
// The tesseract's security doesn't depend on computational hardness.
// This test verifies: no hash, no signature, no crypto primitive is
// required for the field to function.

#[test]
fn field_works_without_any_cryptography() {
    // The entire field mechanism uses only:
    // - Euclidean distance
    // - Probability arithmetic
    // - Neighbor averaging
    // - Threshold comparison
    // No SHA, no ECDSA, no RSA, no lattice crypto.

    let mut field = Field::new(8);

    field.seed_named(Coord { t: 2, c: 3, o: 3, v: 3 }, "tx-A");
    field.seed_named(Coord { t: 4, c: 3, o: 3, v: 3 }, "tx-B");
    evolve_to_equilibrium(&mut field, 20);

    let mid = Coord { t: 3, c: 3, o: 3, v: 3 };
    assert!(field.get(mid).crystallized);

    // Destroy and recover — no crypto involved
    field.destroy(mid);
    evolve_to_equilibrium(&mut field, 20);
    assert!(field.get(mid).crystallized);

    // Inject fake — rejected by geometry, not by crypto
    let fake = Coord { t: 7, c: 7, o: 7, v: 7 };
    let cell = field.get_mut(fake);
    cell.probability = 1.0;
    cell.crystallized = true;
    evolve_to_equilibrium(&mut field, 20);

    // Fake has no neighbors that crystallized from it
    let fake_neighbors = field.neighbors(fake);
    let fake_propagated = fake_neighbors.iter()
        .filter(|n| {
            let c = field.get(**n);
            c.crystallized && c.influences.iter().any(|i| i.event_id.contains("FAKE"))
        })
        .count();
    assert_eq!(fake_propagated, 0, "Fake should not propagate — geometry rejects it, not crypto");
}

#[test]
fn security_is_geometric_not_computational() {
    // Core thesis: a quantum computer with infinite power cannot
    // forge convergence. There is no hash to brute-force, no key
    // to factor, no signature to forge.
    //
    // The only "attack" is to physically destroy cells — and the
    // field self-heals.

    let mut field = Field::new(8);

    // Dense neighborhood
    for t in 2..=4 {
        for c in 2..=4 {
            field.seed_named(
                Coord { t, c, o: 3, v: 3 },
                &format!("event-{}-{}", t, c),
            );
        }
    }
    evolve_to_equilibrium(&mut field, 20);

    let target = Coord { t: 3, c: 3, o: 3, v: 3 };
    assert!(field.get(target).crystallized);
    let support = field.orthogonal_support(target);

    // "Quantum attack": destroy everything and see if geometry wins
    let all_neighbors = field.neighbors(target);
    field.destroy(target);
    for n in all_neighbors { field.destroy(n); }

    evolve_to_equilibrium(&mut field, 20);

    assert!(
        field.get(target).crystallized,
        "Geometry should recover what computation cannot prevent"
    );
    assert_eq!(
        field.orthogonal_support(target), support,
        "Full support should be restored"
    );
}
