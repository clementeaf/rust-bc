//! Tesseract — 4D geometric certainty field
//!
//! Data persists not because someone stores it, but because independent
//! evidence from orthogonal dimensions converges on the same fact.
//!
//! Core model:
//!   - Each dimension (Temporal, Context, Origin, Verification) is backed
//!     by structurally independent evidence sources.
//!   - σ measures PROVEN INDEPENDENCE: how many dimensions have attestations
//!     from validators bound to that dimension.
//!   - Crystallization requires σ = 4: all dimensions independently attested.
//!   - Security scales multiplicatively with dimensions, not additively.
//!
//! Sparse implementation: only cells with p > 0 are stored.

pub mod adversarial;
pub mod causality;
pub mod conservation;
pub mod contraction;
pub mod crystallization;
pub mod entropy;
pub mod lyapunov;
pub mod gravity;
pub mod proof;
pub mod mapper;
pub mod node;
pub mod wallet;
pub mod identity;
pub mod persistence;
pub mod economics;
pub mod contribution;
pub mod p2p;

use std::collections::HashMap;
use std::collections::HashSet;
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
/// Maximum axis radius for orbital seeding. Cells beyond this get no probability.
/// Capped at size/4 to prevent overlapping orbitals in small fields.
/// At SEED_RADIUS=3: (2×3+1)⁴ = 2401 cells per seed — good balance.
pub const SEED_RADIUS: usize = 3;
/// Probability boost pushed to neighbors when a cell crystallizes.
/// Creates dynamic correlation beyond the seed radius (crystallization cascade).
pub const CASCADE_STRENGTH: f64 = 0.08;

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

/// The 4 orthogonal dimensions of the tesseract.
/// Each is backed by a structurally independent class of evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Dimension {
    /// When was this observed? Independent clocks/timestamps.
    Temporal,
    /// In what channel/context did this occur?
    Context,
    /// Which independent entity attests to this?
    Origin,
    /// Against what state was this verified?
    Verification,
}

impl Dimension {
    pub const ALL: [Dimension; 4] = [
        Dimension::Temporal,
        Dimension::Context,
        Dimension::Origin,
        Dimension::Verification,
    ];
}

impl fmt::Display for Dimension {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Dimension::Temporal => write!(f, "T"),
            Dimension::Context => write!(f, "C"),
            Dimension::Origin => write!(f, "O"),
            Dimension::Verification => write!(f, "V"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
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

/// An attestation bound to ONE dimension.
/// A validator can only attest on the dimension it is bound to.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Attestation {
    /// Which dimension this attestation covers.
    pub dimension: Dimension,
    /// Unique identifier of the validator (bound to this dimension).
    pub validator_id: String,
    /// The event being attested.
    pub event_id: String,
    /// Strength of the attestation (distance-decayed from seed center).
    pub weight: f64,
}

/// Legacy influence — kept for backward compatibility during migration.
/// Will be removed once all modules use Attestation.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Influence {
    pub event_id: String,
    pub weight: f64,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Cell {
    pub probability: f64,
    pub crystallized: bool,
    /// Legacy influences (undifferentiated by dimension).
    pub influences: Vec<Influence>,
    /// Dimension-bound attestations: the new model.
    /// A cell crystallizes only when all 4 dimensions are attested.
    pub attestations: HashMap<Dimension, Vec<Attestation>>,
}

impl Cell {
    pub fn new() -> Self {
        Self {
            probability: 0.0,
            crystallized: false,
            influences: Vec::new(),
            attestations: HashMap::new(),
        }
    }

    /// How many dimensions have at least one attestation from a unique validator.
    pub fn attested_dimensions(&self) -> usize {
        self.attestations.iter()
            .filter(|(_, atts)| !atts.is_empty())
            .count()
    }

    /// Set of unique validator IDs across all dimensions.
    pub fn unique_validators(&self) -> HashSet<&str> {
        self.attestations.values()
            .flat_map(|atts| atts.iter().map(|a| a.validator_id.as_str()))
            .collect()
    }

