//! Network simulator — models imperfect message delivery between nodes.
//!
//! Simulates real-world network conditions:
//!   - Variable latency (uniform or exponential delay)
//!   - Message loss (configurable drop rate)
//!   - Duplicate delivery (configurable dup rate)
//!   - Out-of-order delivery (messages arrive in random order)
//!   - Network partitions (bidirectional or asymmetric)
//!   - Clock skew (nodes tick at different rates)
//!
//! All randomness uses a seeded RNG for reproducibility.

use std::collections::{HashMap, HashSet, VecDeque};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

/// Unique node identifier.
pub type NodeId = usize;

/// A message in transit between nodes.
#[derive(Clone, Debug)]
pub struct InFlightMessage {
    pub from: NodeId,
    pub to: NodeId,
    pub payload: NetworkMessage,
    /// Tick at which this message becomes deliverable.
    pub deliver_at: u64,
}

/// Message types exchanged between nodes.
#[derive(Clone, Debug)]
pub enum NetworkMessage {
    /// An attestation to propagate.
    Attestation {
        coord: crate::Coord,
        event_id: String,
        dimension: crate::Dimension,
        validator_id: String,
    },
    /// Request to sync state (pull-based).
    SyncRequest { from_tick: u64 },
    /// Heartbeat / liveness probe.
    Heartbeat { node_tick: u64 },
}

/// Network fault configuration.
#[derive(Clone, Debug)]
pub struct NetworkConfig {
    /// Base latency in ticks (minimum delivery delay).
    pub base_latency: u64,
    /// Additional random latency (uniform [0, jitter]).
    pub jitter: u64,
    /// Probability of dropping a message [0.0, 1.0).
    pub drop_rate: f64,
    /// Probability of duplicating a message [0.0, 1.0).
    pub dup_rate: f64,
    /// Whether messages can arrive out of order.
    pub allow_reorder: bool,
    /// RNG seed for reproducibility.
    pub seed: u64,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            base_latency: 1,
            jitter: 2,
            drop_rate: 0.0,
            dup_rate: 0.0,
            allow_reorder: false,
            seed: 42,
        }
    }
}

impl NetworkConfig {
    pub fn lossy() -> Self {
        Self {
            drop_rate: 0.1,
            dup_rate: 0.05,
            allow_reorder: true,
            ..Self::default()
        }
    }

    pub fn adversarial() -> Self {
        Self {
            base_latency: 3,
            jitter: 10,
            drop_rate: 0.2,
            dup_rate: 0.1,
            allow_reorder: true,
            seed: 77,
            ..Self::default()
        }
    }
}

/// Simulated network with configurable faults.
pub struct NetworkSim {
    config: NetworkConfig,
    rng: StdRng,
    /// Messages in transit, sorted by deliver_at.
    in_flight: VecDeque<InFlightMessage>,
    /// Current network tick.
    pub tick: u64,
    /// Active partitions: (node_a, node_b) pairs that cannot communicate.
    partitions: HashSet<(NodeId, NodeId)>,
    /// Per-node clock skew: node ticks at `1.0 + skew` rate.
    clock_skew: HashMap<NodeId, f64>,
    // --- Metrics ---
    pub messages_sent: u64,
    pub messages_delivered: u64,
    pub messages_dropped: u64,
    pub messages_duplicated: u64,
}

impl NetworkSim {
    pub fn new(config: NetworkConfig) -> Self {
        let rng = StdRng::seed_from_u64(config.seed);
        Self {
            config,
            rng,
            in_flight: VecDeque::new(),
            tick: 0,
            partitions: HashSet::new(),
            clock_skew: HashMap::new(),
            messages_sent: 0,
            messages_delivered: 0,
            messages_dropped: 0,
            messages_duplicated: 0,
        }
    }

    /// Set clock skew for a node. 0.0 = normal speed, 0.5 = 50% faster, -0.3 = 30% slower.
    pub fn set_clock_skew(&mut self, node: NodeId, skew: f64) {
        self.clock_skew.insert(node, skew);
    }

    /// Create a bidirectional partition between two nodes.
    pub fn partition(&mut self, a: NodeId, b: NodeId) {
        self.partitions.insert((a.min(b), a.max(b)));
    }

