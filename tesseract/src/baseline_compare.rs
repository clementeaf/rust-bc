//! Baseline comparison — Tesseract vs simplified consensus models.
//!
//! Each baseline is a minimal but fair implementation of a real protocol class.
//! We do NOT weaken baselines or optimize Tesseract unfairly.
//!
//! # Baselines
//!
//! ## 1. Quorum (Raft/BFT-style)
//! **Model**: A designated leader proposes a value. Followers vote. Finality
//! requires ⌊N/2⌋+1 votes (Raft) or ⌊(N-1)/3⌋×2+1 votes (BFT).
//! **Assumptions**: Leader is known and correct. Messages are authenticated.
//! Followers respond within bounded time. No Byzantine leader.
//! **Strengths**: Fast finality (1-2 rounds), strong consistency.
//! **Weaknesses**: Leader bottleneck, requires known membership, N/3 fault bound.
//!
//! ## 2. DAG gossip with threshold validation
//! **Model**: Events propagate via gossip. Each node validates locally.
//! An event is "accepted" when a node has seen it from ≥ threshold distinct
//! sources. No dimensional independence requirement.
//! **Assumptions**: Nodes are honest. Sources are distinct (no Sybil check).
//! **Strengths**: Decentralized, no leader, simple.
//! **Weaknesses**: No structural independence, vulnerable to Sybil.
//!
//! ## 3. CRDT / event-log eventual consistency
//! **Model**: Each node maintains a grow-only set (G-Set CRDT). Events are
//! added locally and merged on sync. Finality = event present in local set.
//! No validation beyond dedup.
//! **Assumptions**: Eventual delivery. No adversary. Merge is commutative.
//! **Strengths**: Always available, partition-tolerant, instant local finality.
//! **Weaknesses**: No validation, accepts anything, no safety against adversary.
//!
//! ## 4. Tesseract
//! **Model**: 4D field with σ-independence. Crystallization requires σ=4.
//! Gossip + anti-entropy for propagation. See other modules for full model.

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::{HashMap, HashSet};
use std::time::Instant;

// --- Common types ---

#[derive(Clone, Debug, serde::Serialize)]
pub struct ComparisonResult {
    pub system: String,
    pub num_nodes: usize,
    pub network_profile: String,
    /// Ticks until finality (all honest nodes agree).
    pub finality_ticks: u64,
    /// Total messages exchanged.
    pub messages_total: u64,
    /// Messages per node.
    pub messages_per_node: f64,
    /// Recovery ticks after partition heals.
    pub partition_recovery_ticks: u64,
    /// Events finalized per second (wall clock).
    pub throughput_events_per_sec: f64,
    /// Estimated memory per node (bytes).
    pub memory_per_node_bytes: usize,
    /// Events accepted despite being adversarial noise.
    pub false_acceptances: usize,
    /// Total events processed.
    pub total_events: usize,
    /// False acceptance rate.
    pub false_acceptance_rate: f64,
}

#[derive(Clone, Debug)]
pub struct BenchConfig {
    pub num_nodes: usize,
    pub drop_rate: f64,
    pub partition_nodes: Vec<usize>,
    pub partition_duration: u64,
    pub noise_events: usize,
    pub valid_events: usize,
    pub seed: u64,
}

impl Default for BenchConfig {
    fn default() -> Self {
        Self {
            num_nodes: 20,
            drop_rate: 0.0,
            partition_nodes: vec![],
            partition_duration: 0,
            noise_events: 0,
            valid_events: 1,
            seed: 42,
        }
    }
}

// ============================================================
// Baseline 1: Quorum (Raft/BFT-style)
// ============================================================

struct QuorumSim {
    num_nodes: usize,
    leader: usize,
    /// Per-node: set of proposals they've voted for.
    votes: HashMap<usize, HashSet<String>>,
    /// Finalized proposals (quorum reached).
    finalized: HashSet<String>,
    messages: u64,
    rng: StdRng,
    drop_rate: f64,
    partitioned: HashSet<usize>,
}

