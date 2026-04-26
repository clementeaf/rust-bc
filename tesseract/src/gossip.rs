//! Gossip protocol — epidemic attestation propagation with anti-entropy.
//!
//! Push gossip: attestations forwarded to `fanout` random peers on receipt.
//! Anti-entropy: periodic pull-based reconciliation ensures SEC (Strong
//! Eventual Consistency). Nodes compare seen-sets and exchange missing
//! attestations.
//!
//! With both mechanisms, the system guarantees:
//!   - All correct nodes converge to the same crystallized core
//!   - After partition heals + anti-entropy round: 100% convergence
//!   - Zero false crystallizations (safety preserved)
//!
//! # Attestation bundles (transport optimization)
//!
//! Bundles pack all 4 dimension attestations into a single network message.
//! This is PURELY a transport optimization — it does NOT change the
//! independence model:
//!
//!   - Independence comes from **origin**: each dimension requires a
//!     cryptographically distinct validator bound to that dimension
//!   - Independence comes from **causal history**: validators with
//!     shared ancestry get penalized by σ_eff (see adversarial.rs)
//!   - Independence does NOT come from **transport**: whether attestations
//!     arrive in 1 message or 4, the σ-independence check is identical
//!
//! A bundle is equivalent to receiving 4 individual messages simultaneously.
//! The receiver applies each attestation independently and checks σ=4
//! exactly as if they arrived separately.
//!
//! # Fanout requirement
//!
//! Convergence requires fanout ≥ ln(N) (epidemic threshold). This is an
//! **explicit condition**, not a universal guarantee:
//!   - Below ln(N): some nodes may never receive attestations (gossip dies)
//!   - At ln(N): O(log N) rounds to reach all nodes w.h.p.
//!   - Above ln(N): faster but more messages
//!
//! The system does NOT enforce minimum fanout — the caller must configure
//! it appropriately for the network size.
//!
//! # Bundle attack surface
//!
//! Bundles introduce these attack vectors (tested below):
//!   - **Bundle loss**: bundle dropped by network → falls back to individual
//!     gossip + anti-entropy (no single point of failure)
//!   - **Bundle replay**: duplicate bundles → dedup rejects (idempotent)
//!   - **Partial bundle**: attacker sends bundle with <4 dims → receiver
//!     applies what it gets, waits for remaining dims via gossip
//!   - **Corrupted bundle**: attestation with wrong validator_id → σ check
//!     rejects (σ < 4 if validator not exclusive)
//!   - **Wave amplification**: cascading waves → bounded by dedup
//!     (each node forwards a bundle at most once per event)

use crate::network_sim::{NetworkMessage, NetworkSim, NodeId};
use crate::{Coord, Dimension, Field};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::{HashMap, HashSet};

/// Attestation record — stored for anti-entropy reconciliation.
#[derive(Clone, Debug)]
pub struct AttestationRecord {
    pub coord: Coord,
    pub event_id: String,
    pub dimension: Dimension,
    pub validator_id: String,
}

/// A simulated node with local state.
pub struct SimNode {
    pub id: NodeId,
    pub field: Field,
    /// Dedup keys for seen attestations.
    seen: HashSet<String>,
    /// Full records for anti-entropy exchange.
    records: Vec<AttestationRecord>,
    pub local_tick: u64,
    /// 0 = no evolve (crystallization in attest()); >0 = evolve every N ticks.
    pub evolve_interval: u64,
}

impl SimNode {
    pub fn new(id: NodeId, field_size: usize) -> Self {
        Self {
            id,
            field: Field::new(field_size),
            seen: HashSet::new(),
            records: Vec::new(),
            local_tick: 0,
            evolve_interval: 0,
        }
    }

    fn dedup_key(event_id: &str, dimension: Dimension, validator_id: &str) -> String {
        format!("{event_id}:{dimension}:{validator_id}")
    }

    pub fn apply_attestation(
        &mut self,
        coord: Coord,
        event_id: &str,
        dimension: Dimension,
        validator_id: &str,
    ) -> bool {
        let key = Self::dedup_key(event_id, dimension, validator_id);
        if !self.seen.insert(key) {
            return false;
        }
        self.field.attest(coord, event_id, dimension, validator_id);
        self.records.push(AttestationRecord {
            coord,
            event_id: event_id.to_string(),
            dimension,
            validator_id: validator_id.to_string(),
        });
        true
    }

    pub fn seen_keys(&self) -> &HashSet<String> {
        &self.seen
    }

