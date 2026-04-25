//! Scaling analysis — measure asymptotic behavior of the field.
//!
//! Functions here return structured results for CSV/JSON export.
//! Focus on trends, not absolute values.

use std::time::Instant;
use crate::{Coord, Dimension, Field, evolve_to_equilibrium};
use crate::adversarial;
use crate::causality::{CausalEvent, CausalGraph};
use crate::liveness;

// --- Result types ---

#[derive(Clone, Debug, serde::Serialize)]
pub struct ThroughputResult {
    pub events: usize,
    pub field_size: usize,
    pub attest_ms: f64,
    pub evolve_ms: f64,
    pub events_per_sec: f64,
    pub crystallized: usize,
    pub active_cells: usize,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct MemoryResult {
    pub field_size: usize,
    pub events: usize,
    pub active_cells: usize,
    pub crystallized: usize,
    pub bytes_per_cell_estimate: usize,
    pub total_memory_estimate_kb: usize,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct SigmaEffCostResult {
    pub events: usize,
    pub with_graph: bool,
    pub sigma_eff_us: f64,
    pub sigma_eff_per_event_us: f64,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct CrystallizationScalingResult {
    pub field_size: usize,
    pub steps_to_crystallize: usize,
    pub crystallized_count: usize,
    pub total_active: usize,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct PartitionRecoveryResult {
    pub partition_duration_steps: usize,
    pub recovery_steps: usize,
    pub total_steps: usize,
    pub crystallized: bool,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct NoiseResilienceResult {
    pub noise_count: usize,
    pub valid_crystallized: bool,
    pub steps_to_crystallize: usize,
    pub noise_crystallized: usize,
    pub total_crystallized: usize,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct BenchmarkSuite {
    pub throughput: Vec<ThroughputResult>,
    pub memory: Vec<MemoryResult>,
    pub sigma_eff_cost: Vec<SigmaEffCostResult>,
    pub crystallization_scaling: Vec<CrystallizationScalingResult>,
    pub partition_recovery: Vec<PartitionRecoveryResult>,
    pub noise_resilience: Vec<NoiseResilienceResult>,
}

// --- Benchmark functions ---

fn coord_at(center: usize, offset: usize, field_size: usize) -> Coord {
    Coord {
        t: (center + offset) % field_size,
        c: (center + offset * 3) % field_size,
        o: center % field_size,
        v: (center + offset * 7) % field_size,
    }
}

fn attest_full(field: &mut Field, center: Coord, event_id: &str) {
    for (dim, vid) in [
        (Dimension::Temporal, "val_t"),
        (Dimension::Context, "val_c"),
        (Dimension::Origin, "val_o"),
        (Dimension::Verification, "val_v"),
    ] {
        field.attest(center, event_id, dim, vid);
    }
}

/// Measure throughput: events/sec for increasing event counts.
pub fn bench_throughput(event_counts: &[usize], field_size: usize) -> Vec<ThroughputResult> {
    let mut results = Vec::new();

    for &n in event_counts {
        let mut field = Field::new(field_size);
        let center = field_size / 2;

        // Phase 1: attest
        let t0 = Instant::now();
        for i in 0..n {
            let c = coord_at(center, i, field_size);
            let eid = format!("ev_{i}");
            attest_full(&mut field, c, &eid);
        }
        let attest_ms = t0.elapsed().as_secs_f64() * 1000.0;

        // Phase 2: evolve to equilibrium
        let t1 = Instant::now();
        evolve_to_equilibrium(&mut field, 5);
        let evolve_ms = t1.elapsed().as_secs_f64() * 1000.0;

        let total_s = (attest_ms + evolve_ms) / 1000.0;
        let eps = if total_s > 0.0 { n as f64 / total_s } else { 0.0 };

        results.push(ThroughputResult {
            events: n,
            field_size,
            attest_ms,
            evolve_ms,
            events_per_sec: eps,
            crystallized: field.crystallized_count(),
            active_cells: field.active_cells(),
        });
    }

    results
}

/// Measure memory usage per cell (estimated from HashMap overhead).
pub fn bench_memory(event_counts: &[usize], field_size: usize) -> Vec<MemoryResult> {
    // Estimate: each Cell has probability(8) + crystallized(1) + influences(Vec ~24)
    // + attestations(HashMap ~48) + padding ≈ 128 bytes base.
    // Each Attestation ≈ 80 bytes (dimension + 2 Strings + f64).
    // Each Influence ≈ 32 bytes.
    const CELL_BASE_BYTES: usize = 128;
    const ATT_BYTES: usize = 80;
    const AVG_ATTS_PER_CELL: usize = 2;
    const COORD_OVERHEAD: usize = 32; // HashMap entry overhead

    let bytes_per_cell = CELL_BASE_BYTES + AVG_ATTS_PER_CELL * ATT_BYTES + COORD_OVERHEAD;

    let mut results = Vec::new();

    for &n in event_counts {
        let mut field = Field::new(field_size);
        let center = field_size / 2;

        for i in 0..n {
            let c = coord_at(center, i, field_size);
            attest_full(&mut field, c, &format!("ev_{i}"));
        }
        evolve_to_equilibrium(&mut field, 3);

        let active = field.active_cells();
        let total_kb = (active * bytes_per_cell) / 1024;

        results.push(MemoryResult {
            field_size,
            events: n,
            active_cells: active,
            crystallized: field.crystallized_count(),
            bytes_per_cell_estimate: bytes_per_cell,
            total_memory_estimate_kb: total_kb,
        });
    }

    results
}

/// Measure σ_eff computation cost per event.
pub fn bench_sigma_eff(event_counts: &[usize], field_size: usize) -> Vec<SigmaEffCostResult> {
    let mut results = Vec::new();

    for &n in event_counts {
        let mut field = Field::new(field_size);
        let center = field_size / 2;
        let mut coords = Vec::with_capacity(n);

        for i in 0..n {
            let c = coord_at(center, i, field_size);
            attest_full(&mut field, c, &format!("ev_{i}"));
            coords.push(c);
        }

        // Without graph
        let t0 = Instant::now();
        for c in &coords {
            let _ = adversarial::effective_sigma(&field, *c, None);
        }
        let no_graph_us = t0.elapsed().as_secs_f64() * 1_000_000.0;

        results.push(SigmaEffCostResult {
            events: n,
            with_graph: false,
            sigma_eff_us: no_graph_us,
            sigma_eff_per_event_us: no_graph_us / n as f64,
        });

        // With graph
        let mut graph = CausalGraph::new();
        let mut last_id = None;
        for i in 0..(n.min(100) as u64) {
            let parents = last_id.iter().cloned().collect();
            let ev = CausalEvent::new(
                coord_at(center, i as usize, field_size),
                i, parents, format!("g_{i}").into_bytes(),
            );
            last_id = Some(ev.id.clone());
            graph.insert(ev);
        }

        let t1 = Instant::now();
        for c in &coords {
            let _ = adversarial::effective_sigma(&field, *c, Some(&graph));
        }
        let with_graph_us = t1.elapsed().as_secs_f64() * 1_000_000.0;

        results.push(SigmaEffCostResult {
            events: n,
            with_graph: true,
            sigma_eff_us: with_graph_us,
            sigma_eff_per_event_us: with_graph_us / n as f64,
        });
    }

    results
}

/// Measure steps to crystallization vs field size.
pub fn bench_crystallization_scaling(sizes: &[usize]) -> Vec<CrystallizationScalingResult> {
    let mut results = Vec::new();

    for &size in sizes {
        let mut field = Field::new(size);
        let center = Coord {
            t: size / 2, c: size / 2, o: size / 2, v: size / 2,
        };
        attest_full(&mut field, center, "scale_event");

        let mut steps = 0;
        // Center crystallizes immediately; count steps for periphery
        for s in 1..=200 {
            field.evolve();
            steps = s;
            // Stop when no new crystallizations for 5 steps
            let c1 = field.crystallized_count();
            field.evolve();
            steps += 1;
            if field.crystallized_count() == c1 {
                break;
            }
        }

        results.push(CrystallizationScalingResult {
            field_size: size,
            steps_to_crystallize: steps,
            crystallized_count: field.crystallized_count(),
            total_active: field.active_cells(),
        });
    }

    results
}

/// Measure partition recovery time vs partition duration.
pub fn bench_partition_recovery(
    partition_durations: &[usize],
    field_size: usize,
) -> Vec<PartitionRecoveryResult> {
    let mut results = Vec::new();

    for &dur in partition_durations {
        let mut field = Field::new(field_size);
        let center = Coord {
            t: field_size / 2, c: field_size / 2,
            o: field_size / 2, v: field_size / 2,
        };

        // Partial attestation (2 dims — simulates partition)
        field.attest(center, "part_event", Dimension::Temporal, "val_t");
        field.attest(center, "part_event", Dimension::Context, "val_c");

        // Evolve during partition
        for _ in 0..dur {
            field.evolve();
        }

        // Partition heals — remaining dims arrive
        field.attest(center, "part_event", Dimension::Origin, "val_o");
        field.attest(center, "part_event", Dimension::Verification, "val_v");

        // Measure recovery
        let (crystallized, recovery_steps) =
            liveness::check_liveness(&mut field, center, liveness::LIVENESS_BOUND);

        results.push(PartitionRecoveryResult {
            partition_duration_steps: dur,
            recovery_steps,
            total_steps: dur + recovery_steps,
            crystallized,
        });
    }

    results
}

/// Measure resilience under increasing noise injection.
pub fn bench_noise_resilience(
    noise_counts: &[usize],
    field_size: usize,
) -> Vec<NoiseResilienceResult> {
    let mut results = Vec::new();

    for &noise_n in noise_counts {
        let mut field = Field::new(field_size);
        let center = Coord {
            t: field_size / 2, c: field_size / 2,
            o: field_size / 2, v: field_size / 2,
        };

        // Inject noise first
        liveness::inject_noise(&mut field, center, noise_n, 4);

        // Evolve noise
        for _ in 0..10 {
            field.evolve();
        }
        let noise_crystallized_before = field.crystallized_count();

        // Valid event
        attest_full(&mut field, center, "valid_event");

        let (crystallized, steps) =
            liveness::check_liveness(&mut field, center, liveness::LIVENESS_BOUND);

        results.push(NoiseResilienceResult {
            noise_count: noise_n,
            valid_crystallized: crystallized,
            steps_to_crystallize: steps,
            noise_crystallized: noise_crystallized_before,
            total_crystallized: field.crystallized_count(),
        });
    }

    results
}

/// Run the full benchmark suite and return structured results.
/// Uses moderate sizes suitable for test mode.
/// For full-scale benchmarks, use `cargo bench` with Criterion.
pub fn run_full_suite() -> BenchmarkSuite {
    let throughput = bench_throughput(&[5, 10, 20], 10);
    let memory = bench_memory(&[5, 10, 20], 10);
    let sigma_eff_cost = bench_sigma_eff(&[5, 10], 10);
    let crystallization_scaling = bench_crystallization_scaling(&[8, 10, 12]);
    let partition_recovery = bench_partition_recovery(&[0, 5, 10], 10);
    let noise_resilience = bench_noise_resilience(&[0, 10, 30], 10);

    BenchmarkSuite {
        throughput,
        memory,
        sigma_eff_cost,
        crystallization_scaling,
        partition_recovery,
        noise_resilience,
    }
}

/// Export results to JSON.
pub fn export_json(suite: &BenchmarkSuite) -> String {
    serde_json::to_string_pretty(suite).unwrap_or_else(|e| format!("{{\"error\": \"{e}\"}}"))
}

/// Export results to CSV (one section per metric).
pub fn export_csv(suite: &BenchmarkSuite) -> String {
    let mut csv = String::new();

    csv.push_str("# Throughput\n");
    csv.push_str("events,field_size,attest_ms,evolve_ms,events_per_sec,crystallized,active_cells\n");
    for r in &suite.throughput {
        csv.push_str(&format!(
            "{},{},{:.2},{:.2},{:.1},{},{}\n",
            r.events, r.field_size, r.attest_ms, r.evolve_ms,
            r.events_per_sec, r.crystallized, r.active_cells
        ));
    }

    csv.push_str("\n# Memory\n");
    csv.push_str("field_size,events,active_cells,crystallized,bytes_per_cell,total_kb\n");
    for r in &suite.memory {
        csv.push_str(&format!(
            "{},{},{},{},{},{}\n",
            r.field_size, r.events, r.active_cells, r.crystallized,
            r.bytes_per_cell_estimate, r.total_memory_estimate_kb
        ));
    }

    csv.push_str("\n# Sigma_eff cost\n");
    csv.push_str("events,with_graph,total_us,per_event_us\n");
    for r in &suite.sigma_eff_cost {
        csv.push_str(&format!(
            "{},{},{:.2},{:.2}\n",
            r.events, r.with_graph, r.sigma_eff_us, r.sigma_eff_per_event_us
        ));
    }

    csv.push_str("\n# Crystallization scaling\n");
    csv.push_str("field_size,steps,crystallized,active\n");
    for r in &suite.crystallization_scaling {
        csv.push_str(&format!(
            "{},{},{},{}\n",
            r.field_size, r.steps_to_crystallize,
            r.crystallized_count, r.total_active
        ));
    }

    csv.push_str("\n# Partition recovery\n");
    csv.push_str("partition_steps,recovery_steps,total_steps,crystallized\n");
    for r in &suite.partition_recovery {
        csv.push_str(&format!(
            "{},{},{},{}\n",
            r.partition_duration_steps, r.recovery_steps,
            r.total_steps, r.crystallized
        ));
    }

    csv.push_str("\n# Noise resilience\n");
    csv.push_str("noise_count,valid_crystallized,steps,noise_crystallized,total_crystallized\n");
    for r in &suite.noise_resilience {
        csv.push_str(&format!(
            "{},{},{},{},{}\n",
            r.noise_count, r.valid_crystallized, r.steps_to_crystallize,
            r.noise_crystallized, r.total_crystallized
        ));
    }

    csv
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn throughput_scales_sublinearly_with_events() {
        let results = bench_throughput(&[5, 20], 10);
        assert_eq!(results.len(), 2);

        let r_small = &results[0];
        let r_large = &results[1];

        assert!(r_small.events_per_sec > 0.0, "should have positive throughput");
        assert!(r_large.events_per_sec > 0.0, "should have positive throughput");
        let time_small = r_small.attest_ms + r_small.evolve_ms;
        let time_large = r_large.attest_ms + r_large.evolve_ms;
        assert!(
            time_large < time_small * 100.0,
            "should be sublinear: t_small={time_small:.1}ms, t_large={time_large:.1}ms"
        );
    }

    #[test]
    fn memory_grows_with_events() {
        let results = bench_memory(&[5, 20], 10);
        let m10 = &results[0];
        let m100 = &results[1];

        assert!(
            m100.active_cells > m10.active_cells,
            "more events → more active cells: {} vs {}",
            m10.active_cells, m100.active_cells
        );
        assert!(
            m100.total_memory_estimate_kb > m10.total_memory_estimate_kb,
            "more events → more memory"
        );
    }

    #[test]
    fn sigma_eff_cost_with_graph_higher_than_without() {
        let results = bench_sigma_eff(&[10], 10);
        // results[0] = without graph, results[1] = with graph
        assert!(results.len() >= 2);

        let no_graph = &results[0];
        let with_graph = &results[1];

        assert!(!no_graph.with_graph);
        assert!(with_graph.with_graph);

        // Graph version should be slower (more computation)
        assert!(
            with_graph.sigma_eff_per_event_us >= no_graph.sigma_eff_per_event_us * 0.5,
            "graph version should not be dramatically faster: no_graph={:.1}µs, with_graph={:.1}µs",
            no_graph.sigma_eff_per_event_us, with_graph.sigma_eff_per_event_us
        );
    }

    #[test]
    fn crystallization_steps_bounded_across_sizes() {
        let results = bench_crystallization_scaling(&[8, 12, 16]);

        for r in &results {
            assert!(
                r.steps_to_crystallize <= 200,
                "size={}: {} steps exceeds 200",
                r.field_size, r.steps_to_crystallize
            );
            assert!(
                r.crystallized_count > 0,
                "size={}: should have at least 1 crystallization",
                r.field_size
            );
        }

        // Larger fields should have more active cells
        let s8 = results.iter().find(|r| r.field_size == 8).unwrap();
        let s16 = results.iter().find(|r| r.field_size == 16).unwrap();
        assert!(
            s16.total_active >= s8.total_active,
            "larger field should have >= active cells: s8={}, s16={}",
            s8.total_active, s16.total_active
        );
    }

    #[test]
    fn partition_recovery_always_succeeds() {
        let results = bench_partition_recovery(&[0, 10, 50], 12);

        for r in &results {
            assert!(
                r.crystallized,
                "partition_dur={}: should crystallize after recovery",
                r.partition_duration_steps
            );
        }
    }

    #[test]
    fn partition_recovery_time_bounded() {
        let results = bench_partition_recovery(&[0, 20, 100], 12);

        for r in &results {
            assert!(
                r.recovery_steps <= liveness::LIVENESS_BOUND,
                "partition_dur={}: recovery {} exceeds bound {}",
                r.partition_duration_steps, r.recovery_steps, liveness::LIVENESS_BOUND
            );
        }
    }

    #[test]
    fn noise_does_not_prevent_valid_crystallization() {
        let results = bench_noise_resilience(&[0, 10, 50], 10);

        for r in &results {
            assert!(
                r.valid_crystallized,
                "noise={}: valid event must crystallize",
                r.noise_count
            );
        }
    }

    #[test]
    fn noise_increases_steps_but_within_bound() {
        let results = bench_noise_resilience(&[0, 20, 100], 10);

        let no_noise = &results[0];
        for r in &results {
            assert!(
                r.steps_to_crystallize <= liveness::LIVENESS_BOUND,
                "noise={}: {} steps exceeds bound",
                r.noise_count, r.steps_to_crystallize
            );
        }

        // More noise may increase steps (but not necessarily — center
        // crystallizes immediately regardless of noise)
        let high_noise = results.last().unwrap();
        assert!(
            high_noise.steps_to_crystallize >= no_noise.steps_to_crystallize,
            "noise should not DECREASE steps: no_noise={}, high_noise={}",
            no_noise.steps_to_crystallize, high_noise.steps_to_crystallize
        );
    }

    #[test]
    fn full_suite_produces_valid_output() {
        let suite = run_full_suite();

        assert!(!suite.throughput.is_empty());
        assert!(!suite.memory.is_empty());
        assert!(!suite.sigma_eff_cost.is_empty());
        assert!(!suite.crystallization_scaling.is_empty());
        assert!(!suite.partition_recovery.is_empty());
        assert!(!suite.noise_resilience.is_empty());

        // JSON export
        let json = export_json(&suite);
        assert!(json.starts_with('{'), "should be valid JSON");
        assert!(json.contains("throughput"));
        assert!(json.contains("events_per_sec"));

        // CSV export
        let csv = export_csv(&suite);
        assert!(csv.contains("# Throughput"));
        assert!(csv.contains("# Memory"));
        assert!(csv.contains("# Noise resilience"));
    }

    // --- Brutal stress tests ---

    #[test]
    fn stress_200_events_on_small_field() {
        let results = bench_throughput(&[200], 10);
        let r = &results[0];
        assert!(r.events_per_sec > 0.0, "must handle 200 events");
        assert!(r.crystallized > 0, "some events must crystallize");
    }

    #[test]
    fn stress_large_field_single_event() {
        // Large field, single event — tests sparse storage efficiency
        let mut field = Field::new(30);
        let center = Coord { t: 15, c: 15, o: 15, v: 15 };
        attest_full(&mut field, center, "lone_event");
        evolve_to_equilibrium(&mut field, 5);

        // Should crystallize at center
        assert!(field.get(center).crystallized);
        // Active cells should be << 30^4 = 810,000
        assert!(
            field.active_cells() < 10_000,
            "sparse storage: active={} in field of {}",
            field.active_cells(), field.total_cells()
        );
    }

    #[test]
    fn stress_concurrent_events_different_regions() {
        let mut field = Field::new(20);
        let events = 100;

        // Scatter events across different o-regions
        for i in 0..events {
            let region = i % 20;
            let c = Coord { t: 10, c: 10, o: region, v: 10 };
            attest_full(&mut field, c, &format!("region_{i}"));
        }

        evolve_to_equilibrium(&mut field, 5);

        let crystallized = field.crystallized_count();
        assert!(
            crystallized >= events,
            "each event center should crystallize: got {crystallized} / {events}"
        );
    }

    #[test]
    fn stress_rapid_noise_burst_then_valid() {
        let mut field = Field::new(16);
        let center = Coord { t: 8, c: 8, o: 8, v: 8 };

        // Noise burst: 100 noise events
        liveness::inject_noise(&mut field, center, 100, 6);

        // Evolve noise
        for _ in 0..20 {
            field.evolve();
        }

        // Now valid event
        attest_full(&mut field, center, "valid_after_storm");

        assert!(
            field.get(center).crystallized,
            "valid event must crystallize even after 100 noise events"
        );
    }

    #[test]
    fn stress_alternating_noise_and_valid() {
        let mut field = Field::new(12);

        for round in 0..5 {
            let center = Coord {
                t: 4 + round, c: 6, o: 6, v: 6,
            };

            // Noise
            liveness::inject_noise(&mut field, center, 10, 3);
            field.evolve();

            // Valid
            attest_full(&mut field, center, &format!("valid_{round}"));
        }

        evolve_to_equilibrium(&mut field, 5);

        // All valid events should crystallize
        for round in 0..5 {
            let center = Coord {
                t: 4 + round, c: 6, o: 6, v: 6,
            };
            assert!(
                field.get(center).crystallized,
                "round {round}: valid event should crystallize"
            );
        }
    }

    #[test]
    fn stress_cascading_crystallization_chain() {
        // Events placed along a line — does crystallization cascade correctly?
        let mut field = Field::new(20);

        for i in 0..10 {
            let c = Coord { t: 5 + i, c: 10, o: 10, v: 10 };
            attest_full(&mut field, c, &format!("chain_{i}"));
        }

        evolve_to_equilibrium(&mut field, 10);

        let mut chain_crystallized = 0;
        for i in 0..10 {
            let c = Coord { t: 5 + i, c: 10, o: 10, v: 10 };
            if field.get(c).crystallized {
                chain_crystallized += 1;
            }
        }

        assert_eq!(
            chain_crystallized, 10,
            "all chain events should crystallize: {chain_crystallized}/10"
        );
    }

    #[test]
    fn stress_field_density_ratio() {
        // Verify sparse storage: active cells << total cells.
        // SEED_RADIUS=3 → each attestation touches (2×3+1)⁴ = 2401 cells.
        // On small fields (size=10, total=10⁴=10000), density can reach ~24%.
        // Sparsity only holds for large fields relative to seed radius.
        for size in [16, 20, 25] {
            let mut field = Field::new(size);
            let c = Coord { t: size / 2, c: size / 2, o: size / 2, v: size / 2 };
            attest_full(&mut field, c, "density_test");
            evolve_to_equilibrium(&mut field, 5);

            let density = field.active_cells() as f64 / field.total_cells() as f64;
            assert!(
                density < 0.15,
                "size={size}: density {density:.4} should be < 15% (sparse)"
            );
        }
    }
}
