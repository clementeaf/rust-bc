//! Gossip protocol — epidemic attestation propagation.
//!
//! Each node maintains a local Field and propagates attestations to
//! random peers via the network simulator. Deduplication ensures
//! convergence despite duplicates and reordering.
//!
//! Protocol:
//!   1. Node receives attestation (local or remote)
//!   2. Applies to local field
//!   3. Forwards to `fanout` random peers (if not already seen)
//!   4. Peers repeat from step 2
//!
//! Convergence: with fanout ≥ ln(N), every attestation reaches all
//! nodes in O(log N) rounds with high probability (epidemic spreading).

use std::collections::{HashMap, HashSet};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use crate::{Coord, Dimension, Field};
use crate::network_sim::{NetworkSim, NetworkMessage, NodeId};

/// A simulated node with local state.
pub struct SimNode {
    pub id: NodeId,
    pub field: Field,
    /// Attestations already seen (dedup key: event_id + dimension + validator_id).
    seen: HashSet<String>,
    /// Local tick counter (may diverge from network tick due to clock skew).
    pub local_tick: u64,
    /// Processing speed: evolve every `evolve_interval` ticks.
    pub evolve_interval: u64,
}

impl SimNode {
    pub fn new(id: NodeId, field_size: usize) -> Self {
        Self {
            id,
            field: Field::new(field_size),
            seen: HashSet::new(),
            local_tick: 0,
            evolve_interval: 0, // 0 = no evolve; crystallization in attest()
        }
    }

    /// Dedup key for an attestation.
    fn dedup_key(event_id: &str, dimension: Dimension, validator_id: &str) -> String {
        format!("{event_id}:{dimension}:{validator_id}")
    }

    /// Apply an attestation to the local field.
    /// Returns true if this was new (not a duplicate).
    pub fn apply_attestation(
        &mut self,
        coord: Coord,
        event_id: &str,
        dimension: Dimension,
        validator_id: &str,
    ) -> bool {
        let key = Self::dedup_key(event_id, dimension, validator_id);
        if !self.seen.insert(key) {
            return false; // duplicate
        }
        self.field.attest(coord, event_id, dimension, validator_id);
        true
    }

    /// Advance local clock and optionally evolve the field.
    /// evolve_interval=0 means never evolve (crystallization happens in attest()).
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
    /// Number of random peers to forward each attestation to.
    pub fanout: usize,
    /// Field size for each node.
    pub field_size: usize,
    /// RNG seed for peer selection.
    pub seed: u64,
}

impl Default for GossipConfig {
    fn default() -> Self {
        Self {
            fanout: 3,
            field_size: 6,
            seed: 42,
        }
    }
}

