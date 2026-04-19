//! Scale tests — verify properties hold at larger field sizes.

use tesseract::*;
use std::time::Instant;

fn bench<F: FnOnce() -> R, R>(label: &str, f: F) -> R {
    let start = Instant::now();
    let result = f();
    println!("  {}: {:.2?}", label, start.elapsed());
    result
}

/// Create a dense cluster of events around a center (like real-world usage).
fn seed_cluster(field: &mut Field, center: Coord, name: &str) {
    let s = field.size;
    // Center + all axis neighbors = 9 events
    field.seed_named(center, &format!("{}-core", name));
    for offset in [1_i64, -1] {
        field.seed_named(Coord { t: ((center.t as i64 + offset).rem_euclid(s as i64)) as usize, ..center }, &format!("{}-t", name));
        field.seed_named(Coord { c: ((center.c as i64 + offset).rem_euclid(s as i64)) as usize, ..center }, &format!("{}-c", name));
        field.seed_named(Coord { o: ((center.o as i64 + offset).rem_euclid(s as i64)) as usize, ..center }, &format!("{}-o", name));
        field.seed_named(Coord { v: ((center.v as i64 + offset).rem_euclid(s as i64)) as usize, ..center }, &format!("{}-v", name));
    }
}

// === 16⁴ ===

#[test]
fn scale_16_seed_and_crystallize() {
    println!("\n=== Field 16⁴ (65,536 logical cells) ===");
    let mut field = Field::new(16);

    bench("seed cluster A", || seed_cluster(&mut field, Coord { t: 4, c: 4, o: 4, v: 4 }, "A"));
    bench("seed cluster B", || seed_cluster(&mut field, Coord { t: 12, c: 12, o: 12, v: 12 }, "B"));

    println!("  active: {} / {} ({:.1}%)", field.active_cells(), field.total_cells(),
        field.active_cells() as f64 / field.total_cells() as f64 * 100.0);

    bench("evolve", || evolve_to_equilibrium(&mut field, 20));

    println!("  crystallized: {}", field.crystallized_count());
    assert!(field.get(Coord { t: 4, c: 4, o: 4, v: 4 }).crystallized);
    assert!(field.get(Coord { t: 12, c: 12, o: 12, v: 12 }).crystallized);
}

#[test]
fn scale_16_self_healing() {
    println!("\n=== Self-healing at 16⁴ (dense neighborhood) ===");
    let mut field = Field::new(16);
    let target = Coord { t: 4, c: 4, o: 4, v: 4 };

    // Dense neighborhood: 3 nearby clusters (realistic — events don't happen in isolation)
    seed_cluster(&mut field, target, "A");
    seed_cluster(&mut field, Coord { t: 6, c: 4, o: 4, v: 4 }, "B");
    seed_cluster(&mut field, Coord { t: 4, c: 6, o: 4, v: 4 }, "C");
    evolve_to_equilibrium(&mut field, 10);
    assert!(field.get(target).crystallized);

    // Destroy target + all neighbors
    let neighbors = field.neighbors(target);
    field.destroy(target);
    for n in neighbors { field.destroy(n); }

    let recovered = bench("recovery", || {
        evolve_to_equilibrium(&mut field, 20);
        field.get(target).crystallized
    });
    println!("  recovered: {}", recovered);
    assert!(recovered);
}

// === 32⁴ ===

#[test]
fn scale_32_seed_and_crystallize() {
    println!("\n=== Field 32⁴ (1,048,576 logical cells) ===");
    let mut field = Field::new(32);

    bench("seed cluster A", || seed_cluster(&mut field, Coord { t: 8, c: 8, o: 8, v: 8 }, "A"));
    bench("seed cluster B", || seed_cluster(&mut field, Coord { t: 24, c: 24, o: 24, v: 24 }, "B"));

    println!("  active: {} / {} ({:.1}%)", field.active_cells(), field.total_cells(),
        field.active_cells() as f64 / field.total_cells() as f64 * 100.0);

    bench("evolve", || evolve_to_equilibrium(&mut field, 20));

    println!("  crystallized: {}", field.crystallized_count());
    assert!(field.get(Coord { t: 8, c: 8, o: 8, v: 8 }).crystallized);
    assert!(field.get(Coord { t: 24, c: 24, o: 24, v: 24 }).crystallized);
}

#[test]
fn scale_32_self_healing() {
    println!("\n=== Self-healing at 32⁴ (dense neighborhood) ===");
    let mut field = Field::new(32);
    let target = Coord { t: 8, c: 8, o: 8, v: 8 };

    // Dense neighborhood: 4 nearby clusters
    seed_cluster(&mut field, target, "A");
    seed_cluster(&mut field, Coord { t: 10, c: 8, o: 8, v: 8 }, "B");
    seed_cluster(&mut field, Coord { t: 8, c: 10, o: 8, v: 8 }, "C");
    seed_cluster(&mut field, Coord { t: 8, c: 8, o: 10, v: 8 }, "D");
    evolve_to_equilibrium(&mut field, 10);
    assert!(field.get(target).crystallized);

    println!("  crystals before attack: {}", field.crystallized_count());

    let neighbors = field.neighbors(target);
    field.destroy(target);
    for n in neighbors { field.destroy(n); }

    let recovered = bench("recovery", || {
        evolve_to_equilibrium(&mut field, 20);
        field.get(target).crystallized
    });
    println!("  recovered: {}", recovered);
    assert!(recovered);
}

#[test]
fn scale_32_coexistence() {
    println!("\n=== Coexistence at 32⁴ ===");
    let mut field = Field::new(32);

    let events = vec![
        Coord { t: 4, c: 4, o: 4, v: 4 },
        Coord { t: 28, c: 28, o: 28, v: 28 },
        Coord { t: 4, c: 28, o: 16, v: 16 },
    ];

    bench("seed 3 clusters", || {
        for (i, ev) in events.iter().enumerate() {
            seed_cluster(&mut field, *ev, &format!("ev{}", i));
        }
    });

    println!("  active: {} / {} ({:.1}%)", field.active_cells(), field.total_cells(),
        field.active_cells() as f64 / field.total_cells() as f64 * 100.0);

    bench("evolve", || evolve_to_equilibrium(&mut field, 20));

    for ev in &events {
        assert!(field.get(*ev).crystallized, "Event at {} should crystallize", ev);
    }

    // Destroy first cluster — others survive
    let ev0 = events[0];
    field.destroy(ev0);
    for n in field.neighbors(ev0) { field.destroy(n); }
    evolve_to_equilibrium(&mut field, 20);

    for ev in &events[1..] {
        assert!(field.get(*ev).crystallized, "Event at {} should survive", ev);
    }
    println!("  coexistence: OK");
}

#[test]
fn scale_32_emergent_records() {
    println!("\n=== Emergent records at 32⁴ ===");
    let mut field = Field::new(32);

    let a = Coord { t: 15, c: 16, o: 16, v: 16 };
    let b = Coord { t: 17, c: 16, o: 16, v: 16 };
    let mid = Coord { t: 16, c: 16, o: 16, v: 16 };

    field.seed_named(a, "Alice→Bob:10");
    field.seed_named(b, "Bob→Carol:5");
    evolve_to_equilibrium(&mut field, 20);

    let mid_cell = field.get(mid);
    println!("  midpoint crystallized: {}", mid_cell.crystallized);
    println!("  record: {}", mid_cell.record());

    assert!(mid_cell.crystallized);
    assert!(mid_cell.influences.iter().any(|i| i.event_id.contains("Alice")));
    assert!(mid_cell.influences.iter().any(|i| i.event_id.contains("Carol")));
}