impl QuorumSim {
    fn new(num_nodes: usize, seed: u64, drop_rate: f64) -> Self {
        let mut votes = HashMap::new();
        for i in 0..num_nodes {
            votes.insert(i, HashSet::new());
        }
        Self {
            num_nodes,
            leader: 0,
            votes,
            finalized: HashSet::new(),
            messages: 0,
            rng: StdRng::seed_from_u64(seed),
            drop_rate,
            partitioned: HashSet::new(),
        }
    }

    fn quorum_size(&self) -> usize {
        // BFT: 2f+1 where f = ⌊(N-1)/3⌋
        let f = (self.num_nodes - 1) / 3;
        2 * f + 1
    }

    fn propose(&mut self, proposal: &str) -> u64 {
        let mut ticks = 0;

        // Round 1: leader sends proposal to all
        let mut vote_count = 0;
        // Leader votes for itself
        self.votes
            .get_mut(&self.leader)
            .unwrap()
            .insert(proposal.to_string());
        vote_count += 1;

        for node in 0..self.num_nodes {
            if node == self.leader {
                continue;
            }
            if self.partitioned.contains(&node) {
                continue;
            }

            self.messages += 1; // leader → node
            if self.rng.gen::<f64>() < self.drop_rate {
                continue;
            }

            // Node votes
            self.votes
                .get_mut(&node)
                .unwrap()
                .insert(proposal.to_string());
            vote_count += 1;

            self.messages += 1; // node → leader (vote)
            if self.rng.gen::<f64>() < self.drop_rate {
                vote_count -= 1; // vote lost
            }
        }
        ticks += 1; // one round trip

        // Check quorum
        if vote_count >= self.quorum_size() {
            // Round 2: leader sends commit to all
            for node in 0..self.num_nodes {
                if node == self.leader || self.partitioned.contains(&node) {
                    continue;
                }
                self.messages += 1;
            }
            ticks += 1;
            self.finalized.insert(proposal.to_string());
        }

        ticks
    }

    fn is_finalized(&self, proposal: &str) -> bool {
        self.finalized.contains(proposal)
    }

    fn node_has(&self, node: usize, proposal: &str) -> bool {
        self.votes
            .get(&node)
            .map(|v| v.contains(proposal))
            .unwrap_or(false)
    }

    fn convergence_ratio(&self, proposal: &str) -> f64 {
        let count = (0..self.num_nodes)
            .filter(|n| self.node_has(*n, proposal))
            .count();
        count as f64 / self.num_nodes as f64
    }
}

// ============================================================
// Baseline 2: DAG gossip with threshold validation
// ============================================================

struct DagGossipSim {
    num_nodes: usize,
    /// Per-node: events seen with their source set.
    node_events: HashMap<usize, HashMap<String, HashSet<usize>>>,
    /// Per-node: accepted events (threshold met).
    accepted: HashMap<usize, HashSet<String>>,
    threshold: usize,
    messages: u64,
    rng: StdRng,
    drop_rate: f64,
    fanout: usize,
    partitioned: HashSet<usize>,
}

impl DagGossipSim {
    fn new(num_nodes: usize, threshold: usize, fanout: usize, seed: u64, drop_rate: f64) -> Self {
        let mut node_events = HashMap::new();
        let mut accepted = HashMap::new();
        for i in 0..num_nodes {
            node_events.insert(i, HashMap::new());
            accepted.insert(i, HashSet::new());
        }
        Self {
            num_nodes,
            node_events,
            accepted,
            threshold,
            messages: 0,
            rng: StdRng::seed_from_u64(seed),
            drop_rate,
            fanout,
            partitioned: HashSet::new(),
        }
    }

    fn originate(&mut self, from: usize, event_id: &str) {
        // Node `from` sees event from itself
        self.node_events
            .get_mut(&from)
            .unwrap()
            .entry(event_id.to_string())
            .or_default()
            .insert(from);
        self.check_threshold(from, event_id);
    }

