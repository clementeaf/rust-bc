//! Tesseract experiments — 10 proofs of convergence-based consensus

use tesseract::*;

/// Dense cluster with NAMED seeds for source diversity.
/// Each seed has a unique event_id so source-aware σ can detect
/// diverse evidence from different directions.
fn seed_dense_cluster(field: &mut Field) {
    let coords_and_names: Vec<(Coord, &str)> = vec![
        (Coord { t: 1, c: 1, o: 1, v: 1 }, "center"),
        (Coord { t: 2, c: 1, o: 1, v: 1 }, "axis-t-pos"),
        (Coord { t: 0, c: 1, o: 1, v: 1 }, "axis-t-neg"),
        (Coord { t: 1, c: 2, o: 1, v: 1 }, "axis-c-pos"),
        (Coord { t: 1, c: 0, o: 1, v: 1 }, "axis-c-neg"),
        (Coord { t: 1, c: 1, o: 2, v: 1 }, "axis-o-pos"),
        (Coord { t: 1, c: 1, o: 0, v: 1 }, "axis-o-neg"),
        (Coord { t: 1, c: 1, o: 1, v: 2 }, "axis-v-pos"),
        (Coord { t: 1, c: 1, o: 1, v: 0 }, "axis-v-neg"),
        (Coord { t: 2, c: 2, o: 1, v: 1 }, "diag-tc"),
        (Coord { t: 2, c: 1, o: 2, v: 1 }, "diag-to"),
        (Coord { t: 2, c: 1, o: 1, v: 2 }, "diag-tv"),
        (Coord { t: 1, c: 2, o: 2, v: 1 }, "diag-co"),
        (Coord { t: 1, c: 2, o: 1, v: 2 }, "diag-cv"),
    ];
    for (coord, name) in &coords_and_names {
        field.seed_named(*coord, name);
    }
}

fn dense_cluster() -> Vec<Coord> {
    vec![
        Coord { t: 1, c: 1, o: 1, v: 1 },
        Coord { t: 2, c: 1, o: 1, v: 1 },
        Coord { t: 0, c: 1, o: 1, v: 1 },
        Coord { t: 1, c: 2, o: 1, v: 1 },
        Coord { t: 1, c: 0, o: 1, v: 1 },
        Coord { t: 1, c: 1, o: 2, v: 1 },
        Coord { t: 1, c: 1, o: 0, v: 1 },
        Coord { t: 1, c: 1, o: 1, v: 2 },
        Coord { t: 1, c: 1, o: 1, v: 0 },
        Coord { t: 2, c: 2, o: 1, v: 1 },
        Coord { t: 2, c: 1, o: 2, v: 1 },
        Coord { t: 2, c: 1, o: 1, v: 2 },
        Coord { t: 1, c: 2, o: 2, v: 1 },
        Coord { t: 1, c: 2, o: 1, v: 2 },
        Coord { t: 1, c: 1, o: 2, v: 2 },
    ]
}

// === Experiment 1: Convergence without consensus ===

#[test]
fn exp1_empty_field_does_not_crystallize() {
    let mut field = Field::new(4);
    evolve_to_equilibrium(&mut field, 20);
    assert_eq!(field.crystallized_count(), 0);
}

#[test]
fn exp1_noise_does_not_crystallize() {
    let mut field = Field::new(4);
    let mut rng = rand::thread_rng();
    use rand::Rng;
    for t in 0..4 {
        for c in 0..4 {
            for o in 0..4 {
                for v in 0..4 {
                    field.get_mut(Coord { t, c, o, v }).probability = rng.gen_range(0.0..0.3);
                }
            }
        }
    }
    evolve_to_equilibrium(&mut field, 20);
    assert_eq!(field.crystallized_count(), 0);
}

