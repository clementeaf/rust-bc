//! Tesseract — 4D probability field
//!
//! Sparse implementation: only cells with p > 0 are stored.
//! Scales to large fields (32⁴ = ~1M logical cells) without
//! allocating memory for empty space.

pub mod mapper;
pub mod node;
pub mod wallet;
pub mod identity;
pub mod persistence;
pub mod economics;
pub mod contribution;

use std::collections::HashMap;
use std::fmt;

// --- Configuration ---

pub const CRYSTALLIZATION_THRESHOLD: f64 = 0.85;
pub const INFLUENCE_FACTOR: f64 = 0.15;
pub const MAX_ITERATIONS: usize = 500;
/// Minimum probability to store a cell. Below this → treat as 0.
/// This is the practical cutoff for Axiom 4 (Orbital Completeness):
/// theoretically ΔP(x) > 0 everywhere, but cells below EPSILON are
/// not stored. See docs/TESSERACT-AXIOMS.md for the formal treatment.
pub const EPSILON: f64 = 0.05;
/// Minimum influence weight to record. Below this → discard.
pub const INFLUENCE_EPSILON: f64 = 0.05;
/// Maximum Euclidean radius for orbital seeding. Cells beyond this get no probability.
/// At SEED_RADIUS=4: p(d=4) = 0.20, still meaningful. Beyond → negligible.
pub const SEED_RADIUS: usize = 4;

// --- Helpers ---

fn wrapping_dist(a: usize, b: usize, size: usize) -> usize {
    let a = a % size;
    let b = b % size;
    let d = if a > b { a - b } else { b - a };
    d.min(size.saturating_sub(d))
}

/// Euclidean distance in 4D toroidal space.
pub fn distance(a: Coord, b: Coord, size: usize) -> f64 {
    let dt = wrapping_dist(a.t, b.t, size) as f64;
    let dc = wrapping_dist(a.c, b.c, size) as f64;
    let do_ = wrapping_dist(a.o, b.o, size) as f64;
    let dv = wrapping_dist(a.v, b.v, size) as f64;
    (dt * dt + dc * dc + do_ * do_ + dv * dv).sqrt()
}

// --- Core types ---

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Coord {
    pub t: usize,
    pub c: usize,
    pub o: usize,
    pub v: usize,
}

impl fmt::Display for Coord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({},{},{},{})", self.t, self.c, self.o, self.v)
    }
}

#[derive(Clone, Debug)]
pub struct Influence {
    pub event_id: String,
    pub weight: f64,
}

#[derive(Clone, Debug)]
pub struct Cell {
    pub probability: f64,
    pub crystallized: bool,
    pub influences: Vec<Influence>,
}

impl Cell {
    pub fn new() -> Self {
        Self { probability: 0.0, crystallized: false, influences: Vec::new() }
    }

    pub fn record(&self) -> String {
        if self.influences.is_empty() {
            return String::from("(empty)");
        }
        let mut sorted = self.influences.clone();
        sorted.sort_by(|a, b| b.weight.partial_cmp(&a.weight).unwrap());
        sorted.iter()
            .map(|inf| format!("{}({:.0}%)", inf.event_id, inf.weight * 100.0))
            .collect::<Vec<_>>()
            .join(" + ")
    }
}

/// Sparse 4D probability field.
/// Only cells with p > EPSILON are stored.
pub struct Field {
    cells: HashMap<Coord, Cell>,
    pub size: usize,
    /// Regional curvature capacity: how much deformation each region can sustain.
    /// Key is the region identifier (typically the o-axis value = org/identity).
    /// When capacity reaches 0, no more deformations can be seeded in that region.
    /// This is a GEOMETRIC constraint of the space, not an economic rule.
    curvature_budget: HashMap<usize, f64>,
}

impl Field {
    pub fn new(size: usize) -> Self {
        Self { cells: HashMap::new(), size, curvature_budget: HashMap::new() }
    }

    /// Number of cells actually stored in memory.
    pub fn active_cells(&self) -> usize {
        self.cells.len()
    }