    pub fn records(&self) -> &[AttestationRecord] {
        &self.records
    }

    /// Check if this node has all 4 dimensions for an event (can build bundle).
    pub fn has_full_event(&self, event_id: &str) -> bool {
        Dimension::ALL.iter().all(|dim| {
            self.records
                .iter()
                .any(|r| r.event_id == event_id && r.dimension == *dim)
        })
    }

    /// Build an attestation bundle for an event (all dims this node knows).
    pub fn build_bundle(&self, event_id: &str) -> Vec<AttestationRecord> {
        self.records
            .iter()
            .filter(|r| r.event_id == event_id)
            .cloned()
            .collect()
    }

    pub fn tick(&mut self) {
        self.local_tick += 1;
        if self.evolve_interval > 0 && self.local_tick % self.evolve_interval == 0 {
            self.field.evolve();
        }
    }
}

/// Gossip configuration.
#[derive(Clone, Debug)]
pub struct GossipConfig {
    pub fanout: usize,
    pub field_size: usize,
    /// Anti-entropy interval: reconcile with a random peer every N ticks. 0 = disabled.
    pub anti_entropy_interval: u64,
    pub seed: u64,
}

impl Default for GossipConfig {
    fn default() -> Self {
        Self {
            fanout: 3,
            field_size: 6,
            anti_entropy_interval: 0,
            seed: 42,
        }
    }
}

/// Distributed simulation with push gossip + pull anti-entropy.
pub struct DistributedSim {
    pub nodes: HashMap<NodeId, SimNode>,
    pub network: NetworkSim,
    gossip_config: GossipConfig,
    rng: StdRng,
    node_ids: Vec<NodeId>,
    pub attestations_originated: u64,
    pub attestations_applied: u64,
    pub duplicates_rejected: u64,
    pub anti_entropy_rounds: u64,
    pub anti_entropy_repairs: u64,
}

impl DistributedSim {
    pub fn new(
        num_nodes: usize,
        gossip_config: GossipConfig,
        net_config: crate::network_sim::NetworkConfig,
    ) -> Self {
        let mut nodes = HashMap::new();
        let node_ids: Vec<NodeId> = (0..num_nodes).collect();
        for &id in &node_ids {
            nodes.insert(id, SimNode::new(id, gossip_config.field_size));
        }

        Self {
            nodes,
            network: NetworkSim::new(net_config),
            rng: StdRng::seed_from_u64(gossip_config.seed),
            gossip_config,
            node_ids,
            attestations_originated: 0,
            attestations_applied: 0,
            duplicates_rejected: 0,
            anti_entropy_rounds: 0,
            anti_entropy_repairs: 0,
        }
    }

    pub fn originate_attestation(
        &mut self,
        origin_node: NodeId,
        coord: Coord,
        event_id: &str,
        dimension: Dimension,
        validator_id: &str,
    ) {
        self.attestations_originated += 1;
        if let Some(node) = self.nodes.get_mut(&origin_node) {
            if node.apply_attestation(coord, event_id, dimension, validator_id) {
                self.attestations_applied += 1;
            }
        }
        self.gossip_forward(origin_node, coord, event_id, dimension, validator_id);
    }

    fn gossip_forward(
        &mut self,
        from: NodeId,
        coord: Coord,
        event_id: &str,
        dimension: Dimension,
        validator_id: &str,
    ) {
        let fanout = self
            .gossip_config
            .fanout
            .min(self.node_ids.len().saturating_sub(1));
        let mut candidates: Vec<NodeId> = self
            .node_ids
            .iter()
            .copied()
            .filter(|&id| id != from)
            .collect();

        let msg = NetworkMessage::Attestation {
            coord,
            event_id: event_id.to_string(),
            dimension,
            validator_id: validator_id.to_string(),
        };

        for _ in 0..fanout {
            if candidates.is_empty() {
                break;
            }
            let idx = self.rng.gen_range(0..candidates.len());
            let target = candidates.swap_remove(idx);
            self.network.send(from, target, msg.clone());
        }
    }

