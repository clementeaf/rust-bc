/**
 * Governance Contracts Module
 *
 * DAO-like voting system for smart contracts:
 * - Proposal creation (text or action proposals)
 * - Voting mechanism (weighted by token holdings)
 * - Proposal execution (if quorum + majority met)
 * - Time-locks for security
 */

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Proposal status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProposalStatus {
    Pending,
    Active,
    Succeeded,
    Failed,
    Executed,
    Cancelled,
}

/// Proposal action type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProposalAction {
    TextProposal(String),
    Transfer { to: String, amount: u64 },
    Custom { action_type: String, params: Vec<String> },
}

/// Vote cast by a voter
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum VoteType {
    For,
    Against,
    Abstain,
}

/// A governance proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub id: u64,
    pub proposer: String,
    pub action: ProposalAction,
    pub status: ProposalStatus,
    pub votes_for: u64,
    pub votes_against: u64,
    pub votes_abstain: u64,
    pub total_voters: u64,
    pub created_block: u64,
    pub voting_period_blocks: u64,
    pub timelock_blocks: u64,
    pub executed_at: Option<u64>,
    pub voters: HashMap<String, VoteType>,
}

impl Proposal {
    pub fn new(
        id: u64,
        proposer: String,
        action: ProposalAction,
        created_block: u64,
        voting_period_blocks: u64,
        timelock_blocks: u64,
    ) -> Self {
        Proposal {
            id,
            proposer,
            action,
            status: ProposalStatus::Pending,
            votes_for: 0,
            votes_against: 0,
            votes_abstain: 0,
            total_voters: 0,
            created_block,
            voting_period_blocks,
            timelock_blocks,
            executed_at: None,
            voters: HashMap::new(),
        }
    }

    /// Calculate voting progress (0-100)
    pub fn voting_progress(&self, current_block: u64) -> u8 {
        let voting_end = self.created_block + self.voting_period_blocks;
        if current_block >= voting_end {
            100
        } else {
            let blocks_passed = current_block.saturating_sub(self.created_block);
            ((blocks_passed as f64 / self.voting_period_blocks as f64) * 100.0) as u8
        }
    }

    /// Check if voting is still active
    pub fn is_voting_active(&self, current_block: u64) -> bool {
        self.status == ProposalStatus::Active
            && current_block < self.created_block + self.voting_period_blocks
    }

    /// Check if proposal can be executed
    pub fn can_execute(&self, current_block: u64) -> bool {
        self.status == ProposalStatus::Succeeded
            && current_block >= self.created_block + self.voting_period_blocks + self.timelock_blocks
    }

    /// Check if proposal passed (majority + quorum)
    pub fn has_passed(&self, quorum_percentage: u8) -> bool {
        let total_votes = self.votes_for + self.votes_against + self.votes_abstain;
        if total_votes == 0 {
            return false;
        }

        let quorum_met = (self.votes_for + self.votes_against + self.votes_abstain) as f64
            >= (self.total_voters as f64 * quorum_percentage as f64 / 100.0);
        let majority_met = self.votes_for > self.votes_against;

        quorum_met && majority_met
    }
}

/// Governance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceConfig {
    pub quorum_percentage: u8,          // 0-100
    pub voting_period_blocks: u64,      // Blocks for voting
    pub timelock_blocks: u64,           // Blocks delay before execution
    pub min_proposal_balance: u64,      // Min tokens to create proposal
    pub proposal_threshold: u8,         // Min % of token holders to propose
}

impl Default for GovernanceConfig {
    fn default() -> Self {
        GovernanceConfig {
            quorum_percentage: 50,
            voting_period_blocks: 1000,
            timelock_blocks: 100,
            min_proposal_balance: 1000,
            proposal_threshold: 1,
        }
    }
}

/// Governance contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceContract {
    pub contract_address: String,
    pub owner: String,
    pub token_address: String,           // Address of governance token
    pub config: GovernanceConfig,
    pub proposals: HashMap<u64, Proposal>,
    pub next_proposal_id: u64,
    pub total_holders: u64,              // Total token holders
    pub executed_proposals: u64,
}

impl GovernanceContract {
    pub fn new(
        contract_address: String,
        owner: String,
        token_address: String,
        config: GovernanceConfig,
    ) -> Self {
        GovernanceContract {
            contract_address,
            owner,
            token_address,
            config,
            proposals: HashMap::new(),
            next_proposal_id: 1,
            total_holders: 0,
            executed_proposals: 0,
        }
    }

    /// Create a proposal
    pub fn create_proposal(
        &mut self,
        proposer: String,
        action: ProposalAction,
        current_block: u64,
        proposer_balance: u64,
    ) -> Result<u64, String> {
        if proposer_balance < self.config.min_proposal_balance {
            return Err(format!(
                "Insufficient balance to propose: {} < {}",
                proposer_balance, self.config.min_proposal_balance
            ));
        }

        if proposer == self.owner {
            // Owner can always propose
        } else if self.total_holders > 0 {
            let threshold_holders = (self.total_holders as f64
                * self.config.proposal_threshold as f64
                / 100.0) as u64;
            if proposer_balance < threshold_holders {
                return Err(format!(
                    "Proposer does not meet threshold: {} < {}",
                    proposer_balance, threshold_holders
                ));
            }
        }

        let proposal_id = self.next_proposal_id;
        let mut proposal = Proposal::new(
            proposal_id,
            proposer,
            action,
            current_block,
            self.config.voting_period_blocks,
            self.config.timelock_blocks,
        );
        proposal.total_voters = self.total_holders;

        self.proposals.insert(proposal_id, proposal);
        self.next_proposal_id += 1;

        Ok(proposal_id)
    }