    /// Total logical cells in the field (S⁴).
    pub fn total_cells(&self) -> usize {
        self.size.pow(4)
    }

    // --- Curvature budget ---

    /// Set the curvature capacity for a region.
    /// Region is identified by the o-axis value (identity/org dimension).
    /// This defines how much the space CAN bend in that region.
    pub fn set_capacity(&mut self, region: usize, capacity: f64) {
        self.curvature_budget.insert(region, capacity);
    }

    /// Get curvature capacity for a region.
    pub fn capacity(&self, region: usize) -> Option<f64> {
        self.curvature_budget.get(&region).copied()
    }

    /// Set or add curvature capacity for a region.
    pub fn add_capacity(&mut self, region: usize, amount: f64) {
        let current = self.curvature_budget.entry(region).or_insert(0.0);
        *current += amount;
    }

    /// Count of crystallized cells in a region.
    pub fn curvature_load(&self, region: usize) -> f64 {
        self.cells.iter()
            .filter(|(coord, cell)| coord.o == region && cell.crystallized)
            .count() as f64
    }

    /// **Binding energy** of a crystallized cell: how strongly the
    /// surrounding field anchors it in place. Range [0.0, 1.0].
    ///
    /// Definition:
    ///   BE(x) = (crystallized_neighbors / 8) × (σ(x) / 4)
    ///
    /// Where:
    ///   - crystallized_neighbors: count of 8 direct neighbors that are crystallized
    ///   - σ(x): orthogonal support (0-4 axes with high-probability neighbors)
    ///
    /// A cell with 8 crystallized neighbors and 4-axis support has BE = 1.0.
    /// A cell with 1 crystallized neighbor and 1-axis support has BE = 0.03.
    ///
    /// Physical analogy: lattice binding energy — how much energy to
    /// remove this atom from the crystal lattice.
    pub fn binding_energy(&self, coord: Coord) -> f64 {
        let neighbors = self.neighbors(coord);
        let crystal_neighbors = neighbors.iter()
            .filter(|n| self.get(**n).crystallized)
            .count() as f64;
        let neighbor_ratio = crystal_neighbors / 8.0;
        let support_ratio = self.orthogonal_support(coord) as f64 / 4.0;
        neighbor_ratio * support_ratio
    }

    pub fn get(&self, coord: Coord) -> &Cell {
        static EMPTY: std::sync::LazyLock<Cell> = std::sync::LazyLock::new(Cell::new);
        self.cells.get(&coord).unwrap_or(&EMPTY)
    }

    pub fn get_mut(&mut self, coord: Coord) -> &mut Cell {
        self.cells.entry(coord).or_insert_with(Cell::new)
    }

    pub fn seed(&mut self, center: Coord) {
        self.seed_named(center, &format!("ev@{}", center));
    }

    /// Seed a named event. No pre-validation. No locks. No rejection.
    /// The event enters the field freely. If the region is over-capacity,
    /// the field's evolution physics will decay the weakest crystallizations.
    /// Both competing deformations exist — the field decides which survives.
    pub fn seed_named(&mut self, center: Coord, event_id: &str) {
        let s = self.size;
        let axis_max = SEED_RADIUS.min(s / 2);

        for dt_signed in -(axis_max as i64)..=(axis_max as i64) {
            let t = ((center.t as i64 + dt_signed).rem_euclid(s as i64)) as usize;
            for dc_signed in -(axis_max as i64)..=(axis_max as i64) {
                let c = ((center.c as i64 + dc_signed).rem_euclid(s as i64)) as usize;
                for do_signed in -(axis_max as i64)..=(axis_max as i64) {
                    let o = ((center.o as i64 + do_signed).rem_euclid(s as i64)) as usize;
                    for dv_signed in -(axis_max as i64)..=(axis_max as i64) {
                        let v = ((center.v as i64 + dv_signed).rem_euclid(s as i64)) as usize;
                        let coord = Coord { t, c, o, v };

                        let dist = distance(center, coord, s);
                        let p = 1.0 / (1.0 + dist);

                        if p < EPSILON { continue; }

                        let cell = self.get_mut(coord);
                        cell.probability = (cell.probability + p).min(1.0);

                        if p >= INFLUENCE_EPSILON {
                            cell.influences.push(Influence {
                                event_id: event_id.to_string(),
                                weight: p,
                            });
                        }

                        if !cell.crystallized && cell.probability >= CRYSTALLIZATION_THRESHOLD {
                            cell.crystallized = true;
                            cell.probability = 1.0;
                        }
                    }
                }
            }
        }
    }

