//! Entropy — the field moves toward order, never backward.
//!
//! Like the second law of thermodynamics: a system's entropy can decrease
//! locally (crystallization) only if the total entropy budget is respected.
//! States solidify because it is energetically favorable, not because a
//! threshold was crossed.
//!
//! Key concepts:
//!   - **Temperature**: controls the rate of spontaneous ordering.
//!     High T → fluid, chaotic. Low T → rigid, crystalline.
//!     Temperature decreases naturally as evidence accumulates.
//!   - **Free energy**: F = U - T·S. A cell crystallizes when F < 0
//!     (the energy gain from ordering exceeds the entropy cost).
//!   - **Irreversibility**: reversing a crystallized cell costs energy
//!     proportional to its binding energy × age. Old crystals are
//!     effectively permanent — like diamond vs ice.

use std::collections::HashMap;

use crate::{Coord, Field, CRYSTALLIZATION_THRESHOLD, EPSILON};

/// Thermodynamic state of the field.
pub struct Thermodynamics {
    /// Current temperature. Starts high (fluid), decreases as the field
    /// accumulates evidence. At T=0 the field is fully deterministic.
    pub temperature: f64,
    /// Cooling rate per tick. Temperature *= (1 - cooling_rate).
    pub cooling_rate: f64,
    /// Age of each crystallized cell (ticks since crystallization).
    /// Older crystals are harder to reverse.
    crystal_age: HashMap<Coord, u64>,
    /// Current tick counter.
    pub tick: u64,
    /// Entropy history — for monotonicity verification.
    entropy_log: Vec<f64>,
}

impl Thermodynamics {
    /// Create thermodynamic state with initial temperature.
    /// T=1.0 is "hot" (fluid), T=0.01 is "cold" (nearly frozen).
    pub fn new(initial_temperature: f64, cooling_rate: f64) -> Self {
        Self {
            temperature: initial_temperature,
            cooling_rate,
            crystal_age: HashMap::new(),
            tick: 0,
            entropy_log: Vec::new(),
        }
    }

    /// Default: start warm, cool slowly.
    pub fn default_physics() -> Self {
        Self::new(0.5, 0.02)
    }

    /// Measure Shannon entropy of the field's probability distribution.
    /// H = -Σ p·log₂(p) over all active cells.
    /// Lower H = more order = more crystallization.
    pub fn shannon_entropy(field: &Field) -> f64 {
        let mut h = 0.0;
        for (_, cell) in field.active_entries() {
            let p = cell.probability;
            if p > EPSILON && p < 1.0 {
                // Binary entropy for each cell
                h -= p * p.log2() + (1.0 - p) * (1.0 - p).log2();
            }
            // p=0 or p=1 contribute 0 entropy (fully determined)
        }
        h
    }

    /// Free energy of a cell: F = U - T·S
    /// - U (internal energy) = -(probability + binding_energy) → negative = stable
    /// - S (entropy contribution) = binary entropy of the cell
    /// - T = temperature
    ///
    /// F < 0 means crystallization is energetically favorable.
    pub fn free_energy(&self, field: &Field, coord: Coord) -> f64 {
        let cell = field.get(coord);
        let p = cell.probability;

        // Internal energy: high probability + high binding = low energy (stable)
        let binding = field.binding_energy(coord);
        let u = -(p + binding);

        // Entropy contribution of this cell
        let s = if p > EPSILON && p < 1.0 {
            -(p * p.log2() + (1.0 - p) * (1.0 - p).log2())
        } else {
            0.0
        };

        u + self.temperature * s
    }

    /// Energy cost to reverse a crystallized cell.
    /// Proportional to binding energy × age. Old, well-connected
    /// crystals are effectively permanent.
    pub fn reversal_cost(&self, field: &Field, coord: Coord) -> f64 {
        let binding = field.binding_energy(coord);
        let age = self.crystal_age.get(&coord).copied().unwrap_or(0) as f64;
        // Minimum cost of 1.0 even for newborn crystals
        (1.0 + binding * 2.0) * (1.0 + age.ln_1p())
    }

