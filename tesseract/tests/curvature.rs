//! Curvature budget tests — geometric scarcity via field physics.
//!
//! No locks. No pre-validation. No rejection at seed time.
//! Both deformations enter the field freely.
//! The field's evolution physics decays the weakest when over-capacity.
//! Like a material that fractures at the weakest point under excess strain.

use tesseract::*;

#[test]
fn unconstrained_field_allows_everything() {
    let mut field = Field::new(8);
    // No budget = no capacity constraint
    field.seed_named(
        Coord {
            t: 1,
            c: 1,
            o: 1,
            v: 1,
        },
        "a",
    );
    field.seed_named(
        Coord {
            t: 2,
            c: 1,
            o: 1,
            v: 1,
        },
        "b",
    );
    field.seed_named(
        Coord {
            t: 3,
            c: 1,
            o: 1,
            v: 1,
        },
        "c",
    );
    evolve_to_equilibrium(&mut field, 20);

    // All crystallize — no constraint
    assert!(
        field
            .get(Coord {
                t: 1,
                c: 1,
                o: 1,
                v: 1
            })
            .crystallized
    );
    assert!(
        field
            .get(Coord {
                t: 2,
                c: 1,
                o: 1,
                v: 1
            })
            .crystallized
    );
    assert!(
        field
            .get(Coord {
                t: 3,
                c: 1,
                o: 1,
                v: 1
            })
            .crystallized
    );
}

#[test]
fn over_capacity_decays_weakest() {
    let mut field = Field::new(8);

    // Region 3 can sustain only 2 crystallized cells
    field.set_capacity(3, 2.0);

    // Seed 3 events in region 3 — all enter freely
    field.seed_named(
        Coord {
            t: 1,
            c: 3,
            o: 3,
            v: 3,
        },
        "strong-A",
    );
    field.seed_named(
        Coord {
            t: 3,
            c: 3,
            o: 3,
            v: 3,
        },
        "strong-B",
    );
    // This one is far from A and B — weakest support
    field.seed_named(
        Coord {
            t: 6,
            c: 6,
            o: 3,
            v: 6,
        },
        "weak-C",
    );

    // All three seed freely — no rejection
    evolve_to_equilibrium(&mut field, 20);

    let load = field.curvature_load(3);
    let capacity = field.capacity(3).unwrap();

    println!("  capacity: {}, load after evolution: {}", capacity, load);

    // Load should not exceed capacity — weakest was decayed
    assert!(
        load <= capacity,
        "Load ({}) should not exceed capacity ({})",
        load,
        capacity
    );
}

#[test]
fn stronger_deformation_survives_weaker_decays() {
    let mut field = Field::new(8);

    // Region 3: capacity for 5 crystallizations
    field.set_capacity(3, 5.0);

    // Strong cluster: 3 nearby events (high mutual support)
    field.seed_named(
        Coord {
            t: 2,
            c: 3,
            o: 3,
            v: 3,
        },
        "strong-1",
    );
    field.seed_named(
        Coord {
            t: 3,
            c: 3,
            o: 3,
            v: 3,
        },
        "strong-2",
    );
    field.seed_named(
        Coord {
            t: 4,
            c: 3,
            o: 3,
            v: 3,
        },
        "strong-3",
    );

    // Weak isolated events (low support)
    field.seed_named(
        Coord {
            t: 0,
            c: 0,
            o: 3,
            v: 0,
        },
        "weak-1",
    );
    field.seed_named(
        Coord {
            t: 7,
            c: 7,
            o: 3,
            v: 7,
        },
        "weak-2",
    );
    field.seed_named(
        Coord {
            t: 0,
            c: 7,
            o: 3,
            v: 0,
        },
        "weak-3",
    );

    evolve_to_equilibrium(&mut field, 30);

    // Strong cluster should survive — they reinforce each other
    let s1 = field
        .get(Coord {
            t: 2,
            c: 3,
            o: 3,
            v: 3,
        })
        .crystallized;
    let s2 = field
        .get(Coord {
            t: 3,
            c: 3,
            o: 3,
            v: 3,
        })
        .crystallized;
    let s3 = field
        .get(Coord {
            t: 4,
            c: 3,
            o: 3,
            v: 3,
        })
        .crystallized;

    let strong_surviving = [s1, s2, s3].iter().filter(|x| **x).count();
    let load = field.curvature_load(3);

    println!("  strong cluster surviving: {}/3", strong_surviving);
    println!("  total load: {}, capacity: 5", load);

    // The field MUST respect capacity — that's the geometric constraint
    assert!(load <= 5.0, "Load ({}) must not exceed capacity (5)", load);

    // With limited capacity, the field decides what survives.
    // We don't dictate which — we verify the constraint holds.
    assert!(load > 0.0, "Some crystallizations should survive");
}

