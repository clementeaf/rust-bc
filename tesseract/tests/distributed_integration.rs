//! Distributed integration tests — 10, 100, 1000 node simulations.
//!
//! Results are empirical observations, not formal guarantees.
//! All tests use small field_size (6) to stay fast in debug mode.
//! For large-scale analysis, use release mode: `cargo test --release`.

use tesseract::Coord;
use tesseract::gossip::{DistributedSim, GossipConfig};
use tesseract::network_sim::NetworkConfig;

fn coord(t: usize, c: usize, o: usize, v: usize) -> Coord {
    Coord { t, c, o, v }
}

#[derive(Clone, Debug, serde::Serialize)]
struct IntegrationResult {
    num_nodes: usize,
    network_profile: String,
    convergence_ticks: u64,
    convergence_ratio: f64,
    false_crystallization_rate: f64,
    messages_sent: u64,
    messages_per_node: f64,
    duplicate_rate: f64,
    drop_rate: f64,
}

fn run_convergence_test(
    num_nodes: usize,
    gossip: GossipConfig,
    net: NetworkConfig,
    profile_name: &str,
    max_ticks: u64,
) -> IntegrationResult {
    let mut sim = DistributedSim::new(num_nodes, gossip, net);
    let center = coord(3, 3, 3, 3);
    sim.originate_full_event(0, center, "integration_event");

    let mut convergence_tick = max_ticks;
    for t in 1..=max_ticks {
        sim.step();
        if sim.crystallization_ratio(center) >= 1.0 {
            convergence_tick = t;
            break;
        }
    }

    let ratio = sim.crystallization_ratio(center);
    let metrics = sim.metrics();
    let empty = coord(1, 1, 1, 1);
    let false_c = sim.false_crystallizations(empty, 0);

    IntegrationResult {
        num_nodes,
        network_profile: profile_name.to_string(),
        convergence_ticks: convergence_tick,
        convergence_ratio: ratio,
        false_crystallization_rate: false_c as f64 / num_nodes as f64,
        messages_sent: metrics.network.messages_sent,
        messages_per_node: metrics.network.messages_sent as f64 / num_nodes as f64,
        duplicate_rate: metrics.network.dup_rate_actual,
        drop_rate: metrics.network.drop_rate_actual,
    }
}

fn run_partition_test(
    num_nodes: usize,
    gossip: GossipConfig,
    net: NetworkConfig,
    partition_nodes: &[usize],
    partition_duration: u64,
    max_ticks_after: u64,
) -> IntegrationResult {
    let mut sim = DistributedSim::new(num_nodes, gossip, net);
    let all: Vec<usize> = (0..num_nodes).collect();
    let center = coord(3, 3, 3, 3);

    for &n in partition_nodes {
        sim.network.isolate(n, &all);
    }

    sim.originate_full_event(0, center, "part_event");
    sim.run(partition_duration);

    for &n in partition_nodes {
        sim.network.reconnect(n, &all);
    }
    sim.originate_full_event(0, center, "part_event");

    let mut convergence_tick = partition_duration + max_ticks_after;
    for t in 1..=max_ticks_after {
        sim.step();
        if sim.crystallization_ratio(center) >= 1.0 {
            convergence_tick = partition_duration + t;
            break;
        }
    }

    let metrics = sim.metrics();
    IntegrationResult {
        num_nodes,
        network_profile: format!("partition_{}", partition_nodes.len()),
        convergence_ticks: convergence_tick,
        convergence_ratio: sim.crystallization_ratio(center),
        false_crystallization_rate: 0.0,
        messages_sent: metrics.network.messages_sent,
        messages_per_node: metrics.network.messages_sent as f64 / num_nodes as f64,
        duplicate_rate: metrics.network.dup_rate_actual,
        drop_rate: metrics.network.drop_rate_actual,
    }
}

// All tests use field_size=6 for speed. Gossip fanout >= ln(N).

// --- 10 nodes ---

#[test]
fn ten_nodes_clean() {
    let r = run_convergence_test(10,
        GossipConfig { fanout: 5, field_size: 6, seed: 1 },
        NetworkConfig::default(), "clean", 60);
    assert!(r.convergence_ratio >= 0.9, "10 clean: {:.0}%", r.convergence_ratio * 100.0);
    assert_eq!(r.false_crystallization_rate, 0.0);
}

#[test]
fn ten_nodes_lossy() {
    let r = run_convergence_test(10,
        GossipConfig { fanout: 5, field_size: 6, seed: 2 },
        NetworkConfig::lossy(), "lossy", 80);
    assert!(r.convergence_ratio >= 0.7, "10 lossy: {:.0}%", r.convergence_ratio * 100.0);
    assert_eq!(r.false_crystallization_rate, 0.0);
}