    pub fn destroy(&mut self, coord: Coord) {
        if let Some(cell) = self.cells.get_mut(&coord) {
            cell.probability = 0.0;
            cell.crystallized = false;
        }
    }

    pub fn neighbors(&self, coord: Coord) -> [Coord; 8] {
        let s = self.size;
        [
            Coord { t: (coord.t + 1) % s, ..coord },
            Coord { t: (coord.t + s - 1) % s, ..coord },
            Coord { c: (coord.c + 1) % s, ..coord },
            Coord { c: (coord.c + s - 1) % s, ..coord },
            Coord { o: (coord.o + 1) % s, ..coord },
            Coord { o: (coord.o + s - 1) % s, ..coord },
            Coord { v: (coord.v + 1) % s, ..coord },
            Coord { v: (coord.v + s - 1) % s, ..coord },
        ]
    }

    pub fn orthogonal_support(&self, coord: Coord) -> usize {
        let s = self.size;
        let check = |c: Coord| self.get(c).probability > 0.5;
        let mut axes = 0;
        if check(Coord { t: (coord.t + 1) % s, ..coord }) || check(Coord { t: (coord.t + s - 1) % s, ..coord }) { axes += 1; }
        if check(Coord { c: (coord.c + 1) % s, ..coord }) || check(Coord { c: (coord.c + s - 1) % s, ..coord }) { axes += 1; }
        if check(Coord { o: (coord.o + 1) % s, ..coord }) || check(Coord { o: (coord.o + s - 1) % s, ..coord }) { axes += 1; }
        if check(Coord { v: (coord.v + 1) % s, ..coord }) || check(Coord { v: (coord.v + s - 1) % s, ..coord }) { axes += 1; }
        axes
    }