    /// σ-independence: dimensions attested by validators that DO NOT
    /// appear on any other dimension for this cell. Measures true
    /// structural independence.
    pub fn sigma_independence(&self) -> usize {
        // Count how many times each validator appears across dimensions
        let mut validator_dims: HashMap<&str, HashSet<Dimension>> = HashMap::new();
        for (dim, atts) in &self.attestations {
            for att in atts {
                validator_dims.entry(att.validator_id.as_str())
                    .or_default()
                    .insert(*dim);
            }
        }

        // A dimension is independently attested if it has at least one
        // validator that attests ONLY on that dimension (not on others).
        let mut independent = 0;
        for dim in &Dimension::ALL {
            if let Some(atts) = self.attestations.get(dim) {
                let has_exclusive = atts.iter().any(|att| {
                    validator_dims.get(att.validator_id.as_str())
                        .map(|dims| dims.len() == 1)
                        .unwrap_or(false)
                });
                if has_exclusive {
                    independent += 1;
                }
            }
        }
        independent
    }

    pub fn record(&self) -> String {
        // Prefer attestation-based record if available
        if !self.attestations.is_empty() {
            let mut parts = Vec::new();
            for dim in &Dimension::ALL {
                if let Some(atts) = self.attestations.get(dim) {
                    if !atts.is_empty() {
                        let validators: Vec<String> = atts.iter()
                            .map(|a| format!("{}({:.0}%)", a.validator_id, a.weight * 100.0))
                            .collect();
                        parts.push(format!("[{}:{}]", dim, validators.join("+")));
                    }
                }
            }
            if !parts.is_empty() {
                return parts.join(" ");
            }
        }

        // Fallback to legacy influences
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
    curvature_budget: HashMap<usize, f64>,
    /// Causal graph: partial order of events.
    pub causality: Option<causality::CausalGraph>,
    /// Conserved value layer.
    pub conservation: Option<conservation::ConservedField>,
    /// Dirty set: cells that changed in the last step.
    /// Only these + their neighbors are processed in the next evolve().
    /// Empty = process all (first step or after seeding).
    dirty: HashSet<Coord>,
}

impl Field {
    pub fn new(size: usize) -> Self {
        Self {
            cells: HashMap::new(),
            size,
            curvature_budget: HashMap::new(),
            causality: None,
            conservation: None,
            dirty: HashSet::new(),
        }
    }

    /// Enable causal mode: the field becomes relativistic.
    /// Events must propagate through light cones, not instantaneously.
    pub fn with_causality(mut self) -> Self {
        self.causality = Some(causality::CausalGraph::new());
        self
    }

    /// Enable conservation: value becomes a physical invariant.
    /// Must call `genesis()` on the conservation field to inject initial supply.
    pub fn with_conservation(mut self) -> Self {
        self.conservation = Some(conservation::ConservedField::new());
        self
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

    /// Attest an event from a specific dimension.
    /// This is the new model: each attestation is bound to ONE dimension,
    /// from a validator that is bound to that dimension.
    /// Crystallization requires attestations from all 4 dimensions.
    pub fn attest(
        &mut self,
        center: Coord,
        event_id: &str,
        dimension: Dimension,
        validator_id: &str,
    ) {
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

                        // Record dimension-bound attestation
                        if p >= INFLUENCE_EPSILON {
                            let atts = cell.attestations
                                .entry(dimension)
                                .or_insert_with(Vec::new);
                            // Avoid duplicate attestations from same validator
                            let already = atts.iter()
                                .any(|a| a.validator_id == validator_id && a.event_id == event_id);
                            if !already {
                                atts.push(Attestation {
                                    dimension,
                                    validator_id: validator_id.to_string(),
                                    event_id: event_id.to_string(),
                                    weight: p,
                                });
                            }
                        }

                        // Crystallization now requires σ-independence = 4
                        if !cell.crystallized
                            && cell.probability >= CRYSTALLIZATION_THRESHOLD
                            && cell.sigma_independence() >= 4
                        {
                            cell.crystallized = true;
                            cell.probability = 1.0;
                        }
                    }
                }
            }
        }

