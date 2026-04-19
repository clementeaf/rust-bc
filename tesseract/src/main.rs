//! Tesseract — run `cargo test` for the 10 experiments.
//! This binary is a minimal demo of the probability field.

use tesseract::*;

fn main() {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║           TESSERACT — 4D Probability Field              ║");
    println!("║     Convergence without consensus, validation, or trust ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();

    let mut field = Field::new(8);

    // Two events
    let a = Coord { t: 2, c: 3, o: 3, v: 3 };
    let b = Coord { t: 4, c: 3, o: 3, v: 3 };
    let mid = Coord { t: 3, c: 3, o: 3, v: 3 };

    field.seed_named(a, "Alice→Bob:10tok");
    field.seed_named(b, "Bob→Carol:5tok");
    evolve_to_equilibrium(&mut field, 20);

    println!("Events seeded. Field evolved to equilibrium.");
    println!();
    println!("  Alice→Bob at {}: crystallized={}, support={}/4", a, field.get(a).crystallized, field.orthogonal_support(a));
    println!("  Bob→Carol at {}: crystallized={}, support={}/4", b, field.get(b).crystallized, field.orthogonal_support(b));
    println!("  Midpoint  at {}: crystallized={}, support={}/4", mid, field.get(mid).crystallized, field.orthogonal_support(mid));
    println!();
    println!("  Emergent record: {}", field.get(mid).record());
    println!();
    println!("Run `cargo test` for all 10 experiments.");
}