    /// One evolution step. Only processes active cells and their neighbors.
    pub fn evolve(&mut self) -> usize {
        // Collect coords to process: all active cells + their neighbors
        let mut to_process: Vec<Coord> = Vec::new();
        let active_coords: Vec<Coord> = self.cells.keys().copied().collect();
        let mut seen = HashMap::with_capacity(active_coords.len() * 9);

        for coord in &active_coords {
            if seen.insert(*coord, true).is_none() {
                to_process.push(*coord);
            }
            for n in self.neighbors(*coord) {
                if seen.insert(n, true).is_none() {
                    to_process.push(n);
                }
            }
        }

        // Calculate new probabilities
        let updates: Vec<(Coord, f64)> = to_process.iter()
            .filter(|coord| !self.get(**coord).crystallized)
            .map(|coord| {
                let cell_p = self.get(*coord).probability;
                let neighbors = self.neighbors(*coord);
                let neighbor_avg: f64 = neighbors.iter()
                    .map(|n| self.get(*n).probability)
                    .sum::<f64>() / 8.0;

                let delta = (neighbor_avg - cell_p) * INFLUENCE_FACTOR;
                let support = self.orthogonal_support(*coord);
                let (amp, res) = match support {
                    0 | 1 => (1.0, 0.0),
                    2 => (1.5, 0.02),
                    3 => (2.5, 0.05),
                    4 => (4.0, 0.10),
                    _ => (1.0, 0.0),
                };
                let new_p = (cell_p + delta * amp + res).clamp(0.0, 1.0);
                (*coord, new_p)
            })
            .collect();

        // Apply updates
        let mut new_crystallizations = 0;
        for (coord, new_p) in updates {
            if new_p < EPSILON {
                if let Some(cell) = self.cells.get(&coord) {
                    if cell.influences.is_empty() {
                        self.cells.remove(&coord);
                        continue;
                    }
                }
            }

            let cell = self.get_mut(coord);
            cell.probability = new_p;
            if !cell.crystallized && new_p >= CRYSTALLIZATION_THRESHOLD {
                cell.crystallized = true;
                cell.probability = 1.0;
                new_crystallizations += 1;
            }
        }

        // --- Curvature pressure (progressive decay) ---
        //
        // When a region's crystallized load exceeds its geometric capacity,
        // the field applies decay pressure. This is NOT rejection — it is
        // the space itself unable to sustain the deformation.
        //
        // DEFINITIONS:
        //
        // 1. "Weakest" = lowest binding energy.
        //    BE(x) = (crystallized_neighbors/8) × (orthogonal_support/4)
        //    Range [0.0, 1.0]. Measures how deeply embedded a cell is
        //    in the surrounding lattice. Like atomic binding energy.
        //
        // 2. Decay is PROGRESSIVE, not instant.
        //    Each step, over-capacity cells receive decay pressure:
        //      decay(x) = excess_ratio × (1 - BE(x))
        //    Strong cells (high BE) resist. Weak cells (low BE) decay fast.
        //    When probability drops below Θ → un-crystallizes.
        //    Probability doesn't go to zero — residue remains.
        //    Cell can RE-EMERGE if competing deformations also decay.
        //
        // 3. Over-capacity criterion:
        //    excess = load(region) - capacity(region)
        //    excess_ratio = excess / load    ∈ (0, 1)
        //    When excess_ratio = 0: no pressure.
        //    When excess_ratio → 1: extreme pressure (nearly all must go).
        //
        if !self.curvature_budget.is_empty() {
            let regions: Vec<usize> = self.curvature_budget.keys().copied().collect();
            for region in regions {
                let capacity = self.curvature_budget[&region];
                let load = self.curvature_load(region);

                if load <= capacity { continue; }

                let excess_ratio = (load - capacity) / load;

                // Collect crystallized cells with their binding energy
                let region_crystals: Vec<(Coord, f64)> = self.cells.iter()
                    .filter(|(coord, cell)| coord.o == region && cell.crystallized)
                    .map(|(coord, _)| (*coord, self.binding_energy(*coord)))
                    .collect();

                // Sort by binding energy: weakest first
                let mut sorted_crystals = region_crystals;
                sorted_crystals.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

                // Progressively decay from weakest until load ≤ capacity.
                // Each decayed cell reduces load by 1. Decay is definitive
                // per step but the cell retains probability residue —
                // it can re-crystallize on a future step if conditions change
                // (e.g., a competitor also decayed, freeing capacity).
                let mut current_load = load;
                for (coord, _be) in &sorted_crystals {
                    if current_load <= capacity { break; }

                    if let Some(cell) = self.cells.get_mut(coord) {
                        cell.crystallized = false;
                        // Residue: probability drops but doesn't vanish.
                        // The deformation happened — it just can't stabilize.
                        cell.probability = (cell.probability - excess_ratio).max(EPSILON);
                        current_load -= 1.0;
                    }
                }
            }
        }

        new_crystallizations
    }

    /// Iterate over all active (stored) cells.
    pub fn active_entries(&self) -> impl Iterator<Item = (Coord, &Cell)> {
        self.cells.iter().map(|(k, v)| (*k, v))
    }

    pub fn crystallized_count(&self) -> usize {
        self.cells.values().filter(|c| c.crystallized).count()
    }

    pub fn crystallized_cells(&self) -> Vec<Coord> {
        self.cells.iter()
            .filter(|(_, c)| c.crystallized)
            .map(|(coord, _)| *coord)
            .collect()
    }

    pub fn is_consistent(&self) -> bool {
        self.crystallized_cells().iter().all(|coord| self.orthogonal_support(*coord) >= 2)
    }
}

/// Evolve a field to equilibrium.
pub fn evolve_to_equilibrium(field: &mut Field, stable_for: usize) {
    let mut stable = 0;
    for _ in 1..=MAX_ITERATIONS {
        if field.evolve() == 0 { stable += 1; } else { stable = 0; }
        if stable >= stable_for { break; }
    }
}
