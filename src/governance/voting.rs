//! Stake-weighted voting for governance proposals.
//!
//! Validators vote with power proportional to their staked amount.
//! After the voting period, votes are tallied against quorum and
//! pass threshold to determine the proposal outcome.

use std::collections::HashMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use super::proposals::ProposalId;

/// A vote cast by a validator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VoteOption {
    Yes,
    No,
    Abstain,
}

/// A single vote record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Vote {
    pub voter: String,
    pub proposal_id: ProposalId,
    pub option: VoteOption,
    /// Voting power (= staked amount at time of vote).
    pub power: u64,
    /// Block height when the vote was cast.
    pub voted_at: u64,
}

/// Result of tallying votes for a proposal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TallyResult {
    pub proposal_id: ProposalId,
    pub yes_power: u64,
    pub no_power: u64,
    pub abstain_power: u64,
    pub total_voted_power: u64,
    pub total_staked_power: u64,
    /// Quorum reached: total_voted_power / total_staked_power >= quorum_percent
    pub quorum_reached: bool,
    /// Proposal passed: yes_power / (yes_power + no_power) >= pass_threshold
    pub passed: bool,
}

/// Voting errors.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VotingError {
    #[error("already voted on proposal {0}")]
    AlreadyVoted(ProposalId),
    #[error("zero voting power")]
    ZeroPower,
    #[error("voting period ended for proposal {0}")]
    VotingEnded(ProposalId),
}

/// Vote store for all proposals.
pub struct VoteStore {
    /// proposal_id → (voter → Vote)
    votes: Mutex<HashMap<ProposalId, HashMap<String, Vote>>>,
}

impl VoteStore {
    pub fn new() -> Self {
        Self {
            votes: Mutex::new(HashMap::new()),
        }
    }

    /// Cast a vote. Rejects duplicates and zero-power votes.
    pub fn cast_vote(
        &self,
        proposal_id: ProposalId,
        voter: &str,
        option: VoteOption,
        power: u64,
        current_height: u64,
        voting_ends_at: u64,
    ) -> Result<(), VotingError> {
        if power == 0 {
            return Err(VotingError::ZeroPower);
        }

        if current_height >= voting_ends_at {
            return Err(VotingError::VotingEnded(proposal_id));
        }

        let mut all = self.votes.lock().unwrap();
        let proposal_votes = all.entry(proposal_id).or_default();

        if proposal_votes.contains_key(voter) {
            return Err(VotingError::AlreadyVoted(proposal_id));
        }

        proposal_votes.insert(
            voter.to_string(),
            Vote {
                voter: voter.to_string(),
                proposal_id,
                option,
                power,
                voted_at: current_height,
            },
        );

        Ok(())
    }

    /// Tally votes for a proposal.
    ///
    /// - `total_staked_power`: sum of all active validators' stakes (for quorum calc)
    /// - `quorum_percent`: minimum % of total stake that must vote (e.g. 33)
    /// - `pass_threshold_percent`: minimum % of (yes+no) that must be yes (e.g. 67)
    pub fn tally(
        &self,
        proposal_id: ProposalId,
        total_staked_power: u64,
        quorum_percent: u64,
        pass_threshold_percent: u64,
    ) -> TallyResult {
        let all = self.votes.lock().unwrap();
        let votes = all.get(&proposal_id);

        let (mut yes, mut no, mut abstain) = (0u64, 0u64, 0u64);

        if let Some(proposal_votes) = votes {
            for vote in proposal_votes.values() {
                match vote.option {
                    VoteOption::Yes => yes += vote.power,
                    VoteOption::No => no += vote.power,
                    VoteOption::Abstain => abstain += vote.power,
                }
            }
        }

        let total_voted = yes + no + abstain;

        let quorum_reached =
            total_staked_power > 0 && (total_voted * 100) / total_staked_power >= quorum_percent;

        let yes_no_total = yes + no;
        let passed = quorum_reached
            && yes_no_total > 0
            && (yes * 100) / yes_no_total >= pass_threshold_percent;

        TallyResult {
            proposal_id,
            yes_power: yes,
            no_power: no,
            abstain_power: abstain,
            total_voted_power: total_voted,
            total_staked_power,
            quorum_reached,
            passed,
        }
    }

    /// Get all votes for a proposal.
    pub fn get_votes(&self, proposal_id: ProposalId) -> Vec<Vote> {
        self.votes
            .lock()
            .unwrap()
            .get(&proposal_id)
            .map(|m| m.values().cloned().collect())
            .unwrap_or_default()
    }

    /// Get a specific voter's vote.
    pub fn get_vote(&self, proposal_id: ProposalId, voter: &str) -> Option<Vote> {
        self.votes
            .lock()
            .unwrap()
            .get(&proposal_id)
            .and_then(|m| m.get(voter).cloned())
    }
}