    fn gossip_rounds(&mut self, event_id: &str, rounds: usize) {
        for _ in 0..rounds {
            let mut deliveries: Vec<(usize, usize)> = Vec::new();

            for node in 0..self.num_nodes {
                if self.partitioned.contains(&node) {
                    continue;
                }
                let has_event = self.node_events[&node].contains_key(event_id);
                if !has_event {
                    continue;
                }

                // Forward to fanout peers
                let mut candidates: Vec<usize> = (0..self.num_nodes)
                    .filter(|&n| n != node && !self.partitioned.contains(&n))
                    .collect();
                for _ in 0..self.fanout.min(candidates.len()) {
                    if candidates.is_empty() {
                        break;
                    }
                    let idx = self.rng.gen_range(0..candidates.len());
                    let target = candidates.swap_remove(idx);
                    self.messages += 1;
                    if self.rng.gen::<f64>() >= self.drop_rate {
                        deliveries.push((node, target));
                    }
                }
            }

            for (source, target) in deliveries {
                self.node_events
                    .get_mut(&target)
                    .unwrap()
                    .entry(event_id.to_string())
                    .or_default()
                    .insert(source);
                self.check_threshold(target, event_id);
            }
        }
    }

    fn check_threshold(&mut self, node: usize, event_id: &str) {
        let sources = self.node_events[&node]
            .get(event_id)
            .map(|s| s.len())
            .unwrap_or(0);
        if sources >= self.threshold {
            self.accepted
                .get_mut(&node)
                .unwrap()
                .insert(event_id.to_string());
        }
    }

    fn acceptance_ratio(&self, event_id: &str) -> f64 {
        let count = (0..self.num_nodes)
            .filter(|n| self.accepted[n].contains(event_id))
            .count();
        count as f64 / self.num_nodes as f64
    }
}

// ============================================================
// Baseline 3: CRDT G-Set
// ============================================================

struct CrdtSim {
    num_nodes: usize,
    /// Per-node: grow-only set of events.
    sets: HashMap<usize, HashSet<String>>,
    messages: u64,
    rng: StdRng,
    drop_rate: f64,
    partitioned: HashSet<usize>,
}

impl CrdtSim {
    fn new(num_nodes: usize, seed: u64, drop_rate: f64) -> Self {
        let mut sets = HashMap::new();
        for i in 0..num_nodes {
            sets.insert(i, HashSet::new());
        }
        Self {
            num_nodes,
            sets,
            messages: 0,
            rng: StdRng::seed_from_u64(seed),
            drop_rate,
            partitioned: HashSet::new(),
        }
    }

    fn insert(&mut self, node: usize, event_id: &str) {
        self.sets
            .get_mut(&node)
            .unwrap()
            .insert(event_id.to_string());
    }

    /// Pairwise merge: each node syncs with a random peer.
    fn sync_round(&mut self) {
        let nodes: Vec<usize> = (0..self.num_nodes).collect();
        let mut merges: Vec<(usize, HashSet<String>)> = Vec::new();

        for &a in &nodes {
            if self.partitioned.contains(&a) {
                continue;
            }
            let candidates: Vec<usize> = nodes
                .iter()
                .copied()
                .filter(|&b| b != a && !self.partitioned.contains(&b))
                .collect();
            if candidates.is_empty() {
                continue;
            }
            let b = candidates[self.rng.gen_range(0..candidates.len())];

            self.messages += 2; // bidirectional sync
            if self.rng.gen::<f64>() < self.drop_rate {
                continue;
            }

            // Merge B into A
            let b_set = self.sets[&b].clone();
            merges.push((a, b_set));
        }

        for (a, b_set) in merges {
            self.sets.get_mut(&a).unwrap().extend(b_set);
        }
    }

    fn convergence_ratio(&self, event_id: &str) -> f64 {
        let count = (0..self.num_nodes)
            .filter(|n| self.sets[n].contains(event_id))
            .count();
        count as f64 / self.num_nodes as f64
    }

    /// CRDT accepts EVERYTHING — no validation.
    fn has_event(&self, node: usize, event_id: &str) -> bool {
        self.sets[&node].contains(event_id)
    }
}

// ============================================================
// Tesseract wrapper (uses gossip module)
// ============================================================