    /// Anti-entropy: pick a random peer, compare seen-sets, apply missing records.
    /// This is instantaneous (no network delay) — models the reconciliation
    /// as an atomic exchange. In production this would be a request-response.
    fn anti_entropy_round(&mut self) {
        if self.node_ids.len() < 2 {
            return;
        }

        // Each node reconciles with one random reachable peer
        let pairs: Vec<(NodeId, NodeId)> = self
            .node_ids
            .iter()
            .map(|&a| {
                let mut candidates: Vec<NodeId> = self
                    .node_ids
                    .iter()
                    .copied()
                    .filter(|&b| b != a && self.network.can_reach(a, b))
                    .collect();
                if candidates.is_empty() {
                    (a, a) // no reachable peer
                } else {
                    let idx = self.rng.gen_range(0..candidates.len());
                    (a, candidates.swap_remove(idx))
                }
            })
            .collect();

        self.anti_entropy_rounds += 1;

        for (a, b) in pairs {
            if a == b {
                continue;
            }

            // Find records in B that A doesn't have
            let a_seen = self.nodes[&a].seen_keys().clone();
            let missing: Vec<AttestationRecord> = self.nodes[&b]
                .records()
                .iter()
                .filter(|r| {
                    let key = SimNode::dedup_key(&r.event_id, r.dimension, &r.validator_id);
                    !a_seen.contains(&key)
                })
                .cloned()
                .collect();

            // Apply missing to A
            for r in missing {
                if let Some(node_a) = self.nodes.get_mut(&a) {
                    if node_a.apply_attestation(r.coord, &r.event_id, r.dimension, &r.validator_id)
                    {
                        self.anti_entropy_repairs += 1;
                    }
                }
            }
        }
    }

    /// Send a bundle of all known attestations for an event to fanout peers.
    fn send_bundle(&mut self, from: NodeId, coord: Coord, event_id: &str) {
        let bundle_atts: Vec<(Dimension, String)> = self.nodes[&from]
            .build_bundle(event_id)
            .iter()
            .map(|r| (r.dimension, r.validator_id.clone()))
            .collect();
        if bundle_atts.is_empty() {
            return;
        }

        let fanout = self
            .gossip_config
            .fanout
            .min(self.node_ids.len().saturating_sub(1));
        let mut candidates: Vec<NodeId> = self
            .node_ids
            .iter()
            .copied()
            .filter(|&id| id != from)
            .collect();

        let msg = NetworkMessage::AttestationBundle {
            coord,
            event_id: event_id.to_string(),
            attestations: bundle_atts,
        };

        for _ in 0..fanout {
            if candidates.is_empty() {
                break;
            }
            let idx = self.rng.gen_range(0..candidates.len());
            let target = candidates.swap_remove(idx);
            self.network.send(from, target, msg.clone());
        }
    }

    pub fn step(&mut self) {
        let delivered = self.network.advance();

        // Track which (node, event) pairs newly crystallized this step
        let mut crystallization_waves: Vec<(NodeId, Coord, String)> = Vec::new();
        let mut forwards: Vec<(NodeId, Coord, String, Dimension, String)> = Vec::new();

        for msg in delivered {
            match msg.payload {
                NetworkMessage::Attestation {
                    coord,
                    event_id,
                    dimension,
                    validator_id,
                } => {
                    let to = msg.to;
                    if let Some(node) = self.nodes.get_mut(&to) {
                        let was_crystallized = node.field.get(coord).crystallized;
                        if node.apply_attestation(coord, &event_id, dimension, &validator_id) {
                            self.attestations_applied += 1;
                            forwards.push((to, coord, event_id.clone(), dimension, validator_id));

                            // Crystallization wave: if this attestation caused crystallization
                            if !was_crystallized && node.field.get(coord).crystallized {
                                crystallization_waves.push((to, coord, event_id));
                            }
                        } else {
                            self.duplicates_rejected += 1;
                        }
                    }
                }
                NetworkMessage::AttestationBundle {
                    coord,
                    event_id,
                    attestations,
                } => {
                    let to = msg.to;
                    if let Some(node) = self.nodes.get_mut(&to) {
                        let was_crystallized = node.field.get(coord).crystallized;
                        let mut any_new = false;
                        for (dim, vid) in &attestations {
                            if node.apply_attestation(coord, &event_id, *dim, vid) {
                                self.attestations_applied += 1;
                                any_new = true;
                            } else {
                                self.duplicates_rejected += 1;
                            }
                        }
                        // If bundle caused crystallization, propagate wave
                        if any_new && !was_crystallized && node.field.get(coord).crystallized {
                            crystallization_waves.push((to, coord, event_id));
                        }
                    }
                }
                _ => {}
            }
        }

        // Forward individual attestations (backward compat)
        for (from, coord, event_id, dimension, validator_id) in forwards {
            // If this node now has the full event, send bundle instead of single
            if self.nodes[&from].has_full_event(&event_id) {
                self.send_bundle(from, coord, &event_id);
            } else {
                self.gossip_forward(from, coord, &event_id, dimension, &validator_id);
            }
        }

        // Crystallization wave: nodes that just crystallized push bundles eagerly
        for (node_id, coord, event_id) in crystallization_waves {
            self.send_bundle(node_id, coord, &event_id);
        }

        // Anti-entropy
        let interval = self.gossip_config.anti_entropy_interval;
        if interval > 0 && self.network.tick % interval == 0 {
            self.anti_entropy_round();
        }

        for node in self.nodes.values_mut() {
            node.tick();
        }
    }

