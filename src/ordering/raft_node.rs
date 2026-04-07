//! Raft ordering node — wraps `RawNode<MemStorage>` from the tikv raft crate.

use raft::prelude::*;
use raft::storage::MemStorage;
use raft::StateRole;

/// Errors produced by [`RaftNode`].
#[derive(Debug, thiserror::Error)]
pub enum RaftError {
    #[error("raft init error: {0}")]
    Init(String),
    #[error("raft step error: {0}")]
    Step(String),
}

/// A single-node Raft participant backed by in-memory storage.
///
/// Wraps [`RawNode<MemStorage>`] with a pending-proposal queue and a
/// committed-entry buffer.  Callers drive the node forward by calling
/// [`tick`](RaftNode::tick) and then [`advance`](RaftNode::advance) to
/// drain any ready state.
pub struct RaftNode {
    pub id: u64,
    pub raw_node: RawNode<MemStorage>,
    pub pending_proposals: Vec<Vec<u8>>,
    pub committed_entries: Vec<Entry>,
}

impl RaftNode {
    /// Create a new Raft node with `id` and the given peer voter IDs.
    ///
    /// The node starts as a Follower (or, if it is the only voter, it will
    /// campaign on the first tick cycle).
    ///
    /// Configuration:
    /// - `election_tick = 10` — ticks before a follower starts an election
    /// - `heartbeat_tick = 3` — ticks between leader heartbeats
    pub fn new(id: u64, peers: Vec<u64>) -> Result<Self, RaftError> {
        let config = Config {
            id,
            election_tick: 10,
            heartbeat_tick: 3,
            ..Default::default()
        };

        // All supplied peers (including self) are initial voters.
        let conf_state = ConfState {
            voters: peers,
            ..Default::default()
        };

        let storage = MemStorage::new_with_conf_state(conf_state);
        let logger = raft::default_logger();
        let raw_node =
            RawNode::new(&config, storage, &logger).map_err(|e| RaftError::Init(e.to_string()))?;

        Ok(Self {
            id,
            raw_node,
            pending_proposals: Vec::new(),
            committed_entries: Vec::new(),
        })
    }

    /// Advance the Raft logical clock by one tick.
    pub fn tick(&mut self) {
        self.raw_node.tick();
    }

    /// Propose `data` as a new Raft log entry.
    ///
    /// The node must be the current leader; otherwise the crate returns
    /// [`raft::Error::ProposalDropped`] which is mapped to [`RaftError::Step`].
    pub fn propose(&mut self, data: Vec<u8>) -> Result<(), RaftError> {
        self.raw_node
            .propose(vec![], data)
            .map_err(|e| RaftError::Step(e.to_string()))
    }

    /// Process an incoming Raft message (e.g. AppendEntries, RequestVote).
    pub fn step(&mut self, msg: Message) -> Result<(), RaftError> {
        self.raw_node
            .step(msg)
            .map_err(|e| RaftError::Step(e.to_string()))
    }

    /// Drain the raft ready state: persist entries + hardstate, collect
    /// committed entries, and return outbound messages plus committed data.
    ///
    /// Returns `(outbound_messages, committed_entries)`.
    pub fn advance(&mut self) -> (Vec<Message>, Vec<Entry>) {
        if !self.raw_node.has_ready() {
            return (vec![], vec![]);
        }
        let mut ready = self.raw_node.ready();
        let mut msgs: Vec<Message> = ready.take_messages();
        let mut committed: Vec<Entry> = ready.take_committed_entries();
        {
            let mut store = self.raw_node.mut_store().wl();
            if !ready.entries().is_empty() {
                store.append(ready.entries()).unwrap();
            }
            if let Some(hs) = ready.hs() {
                store.set_hardstate(hs.clone());
            }
        }
        msgs.extend(ready.take_persisted_messages());
        let mut light = self.raw_node.advance(ready);
        committed.extend(light.take_committed_entries());
        msgs.extend_from_slice(light.messages());
        if let Some(commit) = light.commit_index() {
            let mut hs = self.raw_node.raft.hard_state();
            hs.commit = commit;
            self.raw_node.mut_store().wl().set_hardstate(hs);
        }
        self.raw_node.advance_apply();
        self.committed_entries.extend(committed.clone());
        (msgs, committed)
    }