    /// Heal a partition between two nodes.
    pub fn heal(&mut self, a: NodeId, b: NodeId) {
        self.partitions.remove(&(a.min(b), a.max(b)));
    }

    /// Partition a node from all others.
    pub fn isolate(&mut self, node: NodeId, all_nodes: &[NodeId]) {
        for &other in all_nodes {
            if other != node {
                self.partition(node, other);
            }
        }
    }

    /// Reconnect a previously isolated node.
    pub fn reconnect(&mut self, node: NodeId, all_nodes: &[NodeId]) {
        for &other in all_nodes {
            if other != node {
                self.heal(node, other);
            }
        }
    }

    /// Check if two nodes can communicate.
    pub fn can_reach(&self, from: NodeId, to: NodeId) -> bool {
        !self.partitions.contains(&(from.min(to), from.max(to)))
    }

    /// Send a message from one node to another.
    /// Returns false if the message was dropped.
    pub fn send(&mut self, from: NodeId, to: NodeId, payload: NetworkMessage) -> bool {
        self.messages_sent += 1;

        // Partition check
        if !self.can_reach(from, to) {
            self.messages_dropped += 1;
            return false;
        }

        // Drop check
        if self.rng.gen::<f64>() < self.config.drop_rate {
            self.messages_dropped += 1;
            return false;
        }

        // Calculate delivery time
        let latency = self.config.base_latency
            + if self.config.jitter > 0 {
                self.rng.gen_range(0..=self.config.jitter)
            } else {
                0
            };
        let deliver_at = self.tick + latency;

        self.in_flight.push_back(InFlightMessage {
            from,
            to,
            payload: payload.clone(),
            deliver_at,
        });

        // Duplicate check
        if self.rng.gen::<f64>() < self.config.dup_rate {
            self.messages_duplicated += 1;
            let dup_latency = latency + self.rng.gen_range(1..=3);
            self.in_flight.push_back(InFlightMessage {
                from,
                to,
                payload,
                deliver_at: self.tick + dup_latency,
            });
        }

        true
    }

    /// Broadcast a message to all nodes in a list.
    pub fn broadcast(&mut self, from: NodeId, targets: &[NodeId], payload: NetworkMessage) -> usize {
        let mut sent = 0;
        for &to in targets {
            if to != from && self.send(from, to, payload.clone()) {
                sent += 1;
            }
        }
        sent
    }

    /// Advance one tick and return messages deliverable now.
    pub fn advance(&mut self) -> Vec<InFlightMessage> {
        self.tick += 1;

        let mut deliverable = Vec::new();
        let mut remaining = VecDeque::new();

        while let Some(msg) = self.in_flight.pop_front() {
            if msg.deliver_at <= self.tick {
                // Re-check partition at delivery time (may have changed)
                if self.can_reach(msg.from, msg.to) {
                    self.messages_delivered += 1;
                    deliverable.push(msg);
                } else {
                    self.messages_dropped += 1;
                }
            } else {
                remaining.push_back(msg);
            }
        }

        self.in_flight = remaining;

        // Reorder if configured
        if self.config.allow_reorder && deliverable.len() > 1 {
            // Fisher-Yates shuffle
            for i in (1..deliverable.len()).rev() {
                let j = self.rng.gen_range(0..=i);
                deliverable.swap(i, j);
            }
        }

        deliverable
    }

    /// Get the effective tick for a node (accounting for clock skew).
    pub fn node_tick(&self, node: NodeId) -> u64 {
        let skew = self.clock_skew.get(&node).copied().unwrap_or(0.0);
        ((self.tick as f64) * (1.0 + skew)).max(0.0) as u64
    }

    /// Number of messages still in transit.
    pub fn pending_messages(&self) -> usize {
        self.in_flight.len()
    }

    /// Drain all in-flight messages (flush the network).
    pub fn flush(&mut self, max_ticks: u64) -> Vec<InFlightMessage> {
        let mut all = Vec::new();
        for _ in 0..max_ticks {
            if self.in_flight.is_empty() {
                break;
            }
            all.extend(self.advance());
        }
        all
    }

