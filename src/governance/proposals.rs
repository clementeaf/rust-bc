//! Governance proposals — lifecycle from submission through execution.
//!
//! Proposal flow:
//! 1. **Submit**: proposer deposits tokens, proposal enters `Voting` state
//! 2. **Vote**: validators cast stake-weighted votes during voting period
//! 3. **Tally**: after voting period ends, check quorum + pass threshold
//! 4. **Timelock**: if passed, wait `timelock_blocks` before execution
//! 5. **Execute**: apply parameter changes; refund deposit
//! 6. **Reject/Expire**: deposit returned (reject) or slashed (spam)

use std::collections::HashMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use super::params::ParamValue;

/// Unique proposal identifier.
pub type ProposalId = u64;

/// Proposal status lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProposalStatus {
    /// Active voting period.
    Voting,
    /// Voting ended, passed — waiting for timelock to expire.
    Passed,
    /// Voting ended, did not meet quorum or threshold.
    Rejected,
    /// Timelock expired — changes applied.
    Executed,
    /// Proposal was cancelled by the proposer before voting ended.
    Cancelled,
    /// Voting period expired without reaching quorum.
    Expired,
}

/// The type of change a proposal makes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProposalAction {
    /// Change one or more protocol parameters.
    ParamChange { changes: Vec<(String, ParamValue)> },
    /// Free-form text proposal (signaling only, no on-chain effect).
    TextProposal { title: String, description: String },
}

/// A governance proposal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub id: ProposalId,
    pub proposer: String,
    pub action: ProposalAction,
    pub status: ProposalStatus,
    /// Deposit locked by the proposer (returned on execution or rejection).
    pub deposit: u64,
    /// Block height when the proposal was submitted.
    pub submitted_at: u64,
    /// Block height when voting ends.
    pub voting_ends_at: u64,
    /// Block height when timelock expires (set after passing).
    pub timelock_ends_at: Option<u64>,
    /// Block height when the proposal was finalized.
    pub finalized_at: Option<u64>,
    /// Description / rationale.
    pub description: String,
}

/// Errors from the proposal system.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProposalError {
    #[error("proposal not found: {0}")]
    NotFound(ProposalId),
    #[error("insufficient deposit: need {required}, offered {offered}")]
    InsufficientDeposit { required: u64, offered: u64 },
    #[error("invalid state: expected {expected}, got {got:?}")]
    InvalidState {
        expected: &'static str,
        got: ProposalStatus,
    },
    #[error("voting period not ended (ends at block {0})")]
    VotingNotEnded(u64),
    #[error("timelock not expired (expires at block {0})")]
    TimelockNotExpired(u64),
    #[error("no changes to apply")]
    EmptyChanges,
    #[error("not the proposer")]
    NotProposer,
    #[error("not authorized for emergency veto")]
    NotAuthorized,
}

/// Parameters for submitting a new proposal.
pub struct SubmitParams<'a> {
    pub proposer: &'a str,
    pub action: ProposalAction,
    pub description: &'a str,
    pub deposit: u64,
    pub required_deposit: u64,
    pub current_height: u64,
    pub voting_period: u64,
}

/// Proposal store.
pub struct ProposalStore {
    proposals: Mutex<HashMap<ProposalId, Proposal>>,
    next_id: Mutex<ProposalId>,
}

impl ProposalStore {
    pub fn new() -> Self {
        Self {
            proposals: Mutex::new(HashMap::new()),
            next_id: Mutex::new(1),
        }
    }

    /// Submit a new proposal.
    ///
    /// Returns the proposal ID. Caller must verify the proposer has sufficient
    /// balance and deduct the deposit externally.
    pub fn submit(&self, params: SubmitParams<'_>) -> Result<ProposalId, ProposalError> {
        let SubmitParams {
            proposer,
            action,
            description,
            deposit,
            required_deposit,
            current_height,
            voting_period,
        } = params;
        if deposit < required_deposit {
            return Err(ProposalError::InsufficientDeposit {
                required: required_deposit,
                offered: deposit,
            });
        }

        if let ProposalAction::ParamChange { ref changes } = action {
            if changes.is_empty() {
                return Err(ProposalError::EmptyChanges);
            }
        }

        let mut next = self.next_id.lock().unwrap();
        let id = *next;
        *next += 1;

        let proposal = Proposal {
            id,
            proposer: proposer.to_string(),
            action,
            status: ProposalStatus::Voting,
            deposit,
            submitted_at: current_height,
            voting_ends_at: current_height + voting_period,
            timelock_ends_at: None,
            finalized_at: None,
            description: description.to_string(),
        };

        self.proposals.lock().unwrap().insert(id, proposal);
        Ok(id)
    }

    /// Get a proposal by ID.
    pub fn get(&self, id: ProposalId) -> Option<Proposal> {
        self.proposals.lock().unwrap().get(&id).cloned()
    }