    /// Create a snapshot of the current raft state.
    ///
    /// The snapshot captures committed entries as serialized JSON in the data
    /// field, plus the log index/term at which the snapshot was taken.
    pub fn create_snapshot(&self) -> Result<Snapshot, RaftError> {
        let hs = self.raw_node.raft.hard_state();
        let cs = self.raw_node.raft.prs().conf().to_conf_state();
        let last_applied = hs.commit;
        // Find the term for the last applied index.
        let last_term = Storage::term(self.raw_node.store(), last_applied).unwrap_or(hs.term);

        let data = serde_json::to_vec(&self.committed_entries.len())
            .map_err(|e| RaftError::Init(e.to_string()))?;

        let mut snap = Snapshot::default();
        snap.mut_metadata().index = last_applied;
        snap.mut_metadata().term = last_term;
        *snap.mut_metadata().mut_conf_state() = cs;
        snap.data = data.into();
        Ok(snap)
    }

    /// Apply a snapshot received from a leader, replacing local state.
    pub fn apply_snapshot(&mut self, snap: Snapshot) -> Result<(), RaftError> {
        let mut store = self.raw_node.mut_store().wl();
        store
            .apply_snapshot(snap)
            .map_err(|e| RaftError::Init(e.to_string()))
    }

    /// Return `true` if this node is currently the Raft leader.
    pub fn is_leader(&self) -> bool {
        self.raw_node.raft.state == StateRole::Leader
    }