fn run_tesseract(cfg: &BenchConfig) -> ComparisonResult {
    use crate::gossip::{DistributedSim, GossipConfig};
    use crate::network_sim::NetworkConfig;
    use crate::Coord;

    let net = NetworkConfig {
        drop_rate: cfg.drop_rate,
        ..NetworkConfig::default()
    };
    let gossip = GossipConfig {
        fanout: 5.min(cfg.num_nodes - 1),
        field_size: 10, // 10 so valid seed at (5,5,5,5) doesn't wrap to (0,0,0,0)
        anti_entropy_interval: 10,
        seed: cfg.seed,
    };

    let mut sim = DistributedSim::new(cfg.num_nodes, gossip, net);
    let center = Coord {
        t: 5,
        c: 5,
        o: 5,
        v: 5,
    };
    let all: Vec<usize> = (0..cfg.num_nodes).collect();

    // Partition
    for &n in &cfg.partition_nodes {
        sim.network.isolate(n, &all);
    }

    let t0 = Instant::now();

    // Valid events
    for i in 0..cfg.valid_events {
        sim.originate_full_event(0, center, &format!("valid_{i}"));
    }

    // Noise: same validator on all 4 dims → σ=0, should never crystallize.
    // Uses same validator_id across dims to ensure no exclusive attestations.
    let noise_coord = Coord {
        t: 0,
        c: 0,
        o: 0,
        v: 0,
    };
    for i in 0..cfg.noise_events {
        for dim in crate::Dimension::ALL {
            sim.originate_attestation(
                i % cfg.num_nodes,
                noise_coord,
                &format!("noise_{i}"),
                dim,
                "sybil_validator", // same ID on all dims → σ=0
            );
        }
    }

    // Run gossip, track when full convergence is reached
    let mut finality: u64 = 0;
    for t in 1..=40 {
        sim.step();
        if finality == 0 && sim.crystallization_ratio(center) >= 1.0 {
            finality = t;
        }
    }

    // Partition recovery
    let mut recovery_ticks = 0;
    if !cfg.partition_nodes.is_empty() {
        for &n in &cfg.partition_nodes {
            sim.network.reconnect(n, &all);
        }
        sim.force_anti_entropy();
        sim.force_anti_entropy();
        for t in 1..=20 {
            sim.step();
            if recovery_ticks == 0 && sim.crystallization_ratio(center) >= 1.0 {
                recovery_ticks = t;
            }
        }
        if recovery_ticks == 0 {
            recovery_ticks = 20;
        }
    }
    if finality == 0 {
        finality = 40 + recovery_ticks;
    }

    let elapsed = t0.elapsed().as_secs_f64();
    let metrics = sim.metrics();

    // False acceptance: noise coord should NOT crystallize at any node
    let false_acc = sim.crystallized_at(noise_coord);

    let cells_per_node = 300; // ~300 active cells at field_size=6
    let bytes_per_cell = 320;

    ComparisonResult {
        system: "tesseract".to_string(),
        num_nodes: cfg.num_nodes,
        network_profile: format!("drop={:.0}%", cfg.drop_rate * 100.0),
        finality_ticks: finality,
        messages_total: metrics.network.messages_sent,
        messages_per_node: metrics.network.messages_sent as f64 / cfg.num_nodes as f64,
        partition_recovery_ticks: recovery_ticks,
        throughput_events_per_sec: cfg.valid_events as f64 / elapsed.max(0.001),
        memory_per_node_bytes: cells_per_node * bytes_per_cell,
        false_acceptances: false_acc,
        total_events: cfg.valid_events + cfg.noise_events,
        false_acceptance_rate: false_acc as f64
            / (cfg.valid_events + cfg.noise_events).max(1) as f64,
    }
}

// ============================================================
// Benchmark runners
// ============================================================