    /// Mark a proposal as passed and set the timelock.
    pub fn mark_passed(
        &self,
        id: ProposalId,
        current_height: u64,
        timelock_blocks: u64,
    ) -> Result<(), ProposalError> {
        let mut proposals = self.proposals.lock().unwrap();
        let p = proposals.get_mut(&id).ok_or(ProposalError::NotFound(id))?;

        if p.status != ProposalStatus::Voting {
            return Err(ProposalError::InvalidState {
                expected: "Voting",
                got: p.status,
            });
        }

        if current_height < p.voting_ends_at {
            return Err(ProposalError::VotingNotEnded(p.voting_ends_at));
        }

        p.status = ProposalStatus::Passed;
        p.timelock_ends_at = Some(current_height + timelock_blocks);
        Ok(())
    }

    /// Mark a proposal as rejected.
    pub fn mark_rejected(&self, id: ProposalId, current_height: u64) -> Result<(), ProposalError> {
        let mut proposals = self.proposals.lock().unwrap();
        let p = proposals.get_mut(&id).ok_or(ProposalError::NotFound(id))?;

        if p.status != ProposalStatus::Voting {
            return Err(ProposalError::InvalidState {
                expected: "Voting",
                got: p.status,
            });
        }

        p.status = ProposalStatus::Rejected;
        p.finalized_at = Some(current_height);
        Ok(())
    }

    /// Mark a proposal as executed.
    pub fn mark_executed(
        &self,
        id: ProposalId,
        current_height: u64,
    ) -> Result<Proposal, ProposalError> {
        let mut proposals = self.proposals.lock().unwrap();
        let p = proposals.get_mut(&id).ok_or(ProposalError::NotFound(id))?;

        if p.status != ProposalStatus::Passed {
            return Err(ProposalError::InvalidState {
                expected: "Passed",
                got: p.status,
            });
        }

        if let Some(tl) = p.timelock_ends_at {
            if current_height < tl {
                return Err(ProposalError::TimelockNotExpired(tl));
            }
        }

        p.status = ProposalStatus::Executed;
        p.finalized_at = Some(current_height);
        Ok(p.clone())
    }

    /// Cancel a proposal (only by proposer, only during voting).
    pub fn cancel(
        &self,
        id: ProposalId,
        caller: &str,
        current_height: u64,
    ) -> Result<Proposal, ProposalError> {
        let mut proposals = self.proposals.lock().unwrap();
        let p = proposals.get_mut(&id).ok_or(ProposalError::NotFound(id))?;

        if p.proposer != caller {
            return Err(ProposalError::NotProposer);
        }

        if p.status != ProposalStatus::Voting {
            return Err(ProposalError::InvalidState {
                expected: "Voting",
                got: p.status,
            });
        }

        p.status = ProposalStatus::Cancelled;
        p.finalized_at = Some(current_height);
        Ok(p.clone())
    }

    /// List proposals filtered by status.
    pub fn list_by_status(&self, status: ProposalStatus) -> Vec<Proposal> {
        self.proposals
            .lock()
            .unwrap()
            .values()
            .filter(|p| p.status == status)
            .cloned()
            .collect()
    }

    /// Total number of proposals.
    pub fn count(&self) -> usize {
        self.proposals.lock().unwrap().len()
    }

    /// Emergency veto — cancels a proposal in any non-final state.
    /// Only callable by addresses in the `authorized_vetoers` list.
    pub fn emergency_veto(
        &self,
        id: ProposalId,
        caller: &str,
        authorized_vetoers: &[String],
        current_height: u64,
    ) -> Result<Proposal, ProposalError> {
        if !authorized_vetoers.iter().any(|a| a == caller) {
            return Err(ProposalError::NotAuthorized);
        }

        let mut proposals = self.proposals.lock().unwrap();
        let p = proposals.get_mut(&id).ok_or(ProposalError::NotFound(id))?;

        // Can veto anything that isn't already Executed or Cancelled
        if p.status == ProposalStatus::Executed || p.status == ProposalStatus::Cancelled {
            return Err(ProposalError::InvalidState {
                expected: "Voting or Passed",
                got: p.status,
            });
        }

        p.status = ProposalStatus::Cancelled;
        p.finalized_at = Some(current_height);
        Ok(p.clone())
    }
}