#[test]
fn ten_nodes_adversarial() {
    let r = run_convergence_test(10,
        GossipConfig { fanout: 6, field_size: 6, seed: 3 },
        NetworkConfig::adversarial(), "adversarial", 100);
    assert!(r.convergence_ratio >= 0.5, "10 adversarial: {:.0}%", r.convergence_ratio * 100.0);
    assert_eq!(r.false_crystallization_rate, 0.0);
}

#[test]
fn ten_nodes_partition() {
    let r = run_partition_test(10,
        GossipConfig { fanout: 5, field_size: 6, seed: 4 },
        NetworkConfig::default(), &[7, 8, 9], 15, 60);
    assert!(r.convergence_ratio >= 0.9, "10 partition: {:.0}%", r.convergence_ratio * 100.0);
}

// --- 100 nodes ---

#[test]
fn hundred_nodes_clean() {
    let r = run_convergence_test(100,
        GossipConfig { fanout: 5, field_size: 6, seed: 10 },
        NetworkConfig::default(), "clean", 60);
    assert!(r.convergence_ratio >= 0.9, "100 clean: {:.0}%", r.convergence_ratio * 100.0);
    assert_eq!(r.false_crystallization_rate, 0.0);
}

#[test]
fn hundred_nodes_lossy() {
    let r = run_convergence_test(100,
        GossipConfig { fanout: 6, field_size: 6, seed: 11 },
        NetworkConfig::lossy(), "lossy", 80);
    assert!(r.convergence_ratio >= 0.7, "100 lossy: {:.0}%", r.convergence_ratio * 100.0);
    assert_eq!(r.false_crystallization_rate, 0.0);
}

#[test]
fn hundred_nodes_partition() {
    let partition: Vec<usize> = (80..100).collect(); // 20%
    let r = run_partition_test(100,
        GossipConfig { fanout: 5, field_size: 6, seed: 12 },
        NetworkConfig::default(), &partition, 15, 60);
    assert!(r.convergence_ratio >= 0.7, "100 partition: {:.0}%", r.convergence_ratio * 100.0);
}

// --- 1000 nodes ---

#[test]
fn thousand_nodes_clean() {
    let r = run_convergence_test(1000,
        GossipConfig { fanout: 7, field_size: 6, seed: 100 },
        NetworkConfig { base_latency: 1, jitter: 1, ..NetworkConfig::default() },
        "clean", 80);
    assert!(r.convergence_ratio >= 0.9, "1000 clean: {:.0}%", r.convergence_ratio * 100.0);
    assert_eq!(r.false_crystallization_rate, 0.0);
    assert!(r.messages_per_node < 200.0, "msg overhead: {:.0}", r.messages_per_node);
}

#[test]
fn thousand_nodes_lossy() {
    let r = run_convergence_test(1000,
        GossipConfig { fanout: 8, field_size: 6, seed: 101 },
        NetworkConfig::lossy(), "lossy", 100);
    assert!(r.convergence_ratio >= 0.7, "1000 lossy: {:.0}%", r.convergence_ratio * 100.0);
    assert_eq!(r.false_crystallization_rate, 0.0);
}

#[test]
fn thousand_nodes_partition() {
    let partition: Vec<usize> = (900..1000).collect();
    let r = run_partition_test(1000,
        GossipConfig { fanout: 7, field_size: 6, seed: 102 },
        NetworkConfig::default(), &partition, 15, 80);
    assert!(r.convergence_ratio >= 0.8, "1000 partition: {:.0}%", r.convergence_ratio * 100.0);
}

// --- Safety invariant across all profiles ---

#[test]
fn zero_false_crystallizations_all_profiles() {
    let mut results = Vec::new();

    for (name, net) in [
        ("clean", NetworkConfig::default()),
        ("lossy", NetworkConfig::lossy()),
    ] {
        results.push(run_convergence_test(10,
            GossipConfig { fanout: 5, field_size: 6, seed: 200 },
            net, name, 60));
    }

    results.push(run_convergence_test(50,
        GossipConfig { fanout: 4, field_size: 6, seed: 201 },
        NetworkConfig::default(), "clean_50", 60));

    // JSON
    let json = serde_json::to_string_pretty(&results).unwrap();
    assert!(json.contains("convergence_ratio"));

    // CSV
    let mut csv = String::from("nodes,profile,ticks,ratio,false_rate,msgs,msgs_per_node,dup,drop\n");
    for r in &results {
        csv.push_str(&format!(
            "{},{},{},{:.3},{:.4},{},{:.1},{:.3},{:.3}\n",
            r.num_nodes, r.network_profile, r.convergence_ticks,
            r.convergence_ratio, r.false_crystallization_rate,
            r.messages_sent, r.messages_per_node, r.duplicate_rate, r.drop_rate,
        ));
    }
    assert!(csv.contains("clean"));

    for r in &results {
        assert_eq!(r.false_crystallization_rate, 0.0,
            "{} nodes / {}: false crystallization!", r.num_nodes, r.network_profile);
    }
}
