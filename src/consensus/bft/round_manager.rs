//! Round manager — orchestrates consecutive BFT rounds with leader rotation
//! and timeout-based liveness.
//!
//! Sits above [`BftRound`] and handles:
//! - Round-robin leader election from the validator set
//! - Timeout detection and view change (advance to next round with new leader)
//! - Tracking the highest committed QC for chain continuity

use super::quorum::SignatureVerifier;
use super::round::{BftRound, RoundAction, RoundEvent, RoundState};
use super::types::QuorumCertificate;

/// Configuration for the round manager.
#[derive(Debug, Clone)]
pub struct RoundManagerConfig {
    /// Base timeout in milliseconds for a single round.
    /// Doubles on each consecutive timeout (exponential backoff).
    pub base_timeout_ms: u64,
    /// Maximum timeout in milliseconds (backoff cap).
    pub max_timeout_ms: u64,
}

impl Default for RoundManagerConfig {
    fn default() -> Self {
        Self {
            base_timeout_ms: 3000,
            max_timeout_ms: 30_000,
        }
    }
}

/// Actions emitted by the round manager to the network layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManagerAction {
    /// A round action from the current BftRound.
    Round(RoundAction),
    /// A new round started — caller should reset their timeout timer.
    NewRound {
        round: u64,
        leader_id: String,
        timeout_ms: u64,
    },
    /// No action.
    None,
}

/// Manages consecutive BFT rounds with leader rotation and liveness timeouts.
pub struct RoundManager<V: SignatureVerifier + Clone> {
    node_id: String,
    validators: Vec<String>,
    verifier: V,
    config: RoundManagerConfig,
    /// Current round number.
    current_round: u64,
    /// The active round state machine.
    current: Option<BftRound<V>>,
    /// Number of consecutive timeouts (for exponential backoff).
    consecutive_timeouts: u32,
    /// Highest committed QC seen so far.
    highest_commit_qc: Option<QuorumCertificate>,
}

impl<V: SignatureVerifier + Clone> RoundManager<V> {
    /// Create a new round manager.
    pub fn new(
        node_id: String,
        validators: Vec<String>,
        verifier: V,
        config: RoundManagerConfig,
    ) -> Self {
        Self {
            node_id,
            validators,
            verifier,
            config,
            current_round: 0,
            current: None,
            consecutive_timeouts: 0,
            highest_commit_qc: None,
        }
    }

    /// Current round number.
    pub fn current_round(&self) -> u64 {
        self.current_round
    }

    /// The leader for a given round (round-robin over validators).
    pub fn leader_for_round(&self, round: u64) -> &str {
        if self.validators.is_empty() {
            return "";
        }
        let idx = (round as usize) % self.validators.len();
        &self.validators[idx]
    }

    /// The leader for the current round.
    pub fn current_leader(&self) -> &str {
        self.leader_for_round(self.current_round)
    }

    /// Whether this node is leader for the current round.
    pub fn is_current_leader(&self) -> bool {
        self.current_leader() == self.node_id
    }

    /// Current timeout in ms (with exponential backoff).
    pub fn current_timeout_ms(&self) -> u64 {
        let timeout = self.config.base_timeout_ms
            * 2u64.saturating_pow(self.consecutive_timeouts);
        timeout.min(self.config.max_timeout_ms)
    }

    /// The highest committed QC.
    pub fn highest_commit_qc(&self) -> Option<&QuorumCertificate> {
        self.highest_commit_qc.as_ref()
    }

    /// State of the current round (if active).
    pub fn round_state(&self) -> Option<RoundState> {
        self.current.as_ref().map(|r| r.state())
    }