    pub fn run(&mut self, ticks: u64) {
        for _ in 0..ticks {
            self.step();
        }
    }

    /// Force a single anti-entropy round (useful after partition heals).
    pub fn force_anti_entropy(&mut self) {
        self.anti_entropy_round();
    }

    pub fn originate_full_event(&mut self, node: NodeId, coord: Coord, event_id: &str) {
        for (dim, vid) in [
            (Dimension::Temporal, "val_t"),
            (Dimension::Context, "val_c"),
            (Dimension::Origin, "val_o"),
            (Dimension::Verification, "val_v"),
        ] {
            self.originate_attestation(node, coord, event_id, dim, vid);
        }
        // Send bundle immediately — receivers get all 4 dims in one message
        self.send_bundle(node, coord, event_id);
    }

    pub fn crystallized_at(&self, coord: Coord) -> usize {
        self.nodes
            .values()
            .filter(|n| n.field.get(coord).crystallized)
            .count()
    }

    pub fn consensus_at(&self, coord: Coord) -> bool {
        let first = self
            .nodes
            .values()
            .next()
            .map(|n| n.field.get(coord).crystallized);
        self.nodes
            .values()
            .all(|n| Some(n.field.get(coord).crystallized) == first)
    }

    pub fn crystallization_ratio(&self, coord: Coord) -> f64 {
        let total = self.nodes.len();
        if total == 0 {
            return 0.0;
        }
        self.crystallized_at(coord) as f64 / total as f64
    }

    pub fn false_crystallizations(&self, coord: Coord, origin_node: NodeId) -> usize {
        let origin_crystallized = self
            .nodes
            .get(&origin_node)
            .map(|n| n.field.get(coord).crystallized)
            .unwrap_or(false);
        if origin_crystallized {
            return 0;
        }
        self.nodes
            .values()
            .filter(|n| n.id != origin_node && n.field.get(coord).crystallized)
            .count()
    }