    /// Activate a proposal for voting
    pub fn activate_proposal(&mut self, proposal_id: u64, _current_block: u64) -> Result<(), String> {
        let proposal = self
            .proposals
            .get_mut(&proposal_id)
            .ok_or("Proposal not found")?;

        if proposal.status != ProposalStatus::Pending {
            return Err("Proposal is not pending".to_string());
        }

        proposal.status = ProposalStatus::Active;
        Ok(())
    }

    /// Cast a vote on a proposal
    pub fn vote(
        &mut self,
        proposal_id: u64,
        voter: String,
        vote_type: VoteType,
        voting_power: u64,
        current_block: u64,
    ) -> Result<(), String> {
        let proposal = self
            .proposals
            .get_mut(&proposal_id)
            .ok_or("Proposal not found")?;

        if !proposal.is_voting_active(current_block) {
            return Err("Voting is not active for this proposal".to_string());
        }

        if voting_power == 0 {
            return Err("Voter has no voting power".to_string());
        }

        // Check if voter already voted
        if let Some(existing_vote) = proposal.voters.get(&voter) {
            if *existing_vote == VoteType::For {
                proposal.votes_for = proposal.votes_for.saturating_sub(voting_power);
            } else if *existing_vote == VoteType::Against {
                proposal.votes_against = proposal.votes_against.saturating_sub(voting_power);
            } else {
                proposal.votes_abstain = proposal.votes_abstain.saturating_sub(voting_power);
            }
        }

        // Record new vote
        proposal.voters.insert(voter, vote_type);
        match vote_type {
            VoteType::For => proposal.votes_for += voting_power,
            VoteType::Against => proposal.votes_against += voting_power,
            VoteType::Abstain => proposal.votes_abstain += voting_power,
        }

        Ok(())
    }

    /// Finalize voting and update proposal status
    pub fn finalize_voting(&mut self, proposal_id: u64, current_block: u64) -> Result<(), String> {
        let proposal = self
            .proposals
            .get_mut(&proposal_id)
            .ok_or("Proposal not found")?;

        if proposal.status != ProposalStatus::Active {
            return Err("Proposal is not active".to_string());
        }

        if current_block < proposal.created_block + proposal.voting_period_blocks {
            return Err("Voting period has not ended".to_string());
        }

        if proposal.has_passed(self.config.quorum_percentage) {
            proposal.status = ProposalStatus::Succeeded;
        } else {
            proposal.status = ProposalStatus::Failed;
        }

        Ok(())
    }

    /// Execute a proposal
    pub fn execute_proposal(&mut self, proposal_id: u64, current_block: u64) -> Result<String, String> {
        let proposal = self
            .proposals
            .get_mut(&proposal_id)
            .ok_or("Proposal not found")?;

        if proposal.status == ProposalStatus::Executed {
            return Err("Proposal already executed".to_string());
        }

        if !proposal.can_execute(current_block) {
            return Err("Proposal cannot be executed yet".to_string());
        }

        proposal.status = ProposalStatus::Executed;
        proposal.executed_at = Some(current_block);
        self.executed_proposals += 1;

        let result = match &proposal.action {
            ProposalAction::TextProposal(text) => format!("Text proposal executed: {}", text),
            ProposalAction::Transfer { to, amount } => {
                format!("Transfer executed: {} tokens to {}", amount, to)
            }
            ProposalAction::Custom {
                action_type,
                params,
            } => {
                format!(
                    "Custom action executed: {} with params: {}",
                    action_type,
                    params.join(", ")
                )
            }
        };

        Ok(result)
    }

    /// Cancel a proposal (owner only)
    pub fn cancel_proposal(
        &mut self,
        proposal_id: u64,
        caller: &str,
    ) -> Result<(), String> {
        if caller != self.owner {
            return Err("Only owner can cancel proposals".to_string());
        }

        let proposal = self
            .proposals
            .get_mut(&proposal_id)
            .ok_or("Proposal not found")?;

        if proposal.status == ProposalStatus::Executed {
            return Err("Cannot cancel executed proposal".to_string());
        }

        proposal.status = ProposalStatus::Cancelled;
        Ok(())
    }

    /// Get proposal details
    pub fn get_proposal(&self, proposal_id: u64) -> Option<Proposal> {
        self.proposals.get(&proposal_id).cloned()
    }

    /// Update governance config (owner only)
    pub fn update_config(&mut self, caller: &str, new_config: GovernanceConfig) -> Result<(), String> {
        if caller != self.owner {
            return Err("Only owner can update config".to_string());
        }

        if new_config.quorum_percentage > 100 {
            return Err("Quorum percentage cannot exceed 100".to_string());
        }

        self.config = new_config;
        Ok(())
    }

    /// Update total token holders
    pub fn update_total_holders(&mut self, new_total: u64) {
        self.total_holders = new_total;
    }
}