    /// Start or advance to a specific round.
    ///
    /// Creates a new `BftRound`, selects the leader via round-robin,
    /// and returns a `NewRound` action so the caller can set their timer.
    /// If this node is the leader, also returns a `StartAsLeader` prompt.
    pub fn start_round(&mut self, round: u64) -> ManagerAction {
        self.current_round = round;
        let leader = self.leader_for_round(round).to_string();

        let bft_round = BftRound::new(
            round,
            self.node_id.clone(),
            leader.clone(),
            self.validators.clone(),
            self.verifier.clone(),
        );
        self.current = Some(bft_round);

        ManagerAction::NewRound {
            round,
            leader_id: leader,
            timeout_ms: self.current_timeout_ms(),
        }
    }

    /// Start round 0 (convenience for initialization).
    pub fn start(&mut self) -> ManagerAction {
        self.start_round(0)
    }

    /// Feed an event to the current round. Returns the resulting action.
    pub fn process_event(&mut self, event: RoundEvent) -> ManagerAction {
        let round = match self.current.as_mut() {
            Some(r) => r,
            None => return ManagerAction::None,
        };

        let action = round.process(event);

        // If a Decide action, update highest QC and reset timeout backoff.
        if let RoundAction::Decide { ref commit_qc, .. } = action {
            self.highest_commit_qc = Some(commit_qc.clone());
            self.consecutive_timeouts = 0;
        }

        ManagerAction::Round(action)
    }

    /// Handle a timeout for the current round.
    ///
    /// Increments the backoff counter and advances to the next round with
    /// a new leader. Returns `NewRound` so the caller can reset their timer.
    pub fn on_timeout(&mut self) -> ManagerAction {
        // Notify the current round of the timeout.
        if let Some(ref mut r) = self.current {
            r.process(RoundEvent::Timeout);
        }

        self.consecutive_timeouts += 1;
        let next_round = self.current_round + 1;
        self.start_round(next_round)
    }