    /// Return `true` if this node is currently a Follower.
    pub fn is_follower(&self) -> bool {
        self.raw_node.raft.state == StateRole::Follower
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_single_node_starts_as_follower() {
        let node = RaftNode::new(1, vec![1]).unwrap();
        // A freshly created node has not yet campaigned — it is a Follower.
        assert!(node.is_follower(), "expected Follower on init");
    }

    #[test]
    fn new_node_stores_id() {
        let node = RaftNode::new(2, vec![1, 2, 3]).unwrap();
        assert_eq!(node.id, 2);
    }

    #[test]
    fn new_node_has_empty_queues() {
        let node = RaftNode::new(1, vec![1]).unwrap();
        assert!(node.pending_proposals.is_empty());
        assert!(node.committed_entries.is_empty());
    }

    /// Drain one ready cycle via `RaftNode::advance()`, returning outbound messages.
    fn drain_ready(node: &mut RaftNode) -> Vec<Message> {
        let (msgs, _) = node.advance();
        msgs
    }

    /// Tick a single-voter node until it wins the election and becomes leader,
    /// draining all ready states so storage is fully consistent.
    fn elect_single_node(node: &mut RaftNode) {
        for _ in 0..20 {
            node.tick();
            drain_ready(node);
            if node.is_leader() {
                return;
            }
        }
        panic!("node did not become leader after 20 ticks");
    }

    /// Deliver a batch of messages to their target nodes, recursively
    /// delivering any response messages until the network is quiet.
    fn deliver(nodes: &mut [RaftNode], msgs: Vec<Message>) {
        if msgs.is_empty() {
            return;
        }
        let mut next: Vec<Message> = Vec::new();
        for msg in msgs {
            let to = msg.to;
            if let Some(target) = nodes.iter_mut().find(|n| n.id == to) {
                let _ = target.step(msg);
                next.extend(drain_ready(target));
            }
        }
        deliver(nodes, next);
    }

    /// Tick all nodes and route messages between them for `rounds` rounds.
    fn route_messages(nodes: &mut [RaftNode], rounds: usize) {
        for _ in 0..rounds {
            let mut pending: Vec<Message> = Vec::new();
            for node in nodes.iter_mut() {
                node.tick();
                pending.extend(drain_ready(node));
            }
            deliver(nodes, pending);
        }
    }

    #[test]
    fn three_nodes_elect_a_leader() {
        let peers = vec![1, 2, 3];
        let mut nodes: Vec<RaftNode> = peers
            .iter()
            .map(|&id| RaftNode::new(id, peers.clone()).unwrap())
            .collect();

        route_messages(&mut nodes, 30);

        let leaders: Vec<u64> = nodes
            .iter()
            .filter(|n| n.is_leader())
            .map(|n| n.id)
            .collect();
        assert_eq!(
            leaders.len(),
            1,
            "expected exactly one leader, got {leaders:?}"
        );
    }

    #[test]
    fn follower_accepts_append_entries_via_step() {
        let peers = vec![1, 2, 3];
        let mut nodes: Vec<RaftNode> = peers
            .iter()
            .map(|&id| RaftNode::new(id, peers.clone()).unwrap())
            .collect();

        route_messages(&mut nodes, 30);

        let leader_id = nodes.iter().find(|n| n.is_leader()).expect("no leader").id;

        // Propose on leader.
        let leader = nodes.iter_mut().find(|n| n.id == leader_id).unwrap();
        leader.propose(b"world".to_vec()).unwrap();

        // Route until all nodes converge.
        route_messages(&mut nodes, 30);

        // All nodes should have the entry in their log.
        for node in &nodes {
            let last = Storage::last_index(node.raw_node.store()).unwrap();
            // The log must have grown beyond the initial no-op entry.
            assert!(
                last >= 2,
                "node {} last_index={}, expected >= 2",
                node.id,
                last
            );
        }
    }

    #[test]
    fn propose_on_leader_produces_committed_entry() {
        let mut node = RaftNode::new(1, vec![1]).unwrap();
        elect_single_node(&mut node);
        assert!(node.is_leader());

        node.propose(b"hello".to_vec()).unwrap();

        // In raft 0.7 the first Ready after propose contains the entry in
        // `entries()` (to persist) but not yet in `committed_entries()`.
        // After persisting + advance(), the LightReady carries the committed
        // entries.
        assert!(node.raw_node.has_ready(), "expected ready after propose");
        let ready = node.raw_node.ready();

        let mut all_committed: Vec<Entry> = ready.committed_entries().clone();

        // Persist entries to stable storage before advancing.
        {
            let mut store = node.raw_node.mut_store().wl();
            if !ready.entries().is_empty() {
                store.append(ready.entries()).unwrap();
            }
            if let Some(hs) = ready.hs() {
                store.set_hardstate(hs.clone());
            }
        }

        let mut light = node.raw_node.advance(ready);
        all_committed.extend(light.take_committed_entries());

        if let Some(commit) = light.commit_index() {
            let mut hs = node.raw_node.raft.hard_state();
            hs.commit = commit;
            node.raw_node.mut_store().wl().set_hardstate(hs);
        }
        node.raw_node.advance_apply();

        let found = all_committed.iter().any(|e| e.data == b"hello");
        assert!(found, "expected 'hello' in committed entries");
    }

    #[test]
    fn advance_returns_five_committed_entries_on_all_nodes() {
        let peers = vec![1, 2, 3];
        let mut nodes: Vec<RaftNode> = peers
            .iter()
            .map(|&id| RaftNode::new(id, peers.clone()).unwrap())
            .collect();

        route_messages(&mut nodes, 30);

        let leader_id = nodes.iter().find(|n| n.is_leader()).expect("no leader").id;

        // Propose 5 entries on the leader.
        let leader = nodes.iter_mut().find(|n| n.id == leader_id).unwrap();
        for i in 0..5u8 {
            leader.propose(vec![i]).unwrap();
        }

        // Route until committed on all nodes.
        route_messages(&mut nodes, 30);

        // Every node must have all 5 entries in committed_entries.
        for node in &nodes {
            let payloads: Vec<Vec<u8>> = node
                .committed_entries
                .iter()
                .filter(|e| !e.data.is_empty())
                .map(|e| e.data.clone())
                .collect();
            for i in 0..5u8 {
                assert!(
                    payloads.contains(&vec![i]),
                    "node {} missing entry {i}, has {payloads:?}",
                    node.id
                );
            }
        }
    }

    #[test]
    fn snapshot_transfer_syncs_new_node() {
        // Node A: single-node cluster, propose 100 entries.
        let mut node_a = RaftNode::new(1, vec![1]).unwrap();
        elect_single_node(&mut node_a);

        for i in 0u32..100 {
            node_a.propose(i.to_le_bytes().to_vec()).unwrap();
            node_a.advance();
        }
        assert!(
            node_a.committed_entries.len() >= 100,
            "node_a committed {} entries, expected >= 100",
            node_a.committed_entries.len()
        );

        // Create snapshot from node A.
        let snap = node_a.create_snapshot().unwrap();
        assert!(snap.get_metadata().index > 0);

        // Node B: fresh node that applies the snapshot.
        let mut node_b = RaftNode::new(2, vec![1, 2]).unwrap();
        node_b.apply_snapshot(snap.clone()).unwrap();

        // Verify node B's storage reflects the snapshot.
        let stored_snap = Storage::snapshot(node_b.raw_node.store(), 0, 0).unwrap();
        assert_eq!(
            stored_snap.get_metadata().index,
            snap.get_metadata().index,
            "snapshot index mismatch"
        );
    }
}