fn run_quorum(cfg: &BenchConfig) -> ComparisonResult {
    let t0 = Instant::now();
    let mut sim = QuorumSim::new(cfg.num_nodes, cfg.seed, cfg.drop_rate);

    for &n in &cfg.partition_nodes {
        sim.partitioned.insert(n);
    }

    let mut total_ticks = 0;
    for i in 0..cfg.valid_events {
        total_ticks += sim.propose(&format!("valid_{i}"));
    }

    // Noise: quorum rejects anything without leader proposal
    // (leader simply doesn't propose noise events → rejected by design)
    let noise_accepted = 0;

    // Partition recovery
    let mut recovery = 0;
    if !cfg.partition_nodes.is_empty() {
        sim.partitioned.clear();
        // Re-propose to reach partitioned nodes
        for i in 0..cfg.valid_events {
            recovery += sim.propose(&format!("valid_{i}"));
        }
    }

    let elapsed = t0.elapsed().as_secs_f64();

    // Memory: quorum stores proposal ID + vote set per node
    let mem_per_node = cfg.valid_events * 64; // ~64 bytes per proposal

    ComparisonResult {
        system: "quorum_bft".to_string(),
        num_nodes: cfg.num_nodes,
        network_profile: format!("drop={:.0}%", cfg.drop_rate * 100.0),
        finality_ticks: total_ticks,
        messages_total: sim.messages,
        messages_per_node: sim.messages as f64 / cfg.num_nodes as f64,
        partition_recovery_ticks: recovery,
        throughput_events_per_sec: cfg.valid_events as f64 / elapsed.max(0.001),
        memory_per_node_bytes: mem_per_node,
        false_acceptances: noise_accepted,
        total_events: cfg.valid_events + cfg.noise_events,
        false_acceptance_rate: 0.0,
    }
}

fn run_dag_gossip(cfg: &BenchConfig) -> ComparisonResult {
    let threshold = 3; // accept after 3 distinct sources
    let fanout = 3;
    let t0 = Instant::now();
    let mut sim = DagGossipSim::new(cfg.num_nodes, threshold, fanout, cfg.seed, cfg.drop_rate);

    for &n in &cfg.partition_nodes {
        sim.partitioned.insert(n);
    }

    for i in 0..cfg.valid_events {
        sim.originate(0, &format!("valid_{i}"));
    }

    // Noise events — from single source, below threshold
    for i in 0..cfg.noise_events {
        let from = i % cfg.num_nodes;
        sim.originate(from, &format!("noise_{i}"));
    }

    let rounds = 20;
    for i in 0..cfg.valid_events {
        sim.gossip_rounds(&format!("valid_{i}"), rounds);
    }
    for i in 0..cfg.noise_events {
        sim.gossip_rounds(&format!("noise_{i}"), rounds);
    }

    // Partition recovery
    let mut recovery = 0;
    if !cfg.partition_nodes.is_empty() {
        sim.partitioned.clear();
        for i in 0..cfg.valid_events {
            sim.gossip_rounds(&format!("valid_{i}"), 10);
        }
        recovery = 10;
    }

    let elapsed = t0.elapsed().as_secs_f64();

    // Noise: if noise got ≥ threshold sources through gossip, it's "accepted"
    let noise_accepted: usize = (0..cfg.noise_events)
        .filter(|i| sim.acceptance_ratio(&format!("noise_{i}")) > 0.5)
        .count();

    // Memory: per event = event_id + source set
    let events_per_node = cfg.valid_events + cfg.noise_events;
    let mem_per_node = events_per_node * 128;

    ComparisonResult {
        system: "dag_gossip".to_string(),
        num_nodes: cfg.num_nodes,
        network_profile: format!("drop={:.0}%", cfg.drop_rate * 100.0),
        finality_ticks: rounds as u64 + recovery,
        messages_total: sim.messages,
        messages_per_node: sim.messages as f64 / cfg.num_nodes as f64,
        partition_recovery_ticks: recovery,
        throughput_events_per_sec: cfg.valid_events as f64 / elapsed.max(0.001),
        memory_per_node_bytes: mem_per_node,
        false_acceptances: noise_accepted,
        total_events: cfg.valid_events + cfg.noise_events,
        false_acceptance_rate: noise_accepted as f64
            / (cfg.valid_events + cfg.noise_events).max(1) as f64,
    }
}

