//! Convergence and scale benchmarks for Tesseract.
//!
//! Measures the metrics that matter for production viability:
//! - Convergence time vs field size
//! - Throughput (events/sec) vs field size
//! - Memory (active cells) vs events
//! - Self-healing latency
//! - Crystallization efficiency

use std::time::Instant;
use tesseract::mapper::*;
use tesseract::*;

// --- Helpers ---

fn seed_cluster(field: &mut Field, center: Coord, name: &str) {
    let s = field.size;
    field.seed_named(center, &format!("{}-core", name));
    for offset in [1_i64, -1] {
        field.seed_named(
            Coord {
                t: ((center.t as i64 + offset).rem_euclid(s as i64)) as usize,
                ..center
            },
            &format!("{}-t", name),
        );
        field.seed_named(
            Coord {
                c: ((center.c as i64 + offset).rem_euclid(s as i64)) as usize,
                ..center
            },
            &format!("{}-c", name),
        );
        field.seed_named(
            Coord {
                o: ((center.o as i64 + offset).rem_euclid(s as i64)) as usize,
                ..center
            },
            &format!("{}-o", name),
        );
        field.seed_named(
            Coord {
                v: ((center.v as i64 + offset).rem_euclid(s as i64)) as usize,
                ..center
            },
            &format!("{}-v", name),
        );
    }
}

fn timed<F: FnOnce() -> R, R>(f: F) -> (R, std::time::Duration) {
    let start = Instant::now();
    let r = f();
    (r, start.elapsed())
}

fn evolve_counting(field: &mut Field, stable_for: usize) -> (usize, std::time::Duration) {
    let start = Instant::now();
    let mut stable = 0;
    let mut steps = 0;
    for _ in 1..=MAX_ITERATIONS {
        steps += 1;
        if field.evolve() == 0 {
            stable += 1;
        } else {
            stable = 0;
        }
        if stable >= stable_for {
            break;
        }
    }
    (steps, start.elapsed())
}

// === 1. Convergence Time vs Field Size ===

#[test]
fn bench_convergence_by_size() {
    println!("\n╔══════════════════════════════════════════════════════════╗");
    println!("║  CONVERGENCE TIME vs FIELD SIZE (2 clusters per field)  ║");
    println!("╠════════╦══════════╦════════╦═══════════╦═══════════════╣");
    println!("║  Size  ║ Logical  ║ Active ║ Converge  ║ Crystal/Active║");
    println!("╠════════╬══════════╬════════╬═══════════╬═══════════════╣");

    for size in [8, 16, 32, 64] {
        let mut field = Field::new(size);
        let q = size / 4;
        seed_cluster(
            &mut field,
            Coord {
                t: q,
                c: q,
                o: q,
                v: q,
            },
            "A",
        );
        seed_cluster(
            &mut field,
            Coord {
                t: 3 * q,
                c: 3 * q,
                o: 3 * q,
                v: 3 * q,
            },
            "B",
        );

        let active_before = field.active_cells();
        let (steps, elapsed) = evolve_counting(&mut field, 10);
        let crystals = field.crystallized_count();

        println!(
            "║  {:>3}⁴  ║ {:>8} ║ {:>6} ║ {:>5.2?} {:>2}s ║ {:>5}/{:<5} {:>3}%║",
            size,
            size.pow(4),
            active_before,
            elapsed,
            steps,
            crystals,
            active_before,
            if active_before > 0 {
                crystals * 100 / active_before
            } else {
                0
            }
        );
    }

    println!("╚════════╩══════════╩════════╩═══════════╩═══════════════╝");
}

// === 2. Throughput (events/sec) ===

#[test]
fn bench_throughput() {
    println!("\n╔═══════════════════════════════════════════════════════╗");
    println!("║  THROUGHPUT: events seeded + evolved to equilibrium  ║");
    println!("╠════════╦════════╦══════════╦══════════╦══════════════╣");
    println!("║  Size  ║ Events ║ Seed     ║ Evolve   ║ Events/sec   ║");
    println!("╠════════╬════════╬══════════╬══════════╬══════════════╣");

    for (size, count) in [(8, 50), (16, 50), (32, 50)] {
        let mapper = CoordMapper::new(size).with_time_bucket(10);
        let mut field = Field::new(size);

        let (_, seed_time) = timed(|| {
            for i in 0..count {
                let ev = Event {
                    id: format!("tx-{}", i),
                    timestamp: 100 + (i as u64 / 10),
                    channel: format!("ch-{}", i % 4),
                    org: format!("org-{}", i % 8),
                    data: format!("payload-{}", i),
                };
                let coord = mapper.map(&ev);
                field.seed_named(coord, &ev.data);
            }
        });

        let (_, evolve_time) = timed(|| evolve_to_equilibrium(&mut field, 10));
        let total = seed_time + evolve_time;
        let eps = count as f64 / total.as_secs_f64();

        println!(
            "║  {:>3}⁴  ║  {:>5} ║ {:>8.2?} ║ {:>8.2?} ║ {:>10.0}/s  ║",
            size, count, seed_time, evolve_time, eps
        );
    }

    println!("╚════════╩════════╩══════════╩══════════╩══════════════╝");
}