    pub fn metrics(&self) -> DistributedMetrics {
        DistributedMetrics {
            num_nodes: self.nodes.len(),
            network_tick: self.network.tick,
            attestations_originated: self.attestations_originated,
            attestations_applied: self.attestations_applied,
            duplicates_rejected: self.duplicates_rejected,
            anti_entropy_rounds: self.anti_entropy_rounds,
            anti_entropy_repairs: self.anti_entropy_repairs,
            network: self.network.metrics_summary(),
        }
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct DistributedMetrics {
    pub num_nodes: usize,
    pub network_tick: u64,
    pub attestations_originated: u64,
    pub attestations_applied: u64,
    pub duplicates_rejected: u64,
    pub anti_entropy_rounds: u64,
    pub anti_entropy_repairs: u64,
    pub network: crate::network_sim::NetworkMetrics,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network_sim::NetworkConfig;

    fn coord(t: usize, c: usize, o: usize, v: usize) -> Coord {
        Coord { t, c, o, v }
    }

    #[test]
    fn gossip_propagates_to_all_nodes() {
        let mut sim = DistributedSim::new(
            10,
            GossipConfig {
                fanout: 5,
                field_size: 6,
                ..Default::default()
            },
            NetworkConfig::default(),
        );
        let center = coord(3, 3, 3, 3);
        sim.originate_full_event(0, center, "test_event");
        sim.run(30);
        assert!(
            sim.crystallization_ratio(center) > 0.8,
            "gossip: {:.0}%",
            sim.crystallization_ratio(center) * 100.0
        );
    }

    #[test]
    fn dedup_prevents_redundant_processing() {
        let mut sim = DistributedSim::new(
            5,
            GossipConfig {
                fanout: 4,
                field_size: 6,
                ..Default::default()
            },
            NetworkConfig {
                dup_rate: 0.5,
                ..NetworkConfig::default()
            },
        );
        sim.originate_full_event(0, coord(3, 3, 3, 3), "event");
        sim.run(20);
        assert!(sim.duplicates_rejected > 0);
    }

    #[test]
    fn lossy_network_still_converges() {
        let mut sim = DistributedSim::new(
            10,
            GossipConfig {
                fanout: 4,
                field_size: 6,
                ..Default::default()
            },
            NetworkConfig::lossy(),
        );
        let center = coord(3, 3, 3, 3);
        sim.originate_full_event(0, center, "lossy_event");
        sim.run(60);
        assert!(
            sim.crystallization_ratio(center) > 0.5,
            "lossy: {:.0}%",
            sim.crystallization_ratio(center) * 100.0
        );
    }

    #[test]
    fn partition_heals_with_anti_entropy() {
        let mut sim = DistributedSim::new(
            6,
            GossipConfig {
                fanout: 3,
                field_size: 6,
                anti_entropy_interval: 5,
                ..Default::default()
            },
            NetworkConfig::default(),
        );

        let all: Vec<NodeId> = (0..6).collect();
        sim.network.isolate(4, &all);
        sim.network.isolate(5, &all);

        let center = coord(3, 3, 3, 3);
        sim.originate_full_event(0, center, "part_event");
        sim.run(20);
        assert!(
            !sim.nodes[&4].field.get(center).crystallized,
            "isolated during partition"
        );

        sim.network.reconnect(4, &all);
        sim.network.reconnect(5, &all);
        // Anti-entropy will fire within 5 ticks
        sim.run(10);

        assert_eq!(
            sim.crystallization_ratio(center),
            1.0,
            "anti-entropy should achieve 100% after heal"
        );
    }

    #[test]
    fn force_anti_entropy_achieves_full_convergence() {
        let mut sim = DistributedSim::new(
            10,
            GossipConfig {
                fanout: 2,
                field_size: 6,
                ..Default::default()
            },
            NetworkConfig::lossy(),
        );
        let center = coord(3, 3, 3, 3);
        sim.originate_full_event(0, center, "event");
        sim.run(30);

        // May not be 100% due to losses
        let before = sim.crystallization_ratio(center);

        // Force reconciliation
        sim.force_anti_entropy();
        sim.force_anti_entropy(); // two rounds for full propagation

        let after = sim.crystallization_ratio(center);
        assert!(
            after >= before,
            "anti-entropy should not decrease convergence"
        );
        assert_eq!(after, 1.0, "forced anti-entropy should reach 100%");
    }

    #[test]
    fn no_false_crystallizations() {
        let mut sim = DistributedSim::new(
            10,
            GossipConfig {
                fanout: 3,
                field_size: 6,
                anti_entropy_interval: 10,
                ..Default::default()
            },
            NetworkConfig::lossy(),
        );
        sim.originate_full_event(0, coord(3, 3, 3, 3), "safe");
        sim.run(40);
        assert_eq!(sim.false_crystallizations(coord(1, 1, 1, 1), 0), 0);
    }

    #[test]
    fn anti_entropy_metrics_tracked() {
        let mut sim = DistributedSim::new(
            5,
            GossipConfig {
                fanout: 2,
                field_size: 6,
                anti_entropy_interval: 5,
                ..Default::default()
            },
            NetworkConfig::default(),
        );
        sim.originate_full_event(0, coord(3, 3, 3, 3), "event");
        sim.run(20);

        let m = sim.metrics();
        assert!(m.anti_entropy_rounds > 0, "should have run anti-entropy");
    }

    // --- Bundle attack surface tests ---

    #[test]
    fn bundle_loss_falls_back_to_individual_gossip() {
        // High drop rate → bundles likely lost. Individual gossip + anti-entropy
        // must still achieve convergence.
        let mut sim = DistributedSim::new(
            10,
            GossipConfig {
                fanout: 5,
                field_size: 6,
                anti_entropy_interval: 5,
                ..Default::default()
            },
            NetworkConfig {
                drop_rate: 0.4,
                ..NetworkConfig::default()
            },
        );
        let center = coord(3, 3, 3, 3);
        sim.originate_full_event(0, center, "lost_bundle");
        sim.run(40);
        sim.force_anti_entropy();
        sim.force_anti_entropy();

        assert_eq!(
            sim.crystallization_ratio(center),
            1.0,
            "should converge despite 40% bundle loss via anti-entropy fallback"
        );
    }

    #[test]
    fn bundle_replay_is_idempotent() {
        // High dup rate → bundles arrive multiple times. Dedup must reject.
        let mut sim = DistributedSim::new(
            5,
            GossipConfig {
                fanout: 4,
                field_size: 6,
                ..Default::default()
            },
            NetworkConfig {
                dup_rate: 0.8,
                ..NetworkConfig::default()
            },
        );
        let center = coord(3, 3, 3, 3);
        sim.originate_full_event(0, center, "replayed_bundle");
        sim.run(20);

        assert!(
            sim.duplicates_rejected > 0,
            "should reject duplicate bundles"
        );
        // Safety: no false crystallizations from replay
        assert_eq!(sim.false_crystallizations(coord(1, 1, 1, 1), 0), 0);
        // Valid event still crystallized
        assert!(sim.nodes[&0].field.get(center).crystallized);
    }

    #[test]
    fn partial_bundle_waits_for_remaining_dims() {
        // Simulate partial bundle: originate only 2 dims, not full event.
        // Receiver should NOT crystallize (σ < 4).
        let mut sim = DistributedSim::new(
            5,
            GossipConfig {
                fanout: 4,
                field_size: 6,
                ..Default::default()
            },
            NetworkConfig::default(),
        );
        let center = coord(3, 3, 3, 3);

        // Only 2 dimensions
        sim.originate_attestation(0, center, "partial", Dimension::Temporal, "val_t");
        sim.originate_attestation(0, center, "partial", Dimension::Context, "val_c");
        sim.run(20);

        // No node should crystallize with only 2 dims
        assert_eq!(
            sim.crystallized_at(center),
            0,
            "partial bundle (2 dims) must not crystallize"
        );

        // Now deliver remaining 2 dims → should crystallize
        sim.originate_attestation(0, center, "partial", Dimension::Origin, "val_o");
        sim.originate_attestation(0, center, "partial", Dimension::Verification, "val_v");
        sim.run(20);

        assert!(
            sim.crystallization_ratio(center) > 0.8,
            "full attestation after partial should converge"
        );
    }

    #[test]
    fn corrupted_bundle_rejected_by_sigma() {
        // Attacker sends "bundle" where all 4 dims use the same validator_id.
        // σ=0 → should NOT crystallize.
        let mut sim = DistributedSim::new(
            5,
            GossipConfig {
                fanout: 4,
                field_size: 6,
                ..Default::default()
            },
            NetworkConfig::default(),
        );
        let center = coord(3, 3, 3, 3);

        // Same validator on all dims → σ=0
        for dim in Dimension::ALL {
            sim.originate_attestation(0, center, "fake_bundle", dim, "same_validator");
        }
        sim.run(20);

        assert_eq!(
            sim.crystallized_at(center),
            0,
            "corrupted bundle (same validator) must not crystallize"
        );
    }

    #[test]
    fn wave_amplification_bounded_by_dedup() {
        // Multiple origins sending the same event → waves converge, don't explode.
        let mut sim = DistributedSim::new(
            10,
            GossipConfig {
                fanout: 5,
                field_size: 6,
                ..Default::default()
            },
            NetworkConfig::default(),
        );
        let center = coord(3, 3, 3, 3);

        // 3 nodes all originate the same event simultaneously
        sim.originate_full_event(0, center, "multi_origin");
        sim.originate_full_event(3, center, "multi_origin");
        sim.originate_full_event(7, center, "multi_origin");
        sim.run(20);

        // Should converge normally
        assert_eq!(
            sim.crystallization_ratio(center),
            1.0,
            "multiple origins should converge"
        );
        // Dedup should have caught the redundant waves
        assert!(
            sim.duplicates_rejected > 0,
            "dedup should reject redundant waves"
        );
        // Messages should not be dramatically more than single-origin
        let m = sim.metrics();
        assert!(
            m.network.messages_sent < 500,
            "wave amplification should be bounded: {} messages",
            m.network.messages_sent
        );
    }

    #[test]
    fn low_fanout_below_ln_n_fails_to_converge() {
        // Fanout=1 with 20 nodes: below ln(20)≈3. Gossip should die out.
        let mut sim = DistributedSim::new(
            20,
            GossipConfig {
                fanout: 1,
                field_size: 6,
                ..Default::default()
            },
            NetworkConfig::default(),
        );
        let center = coord(3, 3, 3, 3);
        sim.originate_full_event(0, center, "low_fanout");
        sim.run(30);

        let ratio = sim.crystallization_ratio(center);
        // With fanout=1, unlikely to reach all 20 nodes
        assert!(
            ratio < 1.0,
            "fanout=1 should not guarantee full convergence: {:.0}%",
            ratio * 100.0
        );
    }
}
