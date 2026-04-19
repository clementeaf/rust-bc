//! Tesseract experiments — 10 proofs of convergence-based consensus

use tesseract::*;

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
    let mut field = Field::new(4);
    for c in dense_cluster() { field.seed(c); }
    evolve_to_equilibrium(&mut field, 10);

    let target = Coord { t: 1, c: 1, o: 1, v: 1 };
    assert!(field.get(target).crystallized);

    field.destroy(target);
    assert!(!field.get(target).crystallized);

    evolve_to_equilibrium(&mut field, 10);
    assert!(field.get(target).crystallized);
    assert_eq!(field.orthogonal_support(target), 4);
}

// === Experiment 3: Rejection of falsehood ===

#[test]
fn exp3_false_injection_does_not_propagate() {
    // Create two identical fields. Inject a fake into one.
    // The fake should NOT cause additional crystallizations beyond
    // what the field produces naturally.
    let mut field_clean = Field::new(8);
    let mut field_fake = Field::new(8);

    for c in dense_cluster().iter().take(9) {
        field_clean.seed(*c);
        field_fake.seed(*c);
    }

    // Inject fake into field_fake only
    let fake = Coord { t: 6, c: 6, o: 6, v: 6 };
    let cell = field_fake.get_mut(fake);
    cell.probability = 1.0;
    cell.crystallized = true;

    evolve_to_equilibrium(&mut field_clean, 20);
    evolve_to_equilibrium(&mut field_fake, 20);

    let clean_count = field_clean.crystallized_count();
    let fake_count = field_fake.crystallized_count();

    // The fake adds itself (1 cell) but should not cause a cascade
    // of new crystallizations. At most +1 (the fake itself).
    assert!(
        fake_count <= clean_count + 1,
        "Fake should not propagate: clean={}, with_fake={}",
        clean_count, fake_count
    );
}

// === Experiment 4: Sustained attack ===

#[test]
fn exp4_ten_attacks_ten_recoveries() {
    let mut field = Field::new(4);
    for c in dense_cluster() { field.seed(c); }
    evolve_to_equilibrium(&mut field, 10);

    let target = Coord { t: 1, c: 1, o: 1, v: 1 };

    for _ in 0..10 {
        field.destroy(target);
        evolve_to_equilibrium(&mut field, 5);
        assert!(field.get(target).crystallized, "Cell should recover after each attack");
    }
}

// === Experiment 5: Axis independence ===

#[test]
fn exp5_one_axis_destroyed_three_sustain() {
    let mut field = Field::new(4);
    for c in dense_cluster() { field.seed(c); }
    evolve_to_equilibrium(&mut field, 10);

    let target = Coord { t: 1, c: 1, o: 1, v: 1 };
    // Destroy T-axis neighbors
    field.destroy(Coord { t: 0, c: 1, o: 1, v: 1 });
    field.destroy(Coord { t: 2, c: 1, o: 1, v: 1 });
    field.destroy(Coord { t: 2, c: 2, o: 1, v: 1 });
    field.destroy(Coord { t: 2, c: 1, o: 2, v: 1 });
    field.destroy(Coord { t: 2, c: 1, o: 1, v: 2 });

    assert!(field.get(target).crystallized, "Target should survive axis attack");
    assert!(field.orthogonal_support(target) >= 3);
}

// === Experiment 6: Total destruction (orbital model) ===

#[test]
fn exp6_total_destruction_recovers() {
    let mut field = Field::new(4);
    for c in dense_cluster() { field.seed(c); }
    evolve_to_equilibrium(&mut field, 10);

    let target = Coord { t: 1, c: 1, o: 1, v: 1 };
    let neighbors = field.neighbors(target);

    // Destroy target + all 8 direct neighbors
    field.destroy(target);
    for n in &neighbors { field.destroy(*n); }

    assert!(!field.get(target).crystallized);
    assert_eq!(field.orthogonal_support(target), 0);

    evolve_to_equilibrium(&mut field, 20);

    assert!(field.get(target).crystallized, "Should recover from total destruction via orbital depth");
    assert_eq!(field.orthogonal_support(target), 4);
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