// === 3. Memory Footprint ===

#[test]
fn bench_memory() {
    println!("\n╔═══════════════════════════════════════════════════╗");
    println!("║  MEMORY: active cells vs events (field size 32)  ║");
    println!("╠═════════╦══════════╦═══════════╦════════════════╣");
    println!("║ Events  ║ Active   ║ Crystal   ║ Sparsity       ║");
    println!("╠═════════╬══════════╬═══════════╬════════════════╣");

    let mapper = CoordMapper::new(32).with_time_bucket(10);
    let mut field = Field::new(32);
    let total_cells = 32_usize.pow(4);

    for batch in [10, 25, 50, 75, 100] {
        // Seed up to this many total events
        let current = field.active_cells();
        let to_add = if batch > 0 { batch } else { 0 };
        for i in 0..to_add {
            let ev = Event {
                id: format!("tx-{}-{}", batch, i),
                timestamp: 100 + (i as u64),
                channel: format!("ch-{}", i % 6),
                org: format!("org-{}", i % 10),
                data: format!("d-{}", i),
            };
            let coord = mapper.map(&ev);
            field.seed_named(coord, &ev.data);
        }
        evolve_to_equilibrium(&mut field, 10);

        let active = field.active_cells();
        let crystals = field.crystallized_count();
        let sparsity = 100.0 - (active as f64 / total_cells as f64 * 100.0);

        println!(
            "║  {:>5}  ║  {:>6}  ║   {:>6}  ║  {:>6.2}% empty ║",
            batch, active, crystals, sparsity
        );
    }

    println!("╚═════════╩══════════╩═══════════╩════════════════╝");
}

// === 4. Self-Healing Latency ===

#[test]
fn bench_self_healing() {
    println!("\n╔═══════════════════════════════════════════════════════╗");
    println!("║  SELF-HEALING: destroy center + neighbors, measure   ║");
    println!("║  time to re-crystallize                              ║");
    println!("╠════════╦══════════════╦══════════════╦═══════════════╣");
    println!("║  Size  ║ Destroy time ║ Heal time    ║ Recovered     ║");
    println!("╠════════╬══════════════╬══════════════╬═══════════════╣");

    for size in [8, 16, 32] {
        let mut field = Field::new(size);
        let q = size / 4;
        let target = Coord {
            t: q,
            c: q,
            o: q,
            v: q,
        };

        // Build dense neighborhood
        seed_cluster(&mut field, target, "A");
        seed_cluster(
            &mut field,
            Coord {
                t: q + 2,
                c: q,
                o: q,
                v: q,
            },
            "B",
        );
        seed_cluster(
            &mut field,
            Coord {
                t: q,
                c: q + 2,
                o: q,
                v: q,
            },
            "C",
        );
        seed_cluster(
            &mut field,
            Coord {
                t: q,
                c: q,
                o: q + 2,
                v: q,
            },
            "D",
        );
        evolve_to_equilibrium(&mut field, 10);

        // Destroy
        let (_, destroy_time) = timed(|| {
            let neighbors = field.neighbors(target);
            field.destroy(target);
            for n in neighbors {
                field.destroy(n);
            }
        });

        // Heal — use full equilibrium (may need many steps for diffusion)
        let (steps, heal_time) = evolve_counting(&mut field, 20);
        let recovered = field.get(target).crystallized;

        println!(
            "║  {:>3}⁴  ║ {:>12.2?} ║ {:>12.2?} ║ {:>6} ({:>2}st) ║",
            size,
            destroy_time,
            heal_time,
            if recovered { "YES" } else { "NO" },
            steps
        );
    }

    println!("╚════════╩══════════════╩══════════════╩═══════════════╝");
}

// === 5. Seed Cost ===

#[test]
fn bench_seed_cost() {
    println!("\n╔════════════════════════════════════════════╗");
    println!("║  SEED COST: single event by field size     ║");
    println!("╠════════╦════════════╦═══════════════════════╣");
    println!("║  Size  ║ Seed time  ║ Cells created         ║");
    println!("╠════════╬════════════╬═══════════════════════╣");

    for size in [8, 16, 32, 64, 128] {
        let mut field = Field::new(size);
        let center = Coord {
            t: size / 2,
            c: size / 2,
            o: size / 2,
            v: size / 2,
        };

        let (_, elapsed) = timed(|| field.seed_named(center, "bench"));

        println!(
            "║  {:>4}⁴ ║ {:>10.2?} ║ {:>6} cells           ║",
            size,
            elapsed,
            field.active_cells()
        );
    }

    println!("╚════════╩════════════╩═══════════════════════╝");
}