    /// Advance to the next round after a successful Decide.
    ///
    /// Resets timeout backoff since progress was made.
    pub fn advance_after_decide(&mut self) -> ManagerAction {
        self.consecutive_timeouts = 0;
        let next_round = self.current_round + 1;
        self.start_round(next_round)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::bft::quorum::AcceptAllVerifier;
    use crate::consensus::bft::round::RoundAction;
    use crate::consensus::bft::types::{BftPhase, VoteMessage};

    fn validators() -> Vec<String> {
        (0..4).map(|i| format!("v{i}")).collect()
    }

    fn manager(node: &str) -> RoundManager<AcceptAllVerifier> {
        RoundManager::new(
            node.into(),
            validators(),
            AcceptAllVerifier,
            RoundManagerConfig::default(),
        )
    }

    fn block_hash(id: u8) -> [u8; 32] {
        let mut h = [0u8; 32];
        h[0] = id;
        h
    }

    fn make_vote(phase: BftPhase, hash_id: u8, round: u64, voter: &str) -> VoteMessage {
        VoteMessage {
            block_hash: block_hash(hash_id),
            round,
            phase,
            voter_id: voter.to_string(),
            signature: vec![1u8; 64],
        }
    }

    // --- leader rotation ---

    #[test]
    fn leader_rotates_round_robin() {
        let m = manager("v0");
        assert_eq!(m.leader_for_round(0), "v0");
        assert_eq!(m.leader_for_round(1), "v1");
        assert_eq!(m.leader_for_round(2), "v2");
        assert_eq!(m.leader_for_round(3), "v3");
        assert_eq!(m.leader_for_round(4), "v0"); // wraps
    }

    #[test]
    fn start_returns_new_round_action() {
        let mut m = manager("v0");
        let action = m.start();
        match action {
            ManagerAction::NewRound {
                round,
                leader_id,
                timeout_ms,
            } => {
                assert_eq!(round, 0);
                assert_eq!(leader_id, "v0");
                assert_eq!(timeout_ms, 3000);
            }
            other => panic!("expected NewRound, got {other:?}"),
        }
        assert_eq!(m.current_round(), 0);
    }

    #[test]
    fn is_current_leader_when_round_matches() {
        let mut m = manager("v0");
        m.start_round(0);
        assert!(m.is_current_leader());

        m.start_round(1);
        assert!(!m.is_current_leader()); // v1 is leader for round 1
    }

    // --- timeout & backoff ---

    #[test]
    fn timeout_advances_round() {
        let mut m = manager("v0");
        m.start();
        let action = m.on_timeout();
        match action {
            ManagerAction::NewRound { round, leader_id, .. } => {
                assert_eq!(round, 1);
                assert_eq!(leader_id, "v1");
            }
            other => panic!("expected NewRound, got {other:?}"),
        }
        assert_eq!(m.current_round(), 1);
    }

    #[test]
    fn timeout_backoff_doubles() {
        let mut m = manager("v0");
        m.start();
        assert_eq!(m.current_timeout_ms(), 3000);

        m.on_timeout(); // consecutive=1
        assert_eq!(m.current_timeout_ms(), 6000);

        m.on_timeout(); // consecutive=2
        assert_eq!(m.current_timeout_ms(), 12000);

        m.on_timeout(); // consecutive=3
        assert_eq!(m.current_timeout_ms(), 24000);

        m.on_timeout(); // consecutive=4 → 48000 capped to 30000
        assert_eq!(m.current_timeout_ms(), 30_000);
    }

    #[test]
    fn backoff_resets_after_decide() {
        let mut m = manager("v0");
        m.start();
        m.on_timeout(); // consecutive=1
        m.on_timeout(); // consecutive=2
        assert_eq!(m.current_timeout_ms(), 12000);

        // Simulate a decide.
        m.advance_after_decide();
        assert_eq!(m.current_timeout_ms(), 3000); // reset
    }

    // --- full flow through manager ---

    #[test]
    fn full_round_via_manager() {
        let mut m = manager("v0");
        m.start();

        // Leader proposes.
        let action = m.process_event(RoundEvent::StartAsLeader {
            block_hash: block_hash(1),
        });
        assert!(matches!(
            action,
            ManagerAction::Round(RoundAction::BroadcastProposal { .. })
        ));

        // Collect votes through all phases.
        for phase in [BftPhase::Prepare, BftPhase::PreCommit, BftPhase::Commit] {
            for voter in &["v0", "v1", "v2"] {
                m.process_event(RoundEvent::Vote(make_vote(phase, 1, 0, voter)));
            }
        }

        assert_eq!(m.round_state(), Some(RoundState::Decided));
        assert!(m.highest_commit_qc().is_some());
        assert_eq!(m.consecutive_timeouts, 0);
    }

    #[test]
    fn follower_receives_proposal_via_manager() {
        let mut m = manager("v1"); // v0 is leader for round 0
        m.start();

        let action = m.process_event(RoundEvent::Proposal {
            block_hash: block_hash(1),
            leader_id: "v0".into(),
        });
        match action {
            ManagerAction::Round(RoundAction::SendVote(vote)) => {
                assert_eq!(vote.voter_id, "v1");
                assert_eq!(vote.phase, BftPhase::Prepare);
            }
            other => panic!("expected SendVote, got {other:?}"),
        }
    }

    #[test]
    fn events_before_start_are_noop() {
        let mut m = manager("v0");
        let action = m.process_event(RoundEvent::Vote(make_vote(
            BftPhase::Prepare, 1, 0, "v0",
        )));
        assert_eq!(action, ManagerAction::None);
    }

    // --- multiple rounds ---

    #[test]
    fn three_consecutive_rounds_rotate_leaders() {
        let mut m = manager("v0");

        for expected_round in 0..3u64 {
            let action = m.start_round(expected_round);
            match action {
                ManagerAction::NewRound { round, leader_id, .. } => {
                    assert_eq!(round, expected_round);
                    let expected_leader = format!("v{}", expected_round % 4);
                    assert_eq!(leader_id, expected_leader);
                }
                other => panic!("expected NewRound, got {other:?}"),
            }
        }
    }
}