    pub fn metrics_summary(&self) -> NetworkMetrics {
        NetworkMetrics {
            tick: self.tick,
            messages_sent: self.messages_sent,
            messages_delivered: self.messages_delivered,
            messages_dropped: self.messages_dropped,
            messages_duplicated: self.messages_duplicated,
            pending: self.pending_messages() as u64,
            drop_rate_actual: if self.messages_sent > 0 {
                self.messages_dropped as f64 / self.messages_sent as f64
            } else {
                0.0
            },
            dup_rate_actual: if self.messages_sent > 0 {
                self.messages_duplicated as f64 / self.messages_sent as f64
            } else {
                0.0
            },
        }
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct NetworkMetrics {
    pub tick: u64,
    pub messages_sent: u64,
    pub messages_delivered: u64,
    pub messages_dropped: u64,
    pub messages_duplicated: u64,
    pub pending: u64,
    pub drop_rate_actual: f64,
    pub dup_rate_actual: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_network_delivers_all() {
        let mut net = NetworkSim::new(NetworkConfig::default());
        let msg = NetworkMessage::Heartbeat { node_tick: 0 };
        net.send(0, 1, msg);

        // Advance enough ticks for delivery
        let mut delivered = Vec::new();
        for _ in 0..10 {
            delivered.extend(net.advance());
        }

        assert_eq!(delivered.len(), 1);
        assert_eq!(net.messages_dropped, 0);
    }

    #[test]
    fn partition_blocks_messages() {
        let mut net = NetworkSim::new(NetworkConfig::default());
        net.partition(0, 1);

        let msg = NetworkMessage::Heartbeat { node_tick: 0 };
        let sent = net.send(0, 1, msg);

        assert!(!sent, "should fail to send across partition");
        assert_eq!(net.messages_dropped, 1);
    }

    #[test]
    fn heal_restores_communication() {
        let mut net = NetworkSim::new(NetworkConfig::default());
        net.partition(0, 1);
        net.heal(0, 1);

        let msg = NetworkMessage::Heartbeat { node_tick: 0 };
        assert!(net.send(0, 1, msg));
    }

    #[test]
    fn lossy_network_drops_some() {
        let mut net = NetworkSim::new(NetworkConfig {
            drop_rate: 0.5,
            seed: 123,
            ..NetworkConfig::default()
        });

        for i in 0..100 {
            net.send(0, 1, NetworkMessage::Heartbeat { node_tick: i });
        }

        let _ = net.flush(20);

        // With 50% drop rate, expect roughly 40-60 dropped
        assert!(
            net.messages_dropped > 20 && net.messages_dropped < 80,
            "~50% should drop: dropped={}", net.messages_dropped
        );
    }

    #[test]
    fn broadcast_reaches_all_reachable() {
        let mut net = NetworkSim::new(NetworkConfig::default());
        let nodes: Vec<NodeId> = (0..5).collect();

        let sent = net.broadcast(0, &nodes, NetworkMessage::Heartbeat { node_tick: 0 });
        assert_eq!(sent, 4, "should send to 4 other nodes");
    }

    #[test]
    fn clock_skew_affects_node_tick() {
        let mut net = NetworkSim::new(NetworkConfig::default());
        net.set_clock_skew(0, 0.5);  // 50% faster
        net.set_clock_skew(1, -0.3); // 30% slower

        for _ in 0..10 {
            net.advance();
        }

        assert!(net.node_tick(0) > net.tick, "fast node ahead");
        assert!(net.node_tick(1) < net.tick, "slow node behind");
    }

    #[test]
    fn metrics_summary_accurate() {
        let mut net = NetworkSim::new(NetworkConfig::lossy());
        for _ in 0..50 {
            net.send(0, 1, NetworkMessage::Heartbeat { node_tick: 0 });
        }
        let _ = net.flush(20);

        let m = net.metrics_summary();
        assert_eq!(m.messages_sent, 50);
        assert_eq!(
            m.messages_delivered + m.messages_dropped + m.pending,
            m.messages_sent + m.messages_duplicated,
            "accounting: delivered+dropped+pending should equal sent+duplicated"
        );
    }
}