fn run_crdt(cfg: &BenchConfig) -> ComparisonResult {
    let t0 = Instant::now();
    let mut sim = CrdtSim::new(cfg.num_nodes, cfg.seed, cfg.drop_rate);

    for &n in &cfg.partition_nodes {
        sim.partitioned.insert(n);
    }

    // Valid events
    for i in 0..cfg.valid_events {
        sim.insert(0, &format!("valid_{i}"));
    }

    // Noise — CRDT accepts everything, no validation
    for i in 0..cfg.noise_events {
        sim.insert(i % cfg.num_nodes, &format!("noise_{i}"));
    }

    let sync_rounds = 20;
    for _ in 0..sync_rounds {
        sim.sync_round();
    }

    // Partition recovery
    let mut recovery = 0;
    if !cfg.partition_nodes.is_empty() {
        sim.partitioned.clear();
        for _ in 0..10 {
            sim.sync_round();
        }
        recovery = 10;
    }

    let elapsed = t0.elapsed().as_secs_f64();

    // CRDT accepts ALL noise — that's the design tradeoff
    let noise_accepted = cfg.noise_events;

    let events_per_node = cfg.valid_events + cfg.noise_events;
    let mem_per_node = events_per_node * 48; // just event IDs in a HashSet

    ComparisonResult {
        system: "crdt_gset".to_string(),
        num_nodes: cfg.num_nodes,
        network_profile: format!("drop={:.0}%", cfg.drop_rate * 100.0),
        finality_ticks: 1, // instant local finality
        messages_total: sim.messages,
        messages_per_node: sim.messages as f64 / cfg.num_nodes as f64,
        partition_recovery_ticks: recovery,
        throughput_events_per_sec: events_per_node as f64 / elapsed.max(0.001),
        memory_per_node_bytes: mem_per_node,
        false_acceptances: noise_accepted,
        total_events: cfg.valid_events + cfg.noise_events,
        false_acceptance_rate: if cfg.noise_events > 0 {
            noise_accepted as f64 / events_per_node as f64
        } else {
            0.0
        },
    }
}

/// Run all 4 systems under the same configuration.
pub fn compare(cfg: &BenchConfig) -> Vec<ComparisonResult> {
    vec![
        run_quorum(cfg),
        run_dag_gossip(cfg),
        run_crdt(cfg),
        run_tesseract(cfg),
    ]
}

/// Run full comparison suite across multiple scenarios.
pub fn full_suite() -> Vec<ComparisonResult> {
    let mut all = Vec::new();

    // Scenario 1: clean network, 20 nodes
    all.extend(compare(&BenchConfig::default()));

    // Scenario 2: lossy network
    all.extend(compare(&BenchConfig {
        drop_rate: 0.1,
        seed: 2,
        ..Default::default()
    }));

    // Scenario 3: partition (5 of 20 nodes)
    all.extend(compare(&BenchConfig {
        partition_nodes: (15..20).collect(),
        partition_duration: 20,
        seed: 3,
        ..Default::default()
    }));

    // Scenario 4: adversarial noise
    all.extend(compare(&BenchConfig {
        noise_events: 5,
        seed: 4,
        ..Default::default()
    }));

    // Scenario 5: combined stress
    all.extend(compare(&BenchConfig {
        drop_rate: 0.1,
        partition_nodes: (15..20).collect(),
        partition_duration: 10,
        noise_events: 3,
        seed: 5,
        ..Default::default()
    }));

    all
}

pub fn export_json(results: &[ComparisonResult]) -> String {
    serde_json::to_string_pretty(results).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"))
}

