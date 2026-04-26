use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use tesseract::adversarial;
use tesseract::causality::{CausalEvent, CausalGraph};
use tesseract::liveness;
use tesseract::{evolve_to_equilibrium, Coord, Dimension, Field};

fn coord_center(size: usize) -> Coord {
    Coord {
        t: size / 2,
        c: size / 2,
        o: size / 2,
        v: size / 2,
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

fn bench_attest_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("attest_throughput");
    for n in [10, 50, 100, 500] {
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            b.iter(|| {
                let mut field = Field::new(16);
                let center = coord_center(16);
                for i in 0..n {
                    let coord = Coord {
                        t: (center.t + i) % 16,
                        c: (center.c + i * 3) % 16,
                        o: center.o,
                        v: (center.v + i * 7) % 16,
                    };
                    attest_full(&mut field, coord, &format!("ev_{i}"));
                }
                black_box(field.crystallized_count())
            });
        });
    }
    group.finish();
}

fn bench_evolve(c: &mut Criterion) {
    let mut group = c.benchmark_group("evolve_step");
    for size in [8, 12, 16, 20] {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            let mut field = Field::new(size);
            let center = coord_center(size);
            attest_full(&mut field, center, "setup");

            b.iter(|| {
                black_box(field.evolve());
            });
        });
    }
    group.finish();
}

fn bench_sigma_independence(c: &mut Criterion) {
    let mut group = c.benchmark_group("sigma_independence");
    for n_atts in [1, 2, 4] {
        group.bench_with_input(BenchmarkId::from_parameter(n_atts), &n_atts, |b, &n| {
            let mut field = Field::new(12);
            let center = coord_center(12);
            let dims = [
                (Dimension::Temporal, "val_t"),
                (Dimension::Context, "val_c"),
                (Dimension::Origin, "val_o"),
                (Dimension::Verification, "val_v"),
            ];
            for i in 0..n {
                field.attest(center, "event", dims[i].0, dims[i].1);
            }

            b.iter(|| black_box(field.get(center).sigma_independence()));
        });
    }
    group.finish();
}

fn bench_sigma_eff(c: &mut Criterion) {
    let mut group = c.benchmark_group("sigma_eff");

    // Without graph
    group.bench_function("no_graph", |b| {
        let mut field = Field::new(12);
        let center = coord_center(12);
        attest_full(&mut field, center, "event");

        b.iter(|| black_box(adversarial::effective_sigma(&field, center, None)));
    });

    // With graph
    group.bench_function("with_graph", |b| {
        let mut field = Field::new(12);
        let center = coord_center(12);
        attest_full(&mut field, center, "event");

        let mut graph = CausalGraph::new();
        let mut last_id = None;
        for i in 0..10u64 {
            let parents = last_id.iter().cloned().collect();
            let ev = CausalEvent::new(center, i, parents, format!("g_{i}").into_bytes());
            last_id = Some(ev.id.clone());
            graph.insert(ev);
        }

        b.iter(|| black_box(adversarial::effective_sigma(&field, center, Some(&graph))));
    });

    group.finish();
}

fn bench_crystallization_vs_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("crystallization_full");
    group.sample_size(10); // expensive

    for size in [8, 12, 16] {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter(|| {
                let mut field = Field::new(size);
                let center = coord_center(size);
                attest_full(&mut field, center, "bench");
                evolve_to_equilibrium(&mut field, 5);
                black_box(field.crystallized_count())
            });
        });
    }
    group.finish();
}

fn bench_noise_resilience(c: &mut Criterion) {
    let mut group = c.benchmark_group("noise_resilience");
    group.sample_size(10);

    for noise in [0, 50, 200] {
        group.bench_with_input(BenchmarkId::from_parameter(noise), &noise, |b, &noise| {
            b.iter(|| {
                let mut field = Field::new(14);
                let center = coord_center(14);
                liveness::inject_noise(&mut field, center, noise, 4);
                attest_full(&mut field, center, "valid");
                let (crystallized, steps) =
                    liveness::check_liveness(&mut field, center, liveness::LIVENESS_BOUND);
                black_box((crystallized, steps))
            });
        });
    }
    group.finish();
}

fn bench_partition_recovery(c: &mut Criterion) {
    let mut group = c.benchmark_group("partition_recovery");
    group.sample_size(10);

    for dur in [0, 10, 50] {
        group.bench_with_input(BenchmarkId::from_parameter(dur), &dur, |b, &dur| {
            b.iter(|| {
                let mut field = Field::new(12);
                let center = coord_center(12);

                field.attest(center, "event", Dimension::Temporal, "val_t");
                field.attest(center, "event", Dimension::Context, "val_c");
                for _ in 0..dur {
                    field.evolve();
                }
                field.attest(center, "event", Dimension::Origin, "val_o");
                field.attest(center, "event", Dimension::Verification, "val_v");

                let (ok, steps) =
                    liveness::check_liveness(&mut field, center, liveness::LIVENESS_BOUND);
                black_box((ok, steps))
            });
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_attest_throughput,
    bench_evolve,
    bench_sigma_independence,
    bench_sigma_eff,
    bench_crystallization_vs_size,
    bench_noise_resilience,
    bench_partition_recovery,
);
criterion_main!(benches);