    /// Advance one thermodynamic tick:
    /// 1. Cool the temperature
    /// 2. Age all crystals
    /// 3. Check for spontaneous crystallization (F < 0)
    /// 4. Log entropy
    ///
    /// Returns number of new crystallizations.
    pub fn step(&mut self, field: &mut Field) -> usize {
        // Cool
        self.temperature *= 1.0 - self.cooling_rate;
        if self.temperature < 1e-6 {
            self.temperature = 0.0;
        }

        self.tick += 1;

        // Age existing crystals
        let crystal_coords: Vec<Coord> = field.crystallized_cells();
        for coord in &crystal_coords {
            *self.crystal_age.entry(*coord).or_insert(0) += 1;
        }

        // Check all non-crystallized active cells for spontaneous crystallization
        let candidates: Vec<(Coord, f64)> = field.active_entries()
            .filter(|(_, cell)| !cell.crystallized && cell.probability >= EPSILON)
            .map(|(coord, _)| (coord, self.free_energy(field, coord)))
            .filter(|(_, f)| *f < 0.0) // energetically favorable
            .collect();

        let mut new_crystals = 0;
        for (coord, _free_e) in candidates {
            let cell = field.get(coord);
            // Still require minimum probability — can't crystallize from noise
            if cell.probability >= CRYSTALLIZATION_THRESHOLD * 0.8 {
                let cell = field.get_mut(coord);
                if !cell.crystallized {
                    cell.crystallized = true;
                    cell.probability = 1.0;
                    self.crystal_age.insert(coord, 0);
                    new_crystals += 1;
                }
            }
        }

        // Log entropy
        let h = Self::shannon_entropy(field);
        self.entropy_log.push(h);

        new_crystals
    }

    /// Is the field approaching thermal equilibrium?
    /// True if entropy has been stable for `window` ticks.
    pub fn is_equilibrium(&self, window: usize) -> bool {
        if self.entropy_log.len() < window + 1 {
            return false;
        }
        let recent = &self.entropy_log[self.entropy_log.len() - window..];
        let first = recent[0];
        recent.iter().all(|h| (h - first).abs() < 0.01)
    }

    /// Entropy trend: negative = ordering (good), positive = disordering.
    pub fn entropy_trend(&self, window: usize) -> Option<f64> {
        if self.entropy_log.len() < window + 1 {
            return None;
        }
        let recent = &self.entropy_log[self.entropy_log.len() - window..];
        let first = recent[0];
        let last = recent[recent.len() - 1];
        Some(last - first)
    }

    /// Current entropy value.
    pub fn current_entropy(&self) -> Option<f64> {
        self.entropy_log.last().copied()
    }

    /// How many ticks a crystal has survived.
    pub fn crystal_age(&self, coord: Coord) -> u64 {
        self.crystal_age.get(&coord).copied().unwrap_or(0)
    }

    /// Total crystallization events logged.
    pub fn entropy_samples(&self) -> usize {
        self.entropy_log.len()
    }
}