impl Default for ProposalStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> ProposalStore {
        ProposalStore::new()
    }

    fn param_change(key: &str, val: u64) -> ProposalAction {
        ProposalAction::ParamChange {
            changes: vec![(key.to_string(), ParamValue::U64(val))],
        }
    }

    fn sp<'a>(
        proposer: &'a str,
        action: ProposalAction,
        desc: &'a str,
        deposit: u64,
        required: u64,
        height: u64,
        period: u64,
    ) -> SubmitParams<'a> {
        SubmitParams {
            proposer,
            action,
            description: desc,
            deposit,
            required_deposit: required,
            current_height: height,
            voting_period: period,
        }
    }

    // --- submit ---

    #[test]
    fn submit_proposal() {
        let s = store();
        let id = s
            .submit(sp(
                "alice",
                param_change("min_tx_fee", 5),
                "raise fee",
                10_000,
                10_000,
                100,
                1000,
            ))
            .unwrap();
        assert_eq!(id, 1);

        let p = s.get(id).unwrap();
        assert_eq!(p.proposer, "alice");
        assert_eq!(p.status, ProposalStatus::Voting);
        assert_eq!(p.voting_ends_at, 1100);
        assert_eq!(p.deposit, 10_000);
    }

    #[test]
    fn submit_insufficient_deposit() {
        let s = store();
        let err = s
            .submit(sp(
                "alice",
                param_change("min_tx_fee", 5),
                "",
                100,
                10_000,
                0,
                1000,
            ))
            .unwrap_err();
        assert!(matches!(err, ProposalError::InsufficientDeposit { .. }));
    }

    #[test]
    fn submit_empty_changes_rejected() {
        let s = store();
        let action = ProposalAction::ParamChange { changes: vec![] };
        let err = s
            .submit(sp("alice", action, "", 10_000, 10_000, 0, 1000))
            .unwrap_err();
        assert!(matches!(err, ProposalError::EmptyChanges));
    }

    #[test]
    fn submit_text_proposal_ok() {
        let s = store();
        let action = ProposalAction::TextProposal {
            title: "Upgrade plan".into(),
            description: "Details...".into(),
        };
        let id = s
            .submit(sp("alice", action, "signal", 10_000, 10_000, 0, 1000))
            .unwrap();
        assert_eq!(s.get(id).unwrap().status, ProposalStatus::Voting);
    }

    #[test]
    fn ids_increment() {
        let s = store();
        let id1 = s
            .submit(sp("a", param_change("k", 1), "", 100, 100, 0, 10))
            .unwrap();
        let id2 = s
            .submit(sp("b", param_change("k", 2), "", 100, 100, 0, 10))
            .unwrap();
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
    }

    // --- lifecycle: pass → execute ---

    #[test]
    fn pass_and_execute_lifecycle() {
        let s = store();
        let id = s
            .submit(sp("alice", param_change("k", 1), "", 100, 100, 0, 100))
            .unwrap();

        // Can't pass before voting ends.
        let err = s.mark_passed(id, 50, 50).unwrap_err();
        assert!(matches!(err, ProposalError::VotingNotEnded(100)));

        // Pass after voting ends.
        s.mark_passed(id, 100, 50).unwrap();
        assert_eq!(s.get(id).unwrap().status, ProposalStatus::Passed);
        assert_eq!(s.get(id).unwrap().timelock_ends_at, Some(150));

        // Can't execute before timelock.
        let err = s.mark_executed(id, 120).unwrap_err();
        assert!(matches!(err, ProposalError::TimelockNotExpired(150)));

        // Execute after timelock.
        let p = s.mark_executed(id, 150).unwrap();
        assert_eq!(p.status, ProposalStatus::Executed);
        assert_eq!(p.finalized_at, Some(150));
    }

    // --- reject ---

    #[test]
    fn reject_proposal() {
        let s = store();
        let id = s
            .submit(sp("alice", param_change("k", 1), "", 100, 100, 0, 100))
            .unwrap();
        s.mark_rejected(id, 100).unwrap();
        assert_eq!(s.get(id).unwrap().status, ProposalStatus::Rejected);
    }

    #[test]
    fn reject_already_passed_fails() {
        let s = store();
        let id = s
            .submit(sp("alice", param_change("k", 1), "", 100, 100, 0, 100))
            .unwrap();
        s.mark_passed(id, 100, 50).unwrap();
        let err = s.mark_rejected(id, 110).unwrap_err();
        assert!(matches!(err, ProposalError::InvalidState { .. }));
    }

    // --- cancel ---

    #[test]
    fn cancel_by_proposer() {
        let s = store();
        let id = s
            .submit(sp("alice", param_change("k", 1), "", 100, 100, 0, 100))
            .unwrap();
        let p = s.cancel(id, "alice", 50).unwrap();
        assert_eq!(p.status, ProposalStatus::Cancelled);
    }

    #[test]
    fn cancel_by_non_proposer_fails() {
        let s = store();
        let id = s
            .submit(sp("alice", param_change("k", 1), "", 100, 100, 0, 100))
            .unwrap();
        let err = s.cancel(id, "bob", 50).unwrap_err();
        assert!(matches!(err, ProposalError::NotProposer));
    }

    // --- list ---

    #[test]
    fn list_by_status() {
        let s = store();
        s.submit(sp("a", param_change("k", 1), "", 100, 100, 0, 100))
            .unwrap();
        s.submit(sp("b", param_change("k", 2), "", 100, 100, 0, 100))
            .unwrap();
        let id3 = s
            .submit(sp("c", param_change("k", 3), "", 100, 100, 0, 100))
            .unwrap();
        s.mark_rejected(id3, 100).unwrap();

        assert_eq!(s.list_by_status(ProposalStatus::Voting).len(), 2);
        assert_eq!(s.list_by_status(ProposalStatus::Rejected).len(), 1);
        assert_eq!(s.count(), 3);
    }
}