impl Default for VoteStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> VoteStore {
        VoteStore::new()
    }

    // --- cast_vote ---

    #[test]
    fn cast_vote_ok() {
        let s = store();
        s.cast_vote(1, "alice", VoteOption::Yes, 1000, 50, 100)
            .unwrap();
        let vote = s.get_vote(1, "alice").unwrap();
        assert_eq!(vote.option, VoteOption::Yes);
        assert_eq!(vote.power, 1000);
    }

    #[test]
    fn cast_vote_duplicate_rejected() {
        let s = store();
        s.cast_vote(1, "alice", VoteOption::Yes, 1000, 50, 100)
            .unwrap();
        let err = s
            .cast_vote(1, "alice", VoteOption::No, 1000, 60, 100)
            .unwrap_err();
        assert!(matches!(err, VotingError::AlreadyVoted(1)));
    }

    #[test]
    fn cast_vote_zero_power_rejected() {
        let s = store();
        let err = s
            .cast_vote(1, "alice", VoteOption::Yes, 0, 50, 100)
            .unwrap_err();
        assert!(matches!(err, VotingError::ZeroPower));
    }

    #[test]
    fn cast_vote_after_deadline_rejected() {
        let s = store();
        let err = s
            .cast_vote(1, "alice", VoteOption::Yes, 1000, 100, 100)
            .unwrap_err();
        assert!(matches!(err, VotingError::VotingEnded(1)));
    }

    #[test]
    fn different_proposals_independent() {
        let s = store();
        s.cast_vote(1, "alice", VoteOption::Yes, 100, 50, 200)
            .unwrap();
        s.cast_vote(2, "alice", VoteOption::No, 100, 50, 200)
            .unwrap();
        assert_eq!(s.get_vote(1, "alice").unwrap().option, VoteOption::Yes);
        assert_eq!(s.get_vote(2, "alice").unwrap().option, VoteOption::No);
    }

    // --- tally ---

    #[test]
    fn tally_narrow_reject_at_threshold_boundary() {
        let s = store();
        // 2000 yes / 3000 total = 66% — just below 67% threshold.
        s.cast_vote(1, "v1", VoteOption::Yes, 1000, 10, 100)
            .unwrap();
        s.cast_vote(1, "v2", VoteOption::Yes, 1000, 10, 100)
            .unwrap();
        s.cast_vote(1, "v3", VoteOption::No, 1000, 10, 100).unwrap();

        let result = s.tally(1, 3000, 33, 67);
        assert!(result.quorum_reached); // 3000/3000 = 100% > 33%
        assert!(!result.passed); // 2000/3000 = 66% < 67%
    }

    #[test]
    fn tally_passed_with_supermajority() {
        let s = store();
        s.cast_vote(1, "v1", VoteOption::Yes, 1000, 10, 100)
            .unwrap();
        s.cast_vote(1, "v2", VoteOption::Yes, 1000, 10, 100)
            .unwrap();
        s.cast_vote(1, "v3", VoteOption::Yes, 500, 10, 100).unwrap();
        s.cast_vote(1, "v4", VoteOption::No, 500, 10, 100).unwrap();

        // yes=2500, no=500 → 2500/3000 = 83% > 67%
        let result = s.tally(1, 3000, 33, 67);
        assert!(result.quorum_reached);
        assert!(result.passed);
        assert_eq!(result.yes_power, 2500);
        assert_eq!(result.no_power, 500);
    }

    #[test]
    fn tally_rejected_below_threshold() {
        let s = store();
        s.cast_vote(1, "v1", VoteOption::Yes, 500, 10, 100).unwrap();
        s.cast_vote(1, "v2", VoteOption::No, 1000, 10, 100).unwrap();

        // yes=500, no=1000 → 500/1500 = 33% < 67%
        let result = s.tally(1, 3000, 33, 67);
        assert!(result.quorum_reached); // 1500/3000 = 50% > 33%
        assert!(!result.passed);
    }

    #[test]
    fn tally_no_quorum() {
        let s = store();
        // Only 1 of 3 validators votes (power 1000 of 3000 = 33%).
        s.cast_vote(1, "v1", VoteOption::Yes, 1000, 10, 100)
            .unwrap();

        // Quorum requires 34%.
        let result = s.tally(1, 3000, 34, 67);
        assert!(!result.quorum_reached); // 1000/3000 = 33% < 34%
        assert!(!result.passed);
    }

    #[test]
    fn tally_abstain_counts_for_quorum_not_threshold() {
        let s = store();
        s.cast_vote(1, "v1", VoteOption::Yes, 500, 10, 100).unwrap();
        s.cast_vote(1, "v2", VoteOption::Abstain, 2000, 10, 100)
            .unwrap();

        // total_voted = 2500/3000 = 83% → quorum met (33%)
        // yes/(yes+no) = 500/500 = 100% → passes threshold (67%)
        let result = s.tally(1, 3000, 33, 67);
        assert!(result.quorum_reached);
        assert!(result.passed);
        assert_eq!(result.abstain_power, 2000);
    }

    #[test]
    fn tally_no_votes() {
        let s = store();
        let result = s.tally(1, 3000, 33, 67);
        assert!(!result.quorum_reached);
        assert!(!result.passed);
        assert_eq!(result.total_voted_power, 0);
    }

    #[test]
    fn tally_zero_total_stake() {
        let s = store();
        s.cast_vote(1, "v1", VoteOption::Yes, 100, 10, 100).unwrap();
        let result = s.tally(1, 0, 33, 67);
        assert!(!result.quorum_reached); // Division by zero guarded
        assert!(!result.passed);
    }

    // --- get_votes ---

    #[test]
    fn get_votes_returns_all() {
        let s = store();
        s.cast_vote(1, "v1", VoteOption::Yes, 100, 10, 100).unwrap();
        s.cast_vote(1, "v2", VoteOption::No, 200, 10, 100).unwrap();

        let votes = s.get_votes(1);
        assert_eq!(votes.len(), 2);
    }

    #[test]
    fn get_votes_empty_proposal() {
        let s = store();
        assert!(s.get_votes(999).is_empty());
    }

    // --- vote serde ---

    #[test]
    fn vote_option_serde_roundtrip() {
        for opt in [VoteOption::Yes, VoteOption::No, VoteOption::Abstain] {
            let json = serde_json::to_string(&opt).unwrap();
            let back: VoteOption = serde_json::from_str(&json).unwrap();
            assert_eq!(opt, back);
        }
    }

    // --- integration: full governance flow ---

    #[test]
    fn full_governance_flow() {
        use super::super::params::{keys, ParamRegistry, ParamValue};
        use super::super::proposals::{ProposalAction, ProposalStatus, ProposalStore};

        let params = ParamRegistry::with_defaults();
        let proposals = ProposalStore::new();
        let votes = VoteStore::new();

        let deposit = params.get_u64(keys::PROPOSAL_DEPOSIT, 10_000);
        let voting_period = params.get_u64(keys::VOTING_PERIOD_BLOCKS, 17_280);
        let timelock = params.get_u64(keys::TIMELOCK_BLOCKS, 5_760);
        let quorum = params.get_u64(keys::QUORUM_PERCENT, 33);
        let threshold = params.get_u64(keys::PASS_THRESHOLD_PERCENT, 67);

        // 1. Submit proposal to change min_tx_fee from 1 to 5.
        let action = ProposalAction::ParamChange {
            changes: vec![(keys::MIN_TX_FEE.to_string(), ParamValue::U64(5))],
        };
        let pid = proposals
            .submit(super::super::proposals::SubmitParams {
                proposer: "alice",
                action,
                description: "raise min fee to 5",
                deposit,
                required_deposit: deposit,
                current_height: 1000,
                voting_period,
            })
            .unwrap();

        // 2. Validators vote (total stake = 10000).
        votes
            .cast_vote(pid, "v1", VoteOption::Yes, 3000, 1001, 1000 + voting_period)
            .unwrap();
        votes
            .cast_vote(pid, "v2", VoteOption::Yes, 4000, 1002, 1000 + voting_period)
            .unwrap();
        votes
            .cast_vote(pid, "v3", VoteOption::No, 2000, 1003, 1000 + voting_period)
            .unwrap();
        // v4 abstains (doesn't vote)

        // 3. Tally after voting period.
        let tally = votes.tally(pid, 10_000, quorum, threshold);
        assert!(tally.quorum_reached); // 9000/10000 = 90%
        assert!(tally.passed); // 7000/9000 = 77% > 67%

        // 4. Mark passed + timelock.
        proposals
            .mark_passed(pid, 1000 + voting_period, timelock)
            .unwrap();
        let p = proposals.get(pid).unwrap();
        assert_eq!(p.status, ProposalStatus::Passed);

        // 5. Execute after timelock.
        let executed = proposals
            .mark_executed(pid, 1000 + voting_period + timelock)
            .unwrap();
        assert_eq!(executed.status, ProposalStatus::Executed);

        // 6. Apply parameter changes.
        if let ProposalAction::ParamChange { changes } = &executed.action {
            for (key, value) in changes {
                params.set(key, value.clone());
            }
        }
        assert_eq!(params.get_u64(keys::MIN_TX_FEE, 0), 5);
    }
}