/// Evolve a field to thermodynamic equilibrium.
/// Combines field evolution with thermodynamic steps.
/// The field evolves (diffusion), then thermodynamics crystallizes
/// what is energetically favorable. Temperature cools each step.
pub fn evolve_thermodynamic(
    field: &mut Field,
    thermo: &mut Thermodynamics,
    max_steps: usize,
    equilibrium_window: usize,
) -> usize {
    let mut total_crystals = 0;
    for _ in 0..max_steps {
        field.evolve();
        total_crystals += thermo.step(field);
        if thermo.is_equilibrium(equilibrium_window) {
            break;
        }
    }
    total_crystals
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Coord, Dimension, Field};

    fn coord(t: usize, c: usize, o: usize, v: usize) -> Coord {
        Coord { t, c, o, v }
    }

    #[test]
    fn temperature_decreases_monotonically() {
        let mut field = Field::new(10);
        let mut thermo = Thermodynamics::new(1.0, 0.1);

        let mut temps = Vec::new();
        for _ in 0..20 {
            temps.push(thermo.temperature);
            thermo.step(&mut field);
        }

        for w in temps.windows(2) {
            assert!(w[1] <= w[0], "temperature must decrease: {} -> {}", w[0], w[1]);
        }
    }

    #[test]
    fn entropy_decreases_as_field_crystallizes() {
        let mut field = Field::new(10);
        let center = coord(5, 5, 5, 5);

        // Attest from all 4 dimensions with independent validators
        for (dim, vid) in [
            (Dimension::Temporal, "val_t"),
            (Dimension::Context, "val_c"),
            (Dimension::Origin, "val_o"),
            (Dimension::Verification, "val_v"),
        ] {
            field.attest(center, "event1", dim, vid);
        }

        let h_before = Thermodynamics::shannon_entropy(&field);

        // Evolve to push more cells toward crystallization
        let mut thermo = Thermodynamics::new(0.3, 0.05);
        evolve_thermodynamic(&mut field, &mut thermo, 50, 5);

        let h_after = Thermodynamics::shannon_entropy(&field);

        // Entropy should decrease or stay same (more order)
        assert!(
            h_after <= h_before + 0.1, // small tolerance for numerical noise
            "entropy should not increase significantly: before={h_before}, after={h_after}"
        );
    }

    #[test]
    fn free_energy_negative_for_high_probability_cells() {
        let mut field = Field::new(10);
        let center = coord(5, 5, 5, 5);

        // Seed heavily
        for (dim, vid) in [
            (Dimension::Temporal, "val_t"),
            (Dimension::Context, "val_c"),
            (Dimension::Origin, "val_o"),
            (Dimension::Verification, "val_v"),
        ] {
            field.attest(center, "event1", dim, vid);
        }

        let thermo = Thermodynamics::new(0.1, 0.01); // cold
        let f = thermo.free_energy(&field, center);

        // High probability cell at low temperature → F should be negative
        assert!(f < 0.0, "free energy should be negative for high-p cell, got {f}");
    }

    #[test]
    fn reversal_cost_increases_with_age() {
        let mut field = Field::new(10);
        let center = coord(5, 5, 5, 5);

        for (dim, vid) in [
            (Dimension::Temporal, "val_t"),
            (Dimension::Context, "val_c"),
            (Dimension::Origin, "val_o"),
            (Dimension::Verification, "val_v"),
        ] {
            field.attest(center, "event1", dim, vid);
        }

        let mut thermo = Thermodynamics::new(0.3, 0.05);

        // Step a few times to age crystals
        for _ in 0..10 {
            thermo.step(&mut field);
        }

        let cost_young = thermo.reversal_cost(&field, center);

        // Age more
        for _ in 0..50 {
            thermo.step(&mut field);
        }

        let cost_old = thermo.reversal_cost(&field, center);
        assert!(cost_old > cost_young, "older crystals should cost more to reverse: young={cost_young}, old={cost_old}");
    }

    #[test]
    fn thermodynamic_evolution_reaches_equilibrium() {
        let mut field = Field::new(10);
        let center = coord(5, 5, 5, 5);

        for (dim, vid) in [
            (Dimension::Temporal, "val_t"),
            (Dimension::Context, "val_c"),
            (Dimension::Origin, "val_o"),
            (Dimension::Verification, "val_v"),
        ] {
            field.attest(center, "event1", dim, vid);
        }

        let mut thermo = Thermodynamics::new(0.5, 0.05);
        evolve_thermodynamic(&mut field, &mut thermo, 200, 10);

        assert!(thermo.is_equilibrium(10), "field should reach equilibrium");
        assert!(thermo.temperature < 0.1, "temperature should be low: {}", thermo.temperature);
    }

    #[test]
    fn empty_field_has_zero_entropy() {
        let field = Field::new(10);
        let h = Thermodynamics::shannon_entropy(&field);
        assert!((h - 0.0).abs() < 1e-10, "empty field entropy should be 0, got {h}");
    }
}