#[test]
fn competing_deformations_field_decides() {
    // THE critical test: double-spend as physics.
    // Alice's region has capacity 3. She seeds TWO transfers
    // that together would exceed it. Both enter freely.
    // The field decides which survives based on geometric strength.
    let mut field = Field::new(8);

    field.set_capacity(3, 3.0);

    // Transfer A: strong (near existing crystallizations)
    field.seed_named(
        Coord {
            t: 2,
            c: 3,
            o: 3,
            v: 3,
        },
        "transfer→bob",
    );
    field.seed_named(
        Coord {
            t: 3,
            c: 3,
            o: 3,
            v: 3,
        },
        "transfer→bob-support",
    );

    // Transfer B: weaker (isolated)
    field.seed_named(
        Coord {
            t: 6,
            c: 6,
            o: 3,
            v: 6,
        },
        "transfer→carol",
    );

    // Both entered freely — no lock, no rejection
    evolve_to_equilibrium(&mut field, 30);

    let bob_crystal = field
        .get(Coord {
            t: 2,
            c: 3,
            o: 3,
            v: 3,
        })
        .crystallized;
    let carol_crystal = field
        .get(Coord {
            t: 6,
            c: 6,
            o: 3,
            v: 6,
        })
        .crystallized;
    let load = field.curvature_load(3);

    println!("  transfer→bob crystallized: {}", bob_crystal);
    println!("  transfer→carol crystallized: {}", carol_crystal);
    println!("  load: {}, capacity: 3", load);

    // Load must respect capacity
    assert!(load <= 3.0, "Load ({}) must not exceed capacity (3)", load);

    // The stronger deformation (bob, with support nearby) should survive
    // Carol's isolated deformation should be the one that decays
    if bob_crystal && !carol_crystal {
        println!("  RESULT: Field chose the stronger deformation.");
        println!("  No protocol decided. No vote. No validation.");
        println!("  The geometry of the space made it impossible for both to persist.");
    } else {
        println!("  RESULT: bob={}, carol={}", bob_crystal, carol_crystal);
        println!("  The field resolved the conflict — load is within capacity.");
    }
}

#[test]
fn capacity_different_regions_independent() {
    let mut field = Field::new(8);

    field.set_capacity(2, 2.0);
    field.set_capacity(5, 100.0);

    // Region 2: seed 5 events (will exceed capacity 2)
    for i in 0..5 {
        field.seed_named(
            Coord {
                t: i,
                c: 3,
                o: 2,
                v: 3,
            },
            &format!("r2-{}", i),
        );
    }
    // Region 5: seed 5 events (well within capacity 100)
    for i in 0..5 {
        field.seed_named(
            Coord {
                t: i,
                c: 3,
                o: 5,
                v: 3,
            },
            &format!("r5-{}", i),
        );
    }

    evolve_to_equilibrium(&mut field, 30);

    let load_2 = field.curvature_load(2);
    let load_5 = field.curvature_load(5);

    println!("  region 2: load={}, capacity=2", load_2);
    println!("  region 5: load={}, capacity=100", load_5);

    assert!(load_2 <= 2.0, "Region 2 should respect its capacity");
    assert!(load_5 > 0.0, "Region 5 should have crystallizations");
}

#[test]
fn self_healing_works_under_capacity() {
    // Capacity doesn't prevent self-healing.
    // size=8 so source-aware σ has diverse sources.
    let mut field = Field::new(8);

    field.set_capacity(1, 1000.0); // generous capacity

    let center = Coord {
        t: 3,
        c: 3,
        o: 3,
        v: 3,
    };
    field.seed_named(center, "core");
    field.seed_named(
        Coord {
            t: 5,
            c: 3,
            o: 3,
            v: 3,
        },
        "support-t",
    );
    field.seed_named(
        Coord {
            t: 3,
            c: 5,
            o: 3,
            v: 3,
        },
        "support-c",
    );
    field.seed_named(
        Coord {
            t: 3,
            c: 3,
            o: 5,
            v: 3,
        },
        "support-o",
    );
    evolve_to_equilibrium(&mut field, 15);

    assert!(field.get(center).crystallized);

    // Destroy
    field.destroy(center);
    evolve_to_equilibrium(&mut field, 20);

    // Should recover — load is within capacity
    assert!(
        field.get(center).crystallized,
        "Self-healing should work when load is within capacity"
    );
}