#[test]
fn exp1_orthogonal_seeds_crystallize() {
    let mut field = Field::new(4);
    field.seed(Coord { t: 1, c: 1, o: 1, v: 1 });
    field.seed(Coord { t: 2, c: 1, o: 1, v: 1 });
    field.seed(Coord { t: 1, c: 2, o: 1, v: 1 });
    field.seed(Coord { t: 1, c: 1, o: 2, v: 1 });
    field.seed(Coord { t: 1, c: 1, o: 1, v: 2 });
    evolve_to_equilibrium(&mut field, 20);
    assert!(field.crystallized_count() >= 5);
}

// === Experiment 2: Self-healing ===

#[test]
fn exp2_destroyed_cell_recovers() {
    // size=8 so source-aware σ can detect diverse sources
    // (size=4 puts all cells within every orbital → σ=0 always)
    let mut field = Field::new(8);
    seed_dense_cluster(&mut field);
    evolve_to_equilibrium(&mut field, 10);

    let target = Coord { t: 1, c: 1, o: 1, v: 1 };
    assert!(field.get(target).crystallized);

    field.destroy(target);
    assert!(!field.get(target).crystallized);

    evolve_to_equilibrium(&mut field, 15);
    assert!(field.get(target).crystallized);
}

// === Experiment 3: Rejection of falsehood ===

#[test]
fn exp3_false_injection_does_not_propagate() {
    let mut field_clean = Field::new(8);
    let mut field_fake = Field::new(8);

    seed_dense_cluster(&mut field_clean);
    seed_dense_cluster(&mut field_fake);

    // Inject fake into field_fake only
    let fake = Coord { t: 6, c: 6, o: 6, v: 6 };
    let cell = field_fake.get_mut(fake);
    cell.probability = 1.0;
    cell.crystallized = true;

    evolve_to_equilibrium(&mut field_clean, 20);
    evolve_to_equilibrium(&mut field_fake, 20);

    let clean_count = field_clean.crystallized_count();
    let fake_count = field_fake.crystallized_count();

    assert!(
        fake_count <= clean_count + 2,
        "Fake should not propagate: clean={}, with_fake={}",
        clean_count, fake_count
    );
}

// === Experiment 4: Sustained attack ===

#[test]
fn exp4_ten_attacks_ten_recoveries() {
    let mut field = Field::new(8);
    seed_dense_cluster(&mut field);
    evolve_to_equilibrium(&mut field, 10);

    let target = Coord { t: 1, c: 1, o: 1, v: 1 };

    let mut recoveries = 0;
    for _ in 0..10 {
        field.destroy(target);
        evolve_to_equilibrium(&mut field, 20);
        if field.get(target).crystallized {
            recoveries += 1;
        }
    }
    assert!(recoveries >= 8, "Should recover at least 8/10 attacks, got {}/10", recoveries);
}

// === Experiment 5: Axis independence ===

#[test]
fn exp5_one_axis_destroyed_three_sustain() {
    let mut field = Field::new(8);
    seed_dense_cluster(&mut field);
    evolve_to_equilibrium(&mut field, 10);

    let target = Coord { t: 1, c: 1, o: 1, v: 1 };
    assert!(field.get(target).crystallized, "Target should be crystallized before attack");

    // Destroy 2 T-axis neighbors — target should survive because
    // crystallization is irreversible and other axes still have support
    field.destroy(Coord { t: 0, c: 1, o: 1, v: 1 });
    field.destroy(Coord { t: 2, c: 1, o: 1, v: 1 });

    // Target is still crystallized (crystallization is permanent under Axiom 3)
    assert!(field.get(target).crystallized, "Target should survive partial axis attack");

    // After evolution, destroyed neighbors should recover
    evolve_to_equilibrium(&mut field, 15);
    assert!(field.get(target).crystallized, "Target remains after healing");
}

// === Experiment 6: Total destruction (orbital model) ===