/// Distributed simulation: nodes + network + gossip.
pub struct DistributedSim {
    pub nodes: HashMap<NodeId, SimNode>,
    pub network: NetworkSim,
    gossip_config: GossipConfig,
    rng: StdRng,
    node_ids: Vec<NodeId>,
    // --- Metrics ---
    pub attestations_originated: u64,
    pub attestations_applied: u64,
    pub duplicates_rejected: u64,
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
        }
    }

    /// Originate an attestation at a specific node and start gossip.
    pub fn originate_attestation(
        &mut self,
        origin_node: NodeId,
        coord: Coord,
        event_id: &str,
        dimension: Dimension,
        validator_id: &str,
    ) {
        self.attestations_originated += 1;

        // Apply locally
        if let Some(node) = self.nodes.get_mut(&origin_node) {
            if node.apply_attestation(coord, event_id, dimension, validator_id) {
                self.attestations_applied += 1;
            }
        }

        // Gossip to fanout random peers
        self.gossip_forward(origin_node, coord, event_id, dimension, validator_id);
    }

    /// Forward attestation to random peers.
    fn gossip_forward(
        &mut self,
        from: NodeId,
        coord: Coord,
        event_id: &str,
        dimension: Dimension,
        validator_id: &str,
    ) {
        let fanout = self.gossip_config.fanout.min(self.node_ids.len() - 1);
        let mut targets = Vec::new();

        // Select random peers (without replacement)
        let mut candidates: Vec<NodeId> = self.node_ids.iter()
            .copied()
            .filter(|&id| id != from)
            .collect();

        for _ in 0..fanout {
            if candidates.is_empty() { break; }
            let idx = self.rng.gen_range(0..candidates.len());
            targets.push(candidates.swap_remove(idx));
        }

        let msg = NetworkMessage::Attestation {
            coord,
            event_id: event_id.to_string(),
            dimension,
            validator_id: validator_id.to_string(),
        };

        for &target in &targets {
            self.network.send(from, target, msg.clone());
        }
    }

    /// Advance one simulation tick:
    /// 1. Deliver pending network messages
    /// 2. Process received attestations (apply + forward)
    /// 3. Tick all nodes (evolve fields)
    pub fn step(&mut self) {
        let delivered = self.network.advance();

        // Process delivered messages
        let mut forwards: Vec<(NodeId, Coord, String, Dimension, String)> = Vec::new();

        for msg in delivered {
            if let NetworkMessage::Attestation { coord, event_id, dimension, validator_id } = msg.payload {
                if let Some(node) = self.nodes.get_mut(&msg.to) {
                    if node.apply_attestation(coord, &event_id, dimension, &validator_id) {
                        self.attestations_applied += 1;
                        // Forward to more peers
                        forwards.push((msg.to, coord, event_id, dimension, validator_id));
                    } else {
                        self.duplicates_rejected += 1;
                    }
                }
            }
        }

        // Process forwards (after borrow is released)
        for (from, coord, event_id, dimension, validator_id) in forwards {
            self.gossip_forward(from, coord, &event_id, dimension, &validator_id);
        }

        // Tick all nodes
        for node in self.nodes.values_mut() {
            node.tick();
        }
    }

    /// Run simulation for N ticks.
    pub fn run(&mut self, ticks: u64) {
        for _ in 0..ticks {
            self.step();
        }
    }

    /// Originate a fully-attested event (4 dimensions) at a node.
    pub fn originate_full_event(&mut self, node: NodeId, coord: Coord, event_id: &str) {
        for (dim, vid) in [
            (Dimension::Temporal, "val_t"),
            (Dimension::Context, "val_c"),
            (Dimension::Origin, "val_o"),
            (Dimension::Verification, "val_v"),
        ] {
            self.originate_attestation(node, coord, event_id, dim, vid);
        }
    }

    /// Check how many nodes have crystallized a specific coordinate.
    pub fn crystallized_at(&self, coord: Coord) -> usize {
        self.nodes.values()
            .filter(|n| n.field.get(coord).crystallized)
            .count()
    }

    /// Check if ALL nodes agree on crystallization at a coordinate.
    pub fn consensus_at(&self, coord: Coord) -> bool {
        let first = self.nodes.values().next()
            .map(|n| n.field.get(coord).crystallized);
        self.nodes.values().all(|n| Some(n.field.get(coord).crystallized) == first)
    }

    /// Fraction of nodes that have crystallized at a coordinate.
    pub fn crystallization_ratio(&self, coord: Coord) -> f64 {
        let total = self.nodes.len();
        if total == 0 { return 0.0; }
        self.crystallized_at(coord) as f64 / total as f64
    }

    /// Check for false crystallizations: cells crystallized at any node
    /// that are NOT crystallized at the origin node.
    pub fn false_crystallizations(&self, coord: Coord, origin_node: NodeId) -> usize {
        let origin_crystallized = self.nodes.get(&origin_node)
            .map(|n| n.field.get(coord).crystallized)
            .unwrap_or(false);

        if origin_crystallized {
            return 0; // origin has it → others having it is correct
        }

        // Origin doesn't have it crystallized → count others that do
        self.nodes.values()
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
            GossipConfig { fanout: 3, field_size: 6, seed: 42 },
            NetworkConfig::default(),
        );

        let center = coord(3, 3, 3, 3);
        sim.originate_full_event(0, center, "test_event");

        // Run enough ticks for gossip to propagate
        sim.run(30);

        let ratio = sim.crystallization_ratio(center);
        assert!(
            ratio > 0.8,
            "gossip should reach most nodes: {:.0}% crystallized",
            ratio * 100.0
        );
    }

    #[test]
    fn dedup_prevents_redundant_processing() {
        let mut sim = DistributedSim::new(
            5,
            GossipConfig { fanout: 4, field_size: 6, seed: 42 },
            NetworkConfig { dup_rate: 0.5, ..NetworkConfig::default() },
        );

        sim.originate_full_event(0, coord(3, 3, 3, 3), "event");
        sim.run(20);

        assert!(
            sim.duplicates_rejected > 0,
            "should reject some duplicates"
        );
    }

    #[test]
    fn lossy_network_still_converges() {
        let mut sim = DistributedSim::new(
            10,
            GossipConfig { fanout: 4, field_size: 6, seed: 42 },
            NetworkConfig::lossy(),
        );

        let center = coord(3, 3, 3, 3);
        sim.originate_full_event(0, center, "lossy_event");

        // More ticks needed for lossy network
        sim.run(60);

        let ratio = sim.crystallization_ratio(center);
        assert!(
            ratio > 0.5,
            "lossy network should still converge: {:.0}%", ratio * 100.0
        );
    }

    #[test]
    fn partition_prevents_propagation_then_heals() {
        let mut sim = DistributedSim::new(
            6,
            GossipConfig { fanout: 3, field_size: 6, seed: 42 },
            NetworkConfig::default(),
        );

        // Partition: nodes 4,5 isolated
        let all: Vec<NodeId> = (0..6).collect();
        sim.network.isolate(4, &all);
        sim.network.isolate(5, &all);

        let center = coord(3, 3, 3, 3);
        sim.originate_full_event(0, center, "part_event");
        sim.run(20);

        // Nodes 4,5 should NOT have the event
        assert!(
            !sim.nodes[&4].field.get(center).crystallized,
            "isolated node 4 should not crystallize"
        );

        // Heal partition
        sim.network.reconnect(4, &all);
        sim.network.reconnect(5, &all);

        // Re-originate to trigger gossip (or send sync)
        sim.originate_full_event(0, center, "part_event");
        sim.run(30);

        // Now they should have it
        let ratio = sim.crystallization_ratio(center);
        assert!(
            ratio > 0.8,
            "after heal, should converge: {:.0}%", ratio * 100.0
        );
    }

    #[test]
    fn no_false_crystallizations() {
        let mut sim = DistributedSim::new(
            10,
            GossipConfig { fanout: 3, field_size: 6, seed: 42 },
            NetworkConfig::lossy(),
        );

        let center = coord(3, 3, 3, 3);
        sim.originate_full_event(0, center, "safe_event");
        sim.run(40);

        // Check a coord where NO event was originated
        let empty = coord(1, 1, 1, 1);
        let false_c = sim.false_crystallizations(empty, 0);
        assert_eq!(
            false_c, 0,
            "no node should crystallize where no event was originated"
        );
    }

    #[test]
    fn different_evolve_speeds() {
        let mut sim = DistributedSim::new(
            5,
            GossipConfig { fanout: 3, field_size: 6, seed: 42 },
            NetworkConfig::default(),
        );

        // Node 0: fast (evolve every tick), Node 4: slow (every 5 ticks)
        sim.nodes.get_mut(&0).unwrap().evolve_interval = 1;
        sim.nodes.get_mut(&4).unwrap().evolve_interval = 5;

        let center = coord(3, 3, 3, 3);
        sim.originate_full_event(0, center, "speed_event");
        sim.run(30);

        // Both should eventually crystallize (attestation applied on receipt)
        assert!(sim.nodes[&0].field.get(center).crystallized, "fast node");
        assert!(sim.nodes[&4].field.get(center).crystallized, "slow node");
    }
}