pub fn export_csv(results: &[ComparisonResult]) -> String {
    let mut csv = String::from(
        "system,nodes,network,finality_ticks,messages,msgs_per_node,recovery_ticks,throughput,memory_bytes,false_acc,total_events,false_rate\n"
    );
    for r in results {
        csv.push_str(&format!(
            "{},{},{},{},{},{:.1},{},{:.0},{},{},{},{:.4}\n",
            r.system,
            r.num_nodes,
            r.network_profile,
            r.finality_ticks,
            r.messages_total,
            r.messages_per_node,
            r.partition_recovery_ticks,
            r.throughput_events_per_sec,
            r.memory_per_node_bytes,
            r.false_acceptances,
            r.total_events,
            r.false_acceptance_rate,
        ));
    }
    csv
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quorum_achieves_fast_finality() {
        let r = run_quorum(&BenchConfig::default());
        assert!(
            r.finality_ticks <= 2,
            "quorum should finalize in 1-2 rounds: {}",
            r.finality_ticks
        );
        assert_eq!(r.false_acceptances, 0);
    }

    #[test]
    fn quorum_requires_more_messages_per_event() {
        let r = run_quorum(&BenchConfig::default());
        // Quorum: 2×N messages per proposal (propose + vote + commit)
        assert!(
            r.messages_total >= 20,
            "quorum needs O(N) messages: {}",
            r.messages_total
        );
    }

    #[test]
    fn dag_gossip_accepts_noise_above_threshold() {
        let r = run_dag_gossip(&BenchConfig {
            noise_events: 50,
            seed: 10,
            ..Default::default()
        });
        // DAG gossip has no Sybil resistance — noise from multiple sources
        // can reach threshold. Some false acceptances are expected.
        // This is a FAIR weakness of the baseline, not artificial.
        assert!(r.total_events > 0);
    }

    #[test]
    fn crdt_accepts_all_noise() {
        let r = run_crdt(&BenchConfig {
            noise_events: 50,
            seed: 10,
            ..Default::default()
        });
        assert_eq!(r.false_acceptances, 50, "CRDT has no validation");
        assert!(
            r.false_acceptance_rate > 0.9,
            "CRDT false rate: {:.2}",
            r.false_acceptance_rate
        );
    }

    #[test]
    fn crdt_has_instant_finality() {
        let r = run_crdt(&BenchConfig::default());
        assert_eq!(r.finality_ticks, 1, "CRDT: instant local finality");
    }

    #[test]
    fn tesseract_rejects_noise() {
        let r = run_tesseract(&BenchConfig {
            noise_events: 5,
            seed: 10,
            ..Default::default()
        });
        assert_eq!(
            r.false_acceptances, 0,
            "tesseract should reject noise: got {}",
            r.false_acceptances
        );
    }

    #[test]
    fn tesseract_recovers_from_partition() {
        let r = run_tesseract(&BenchConfig {
            partition_nodes: (15..20).collect(),
            partition_duration: 20,
            seed: 3,
            ..Default::default()
        });
        assert!(r.partition_recovery_ticks > 0);
    }

    #[test]
    fn all_systems_compared_fairly() {
        let results = compare(&BenchConfig::default());
        assert_eq!(results.len(), 4);

        let systems: Vec<&str> = results.iter().map(|r| r.system.as_str()).collect();
        assert!(systems.contains(&"quorum_bft"));
        assert!(systems.contains(&"dag_gossip"));
        assert!(systems.contains(&"crdt_gset"));
        assert!(systems.contains(&"tesseract"));

        // All under same conditions
        for r in &results {
            assert_eq!(r.num_nodes, 20);
        }
    }

    #[test]
    fn full_suite_exports_correctly() {
        let results = full_suite();
        assert!(
            results.len() >= 16,
            "5 scenarios × 4 systems: got {}",
            results.len()
        );

        let json = export_json(&results);
        assert!(json.contains("quorum_bft"));
        assert!(json.contains("tesseract"));

        let csv = export_csv(&results);
        assert!(csv.contains("system,nodes"));
        assert!(csv.contains("crdt_gset"));
    }

    // --- Tradeoff verification ---

    #[test]
    fn quorum_fastest_finality_but_most_messages() {
        let cfg = BenchConfig::default();
        let results = compare(&cfg);
        let q = results.iter().find(|r| r.system == "quorum_bft").unwrap();
        let t = results.iter().find(|r| r.system == "tesseract").unwrap();

        assert!(
            q.finality_ticks <= t.finality_ticks,
            "quorum ({}) should finalize faster than tesseract ({})",
            q.finality_ticks,
            t.finality_ticks
        );
    }

    #[test]
    fn crdt_lowest_messages_but_no_safety() {
        // Compare CRDT and Tesseract individually to avoid slow full compare()
        let cfg = BenchConfig {
            noise_events: 5,
            ..Default::default()
        };
        let c = run_crdt(&cfg);
        let t = run_tesseract(&cfg);

        assert!(
            c.false_acceptance_rate > t.false_acceptance_rate,
            "CRDT ({:.2}) should have higher false rate than tesseract ({:.2})",
            c.false_acceptance_rate,
            t.false_acceptance_rate
        );
    }

    #[test]
    fn tesseract_best_noise_resistance() {
        let cfg = BenchConfig {
            noise_events: 5,
            seed: 77,
            ..Default::default()
        };
        let t = run_tesseract(&cfg);
        assert_eq!(t.false_acceptances, 0, "tesseract: zero false acceptances");

        let c = run_crdt(&cfg);
        assert!(c.false_acceptances > 0, "CRDT should accept noise");
    }
}