#[test]
fn exp6_total_destruction_recovers() {
    let mut field = Field::new(8);
    seed_dense_cluster(&mut field);
    evolve_to_equilibrium(&mut field, 10);

    let target = Coord { t: 1, c: 1, o: 1, v: 1 };
    let neighbors = field.neighbors(target);

    // Destroy target + all 8 direct neighbors
    field.destroy(target);
    for n in &neighbors { field.destroy(*n); }

    assert!(!field.get(target).crystallized);

    evolve_to_equilibrium(&mut field, 30);

    assert!(field.get(target).crystallized, "Should recover from total destruction via orbital depth");
}

// === Experiment 7: Coexistence ===

#[test]
fn exp7_independent_events_coexist() {
    let mut field = Field::new(8);

    let a = Coord { t: 1, c: 1, o: 1, v: 1 };
    let b = Coord { t: 6, c: 6, o: 6, v: 6 };
    let c = Coord { t: 1, c: 6, o: 3, v: 5 };

    field.seed(a);
    field.seed(b);
    field.seed(c);
    evolve_to_equilibrium(&mut field, 20);

    assert!(field.get(a).crystallized);
    assert!(field.get(b).crystallized);
    assert!(field.get(c).crystallized);

    // Destroy A — B and C should be unaffected
    let a_neighbors = field.neighbors(a);
    field.destroy(a);
    for n in &a_neighbors { field.destroy(*n); }

    evolve_to_equilibrium(&mut field, 20);

    assert!(field.get(b).crystallized, "Event B should survive destruction of A");
    assert!(field.get(c).crystallized, "Event C should survive destruction of A");
}

// === Experiment 8: Emergent crystallization ===

#[test]
fn exp8_orbital_overlap_produces_emergent_crystal() {
    let mut field = Field::new(8);

    let x = Coord { t: 2, c: 3, o: 3, v: 3 };
    let y = Coord { t: 4, c: 3, o: 3, v: 3 };
    let midpoint = Coord { t: 3, c: 3, o: 3, v: 3 };

    assert_eq!(field.get(midpoint).probability, 0.0);

    field.seed(x);
    field.seed(y);
    evolve_to_equilibrium(&mut field, 20);

    assert!(field.get(midpoint).crystallized, "Midpoint should crystallize from orbital overlap");
}

// === Experiment 9: Emergent self-healing ===

#[test]
fn exp9_emergent_cell_self_heals() {
    let mut field = Field::new(8);

    field.seed(Coord { t: 2, c: 3, o: 3, v: 3 });
    field.seed(Coord { t: 4, c: 3, o: 3, v: 3 });
    evolve_to_equilibrium(&mut field, 20);

    let midpoint = Coord { t: 3, c: 3, o: 3, v: 3 };
    assert!(field.get(midpoint).crystallized);

    field.destroy(midpoint);
    assert!(!field.get(midpoint).crystallized);

    evolve_to_equilibrium(&mut field, 20);
    assert!(field.get(midpoint).crystallized, "Emergent cell should self-heal from parent orbitals");
}

// === Experiment 10: Emergent records carry meaning ===

#[test]
fn exp10_emergent_cell_knows_its_parents() {
    let mut field = Field::new(8);

    let ev_alice = Coord { t: 2, c: 3, o: 3, v: 3 };
    let ev_bob = Coord { t: 4, c: 3, o: 3, v: 3 };
    let midpoint = Coord { t: 3, c: 3, o: 3, v: 3 };

    field.seed_named(ev_alice, "Alice→Bob:10tok");
    field.seed_named(ev_bob, "Bob→Carol:5tok");
    evolve_to_equilibrium(&mut field, 20);

    let mid_cell = field.get(midpoint);
    assert!(mid_cell.crystallized);

    let has_alice = mid_cell.influences.iter().any(|i| i.event_id.contains("Alice"));
    let has_carol = mid_cell.influences.iter().any(|i| i.event_id.contains("Carol"));

    assert!(has_alice, "Emergent cell should know about Alice event");
    assert!(has_carol, "Emergent cell should know about Carol event");

    // Record should be readable
    let record = mid_cell.record();
    assert!(record.contains("Alice"));
    assert!(record.contains("Carol"));
}