        self.apply_cascade(event_id);
    }

    /// Causal attestation: like `attest()` but respects light cones.
    /// Probability only reaches cells within the event's causal cone.
    /// Returns the EventId if accepted, None if causality is disabled or
    /// a parent is unknown.
    pub fn attest_causal(
        &mut self,
        center: Coord,
        data: &[u8],
        parents: Vec<causality::EventId>,
        dimension: Dimension,
        validator_id: &str,
    ) -> Option<causality::EventId> {
        let graph = self.causality.as_mut()?;
        let logical_time = graph.current_time;

        let event = causality::CausalEvent::new(
            center, logical_time, parents, data.to_vec(),
        );
        let event_id = event.id.clone();
        let event_id_str = event_id.to_string();

        if !graph.insert(event) {
            return None; // unknown parent — causal violation
        }

        // Now seed probability, but ONLY within the light cone.
        let s = self.size;
        let axis_max = SEED_RADIUS.min(s / 2);
        let cone = causality::LightCone::new(center, logical_time);
        // Use current_time from graph (which advanced after insert)
        let now = self.causality.as_ref().unwrap().current_time;

        for dt_signed in -(axis_max as i64)..=(axis_max as i64) {
            let t = ((center.t as i64 + dt_signed).rem_euclid(s as i64)) as usize;
            for dc_signed in -(axis_max as i64)..=(axis_max as i64) {
                let c = ((center.c as i64 + dc_signed).rem_euclid(s as i64)) as usize;
                for do_signed in -(axis_max as i64)..=(axis_max as i64) {
                    let o = ((center.o as i64 + do_signed).rem_euclid(s as i64)) as usize;
                    for dv_signed in -(axis_max as i64)..=(axis_max as i64) {
                        let v = ((center.v as i64 + dv_signed).rem_euclid(s as i64)) as usize;
                        let coord = Coord { t, c, o, v };

                        // LIGHT CONE CHECK: skip cells outside causal reach
                        if !cone.can_reach(coord, now, s) {
                            continue;
                        }

                        let dist = distance(center, coord, s);
                        let p = 1.0 / (1.0 + dist);

                        if p < EPSILON { continue; }

                        let cell = self.get_mut(coord);
                        cell.probability = (cell.probability + p).min(1.0);

                        if p >= INFLUENCE_EPSILON {
                            let atts = cell.attestations
                                .entry(dimension)
                                .or_insert_with(Vec::new);
                            let already = atts.iter()
                                .any(|a| a.validator_id == validator_id && a.event_id == event_id_str);
                            if !already {
                                atts.push(Attestation {
                                    dimension,
                                    validator_id: validator_id.to_string(),
                                    event_id: event_id_str.clone(),
                                    weight: p,
                                });
                            }
                        }

                        if !cell.crystallized
                            && cell.probability >= CRYSTALLIZATION_THRESHOLD
                            && cell.sigma_independence() >= 4
                        {
                            cell.crystallized = true;
                            cell.probability = 1.0;
                        }
                    }
                }
            }
        }

        self.apply_cascade(&event_id_str);
        Some(event_id)
    }

    /// Advance the field's causal clock by one tick.
    /// Light cones expand — events that couldn't reach distant cells before
    /// now can. Call this between attestation rounds.
    pub fn tick(&mut self) {
        if let Some(ref mut graph) = self.causality {
            graph.current_time += 1;
        }
    }

    pub fn seed(&mut self, center: Coord) {
        self.seed_named(center, &format!("ev@{}", center));
    }

    /// Seed a named event (legacy mode — no dimension binding).
    /// Kept for backward compatibility. Use `attest()` for new code.
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

        // Mark all seeded cells as dirty for next evolve()
        let seeded: Vec<Coord> = self.cells.keys().copied().collect();
        self.dirty.extend(seeded);

        // Crystallization cascade: newly crystallized cells push a small
        // probability boost to their neighbors. This creates dynamic
        // correlation beyond the seed radius and enables genuine
        // phase-transition behavior.
        self.apply_cascade(event_id);
    }

    /// Push cascade boosts from all crystallized cells.
    /// Used after seeding (all crystals cascade to spread the event's influence).
    fn apply_cascade(&mut self, source_event: &str) {
        let crystallized: Vec<Coord> = self.cells.iter()
            .filter(|(_, cell)| cell.crystallized)
            .map(|(coord, _)| *coord)
            .collect();
        self.cascade_boost(&crystallized, source_event);
    }

    /// Push cascade boosts from a specific set of cells.
    /// Used after evolution (only newly crystallized cells cascade).
    fn apply_cascade_from(&mut self, sources: &[Coord]) {
        self.cascade_boost(sources, "");
    }

    /// Shared cascade logic: boost neighbors of the given crystallized cells.
    /// Only boosts cells that ALREADY EXIST (have prior evidence).
    /// This prevents unbounded field growth from cascade alone.
    fn cascade_boost(&mut self, sources: &[Coord], source_event: &str) {
        // Collect existing neighbor coords first to avoid borrow issues
        let boosts: Vec<(Coord, bool)> = sources.iter()
            .flat_map(|coord| {
                self.neighbors(*coord).into_iter().map(|n| {
                    let exists = self.cells.contains_key(&n);
                    (n, exists)
                })
            })
            .collect();

        for (n, existed) in boosts {
            if !existed {
                continue; // Don't create new cells from cascade alone
            }
            let cell = self.get_mut(n);
            let old_p = cell.probability;
            let new_p = (old_p + CASCADE_STRENGTH).min(1.0);
            if new_p > old_p {
                cell.probability = new_p;
                if cell.influences.is_empty() && !source_event.is_empty() {
                    cell.influences.push(Influence {
                        event_id: source_event.to_string(),
                        weight: CASCADE_STRENGTH,
                    });
                }
            }
        }
    }

    pub fn destroy(&mut self, coord: Coord) {
        if let Some(cell) = self.cells.get_mut(&coord) {
            cell.probability = 0.0;
            cell.crystallized = false;
            // Mark destroyed cell + neighbors as dirty so evolve can regenerate
            self.dirty.insert(coord);
            for n in self.neighbors(coord) {
                self.dirty.insert(n);
            }
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

    /// Orthogonal support: σ-independence when attestations are present,
    /// legacy source-diversity check otherwise.
    ///
    /// New model (attestations):
    ///   σ = number of dimensions with at least one exclusive validator
    ///   (a validator that attests ONLY on that dimension for this cell).
    ///   This is geometric certainty: σ=4 means 4 structurally independent
    ///   evidence sources converge.
    ///
    /// Legacy model (influences):
    ///   σ = axes with diverse event sources among neighbors.
    pub fn orthogonal_support(&self, coord: Coord) -> usize {
        let cell = self.get(coord);

        // New model: use σ-independence from attestations
        if !cell.attestations.is_empty() {
            return cell.sigma_independence();
        }

        // Legacy fallback: source-diversity among neighbors
        let s = self.size;
        let my_sources: HashSet<&str> = cell
            .influences.iter().map(|i| i.event_id.as_str()).collect();

        if my_sources.is_empty() {
            let check = |c: Coord| self.get(c).probability > 0.5;
            let mut axes = 0;
            if check(Coord { t: (coord.t + 1) % s, ..coord }) || check(Coord { t: (coord.t + s - 1) % s, ..coord }) { axes += 1; }
            if check(Coord { c: (coord.c + 1) % s, ..coord }) || check(Coord { c: (coord.c + s - 1) % s, ..coord }) { axes += 1; }
            if check(Coord { o: (coord.o + 1) % s, ..coord }) || check(Coord { o: (coord.o + s - 1) % s, ..coord }) { axes += 1; }
            if check(Coord { v: (coord.v + 1) % s, ..coord }) || check(Coord { v: (coord.v + s - 1) % s, ..coord }) { axes += 1; }
            return axes;
        }

        let mut axes = 0;
        let axis_neighbors: [(Coord, Coord); 4] = [
            (Coord { t: (coord.t + 1) % s, ..coord }, Coord { t: (coord.t + s - 1) % s, ..coord }),
            (Coord { c: (coord.c + 1) % s, ..coord }, Coord { c: (coord.c + s - 1) % s, ..coord }),
            (Coord { o: (coord.o + 1) % s, ..coord }, Coord { o: (coord.o + s - 1) % s, ..coord }),
            (Coord { v: (coord.v + 1) % s, ..coord }, Coord { v: (coord.v + s - 1) % s, ..coord }),
        ];

        for (n_pos, n_neg) in &axis_neighbors {
            let has_diverse = [n_pos, n_neg].iter().any(|n| {
                self.get(**n).influences.iter()
                    .any(|inf| !my_sources.contains(inf.event_id.as_str()))
            });
            if has_diverse {
                axes += 1;
            }
        }
        axes
    }

    /// One evolution step. Processes dirty cells + their existing neighbors.
    /// If no dirty set, processes all (first step after seeding).
    /// Does NOT create new cells — field growth happens only via seeding.
    pub fn evolve(&mut self) -> usize {
        // Collect coords to process from dirty set (or all if empty/first run)
        let mut to_process: Vec<Coord> = Vec::new();
        let source_coords: Vec<Coord> = if self.dirty.is_empty() {
            self.cells.keys().copied().collect()
        } else {
            self.dirty.drain().collect()
        };
        let mut seen = HashMap::with_capacity(source_coords.len() * 9);

        for coord in &source_coords {
            if self.cells.contains_key(coord) && seen.insert(*coord, true).is_none() {
                to_process.push(*coord);
            }
            for n in self.neighbors(*coord) {
                if self.cells.contains_key(&n) && seen.insert(n, true).is_none() {
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

        // Apply updates — track which cells actually changed
        let mut new_crystallizations = 0;
        let mut newly_crystallized: Vec<Coord> = Vec::new();
        for (coord, new_p) in updates {
            if new_p < EPSILON {
                if let Some(cell) = self.cells.get(&coord) {
                    if cell.influences.is_empty() {
                        self.cells.remove(&coord);
                        self.dirty.insert(coord);
                        continue;
                    }
                }
            }

            let cell = self.cells.entry(coord).or_insert_with(|| Cell {
                probability: 0.0, crystallized: false,
                influences: Vec::new(), attestations: HashMap::new(),
            });
            let old_p = cell.probability;
            cell.probability = new_p;

            let changed = (new_p - old_p).abs() > 1e-6;

            if !cell.crystallized && new_p >= CRYSTALLIZATION_THRESHOLD {
                // Legacy threshold check passed — also check σ-independence
                // for cells with attestations (unified criterion).
                let has_attestations = !cell.attestations.is_empty();
                let sigma_ok = if has_attestations {
                    cell.sigma_independence() >= 4
                } else {
                    true
                };
                if sigma_ok {
                    cell.crystallized = true;
                    cell.probability = 1.0;
                    new_crystallizations += 1;
                    newly_crystallized.push(coord);
                }
            }

            // Mark dirty if probability changed meaningfully
            if changed || new_crystallizations > 0 {
                self.dirty.insert(coord);
            }
        }

        // Crystallization cascade: only from NEWLY crystallized cells.
        // Cascading from all crystals every step causes runaway growth.
        if new_crystallizations > 0 {
            self.apply_cascade_from(&newly_crystallized);
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

    /// Evaluate crystallization for all non-crystallized active cells
    /// using the given criterion. Returns coords that should crystallize.
    pub fn evaluate_crystallization(
        &self,
        criterion: &dyn crystallization::CrystallizationCriterion,
    ) -> Vec<crystallization::CrystallizationEval> {
        self.cells.iter()
            .filter(|(_, cell)| !cell.crystallized && cell.probability >= EPSILON)
            .map(|(coord, _)| criterion.evaluate(self, *coord))
            .filter(|eval| eval.should_crystallize())
            .collect()
    }

    /// Apply crystallization results: set cells to crystallized state.
    /// Returns number of new crystallizations.
    pub fn apply_crystallizations(&mut self, evals: &[crystallization::CrystallizationEval]) -> usize {
        let mut count = 0;
        let mut newly_crystallized = Vec::new();
        for eval in evals {
            let cell = self.get_mut(eval.coord);
            if !cell.crystallized {
                cell.crystallized = true;
                cell.probability = 1.0;
                count += 1;
                newly_crystallized.push(eval.coord);
            }
        }
        if count > 0 {
            self.apply_cascade_from(&newly_crystallized);
        }
        count
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
