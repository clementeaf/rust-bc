use rust_bc::governance_contracts::*;

#[test]
fn test_governance_contract_creation() {
    let contract = GovernanceContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        GovernanceConfig::default(),
    );

    assert_eq!(contract.contract_address, "0x123");
    assert_eq!(contract.owner, "owner");
    assert_eq!(contract.token_address, "0x456");
    assert_eq!(contract.next_proposal_id, 1);
    assert_eq!(contract.executed_proposals, 0);
}

#[test]
fn test_proposal_creation() {
    let mut contract = GovernanceContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        GovernanceConfig::default(),
    );

    contract.update_total_holders(100);

    let proposal_id = contract
        .create_proposal(
            "owner".to_string(),
            ProposalAction::TextProposal("Test proposal".to_string()),
            100,
            1000,
        )
        .unwrap();

    assert_eq!(proposal_id, 1);
    assert_eq!(contract.next_proposal_id, 2);

    let proposal = contract.get_proposal(proposal_id).unwrap();
    assert_eq!(proposal.id, 1);
    assert_eq!(proposal.proposer, "owner");
    assert_eq!(proposal.status, ProposalStatus::Pending);
    assert_eq!(proposal.created_block, 100);
}

#[test]
fn test_proposal_creation_insufficient_balance() {
    let mut contract = GovernanceContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        GovernanceConfig::default(),
    );

    let result = contract.create_proposal(
        "user1".to_string(),
        ProposalAction::TextProposal("Test".to_string()),
        100,
        500, // Less than min_proposal_balance (1000)
    );

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .contains("Insufficient balance to propose"));
}

#[test]
fn test_proposal_activation() {
    let mut contract = GovernanceContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        GovernanceConfig::default(),
    );

    contract.update_total_holders(100);

    let proposal_id = contract
        .create_proposal(
            "owner".to_string(),
            ProposalAction::TextProposal("Test".to_string()),
            100,
            1000,
        )
        .unwrap();

    contract.activate_proposal(proposal_id, 100).unwrap();

    let proposal = contract.get_proposal(proposal_id).unwrap();
    assert_eq!(proposal.status, ProposalStatus::Active);
}

#[test]
fn test_voting() {
    let mut contract = GovernanceContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        GovernanceConfig::default(),
    );

    contract.update_total_holders(100);

    let proposal_id = contract
        .create_proposal(
            "owner".to_string(),
            ProposalAction::TextProposal("Test".to_string()),
            100,
            1000,
        )
        .unwrap();

    contract.activate_proposal(proposal_id, 100).unwrap();

    // Vote for
    contract
        .vote(proposal_id, "voter1".to_string(), VoteType::For, 50, 100)
        .unwrap();

    let proposal = contract.get_proposal(proposal_id).unwrap();
    assert_eq!(proposal.votes_for, 50);
    assert_eq!(proposal.votes_against, 0);
    assert_eq!(proposal.votes_abstain, 0);

    // Vote against
    contract
        .vote(proposal_id, "voter2".to_string(), VoteType::Against, 30, 100)
        .unwrap();

    let proposal = contract.get_proposal(proposal_id).unwrap();
    assert_eq!(proposal.votes_for, 50);
    assert_eq!(proposal.votes_against, 30);
}

#[test]
fn test_vote_change() {
    let mut contract = GovernanceContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        GovernanceConfig::default(),
    );

    contract.update_total_holders(100);

    let proposal_id = contract
        .create_proposal(
            "owner".to_string(),
            ProposalAction::TextProposal("Test".to_string()),
            100,
            1000,
        )
        .unwrap();

    contract.activate_proposal(proposal_id, 100).unwrap();

    // Vote for
    contract
        .vote(proposal_id, "voter1".to_string(), VoteType::For, 50, 100)
        .unwrap();

    let proposal = contract.get_proposal(proposal_id).unwrap();
    assert_eq!(proposal.votes_for, 50);

    // Change to against
    contract
        .vote(proposal_id, "voter1".to_string(), VoteType::Against, 50, 100)
        .unwrap();

    let proposal = contract.get_proposal(proposal_id).unwrap();
    assert_eq!(proposal.votes_for, 0);
    assert_eq!(proposal.votes_against, 50);
}

#[test]
fn test_voting_during_inactive_proposal() {
    let mut contract = GovernanceContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        GovernanceConfig::default(),
    );

    contract.update_total_holders(100);

    let proposal_id = contract
        .create_proposal(
            "owner".to_string(),
            ProposalAction::TextProposal("Test".to_string()),
            100,
            1000,
        )
        .unwrap();

    // Try to vote without activating
    let result = contract.vote(proposal_id, "voter1".to_string(), VoteType::For, 50, 100);

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .contains("Voting is not active for this proposal"));
}

#[test]
fn test_voting_with_zero_power() {
    let mut contract = GovernanceContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        GovernanceConfig::default(),
    );

    contract.update_total_holders(100);

    let proposal_id = contract
        .create_proposal(
            "owner".to_string(),
            ProposalAction::TextProposal("Test".to_string()),
            100,
            1000,
        )
        .unwrap();

    contract.activate_proposal(proposal_id, 100).unwrap();

    let result = contract.vote(proposal_id, "voter1".to_string(), VoteType::For, 0, 100);

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("no voting power"));
}

#[test]
fn test_finalize_voting_success() {
    let mut contract = GovernanceContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        GovernanceConfig {
            quorum_percentage: 40,
            voting_period_blocks: 1000,
            timelock_blocks: 100,
            min_proposal_balance: 1000,
            proposal_threshold: 1,
        },
    );

    contract.update_total_holders(100);

    let proposal_id = contract
        .create_proposal(
            "owner".to_string(),
            ProposalAction::TextProposal("Test".to_string()),
            100,
            1000,
        )
        .unwrap();

    contract.activate_proposal(proposal_id, 100).unwrap();

    // Vote: 60 for, 30 against = quorum met (90/100 > 40%), majority met (60 > 30)
    contract
        .vote(proposal_id, "voter1".to_string(), VoteType::For, 60, 100)
        .unwrap();
    contract
        .vote(proposal_id, "voter2".to_string(), VoteType::Against, 30, 100)
        .unwrap();

    // Finalize after voting period ends
    contract.finalize_voting(proposal_id, 1200).unwrap();

    let proposal = contract.get_proposal(proposal_id).unwrap();
    assert_eq!(proposal.status, ProposalStatus::Succeeded);
}

#[test]
fn test_finalize_voting_fail_quorum() {
    let mut contract = GovernanceContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        GovernanceConfig {
            quorum_percentage: 80,
            voting_period_blocks: 1000,
            timelock_blocks: 100,
            min_proposal_balance: 1000,
            proposal_threshold: 1,
        },
    );

    contract.update_total_holders(100);

    let proposal_id = contract
        .create_proposal(
            "owner".to_string(),
            ProposalAction::TextProposal("Test".to_string()),
            100,
            1000,
        )
        .unwrap();

    contract.activate_proposal(proposal_id, 100).unwrap();

    // Only 60/100 votes = below 80% quorum
    contract
        .vote(proposal_id, "voter1".to_string(), VoteType::For, 60, 100)
        .unwrap();

    contract.finalize_voting(proposal_id, 1200).unwrap();

    let proposal = contract.get_proposal(proposal_id).unwrap();
    assert_eq!(proposal.status, ProposalStatus::Failed);
}

#[test]
fn test_finalize_voting_fail_majority() {
    let mut contract = GovernanceContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        GovernanceConfig {
            quorum_percentage: 40,
            voting_period_blocks: 1000,
            timelock_blocks: 100,
            min_proposal_balance: 1000,
            proposal_threshold: 1,
        },
    );

    contract.update_total_holders(100);

    let proposal_id = contract
        .create_proposal(
            "owner".to_string(),
            ProposalAction::TextProposal("Test".to_string()),
            100,
            1000,
        )
        .unwrap();

    contract.activate_proposal(proposal_id, 100).unwrap();

    // 40 for, 50 against = quorum met but majority failed
    contract
        .vote(proposal_id, "voter1".to_string(), VoteType::For, 40, 100)
        .unwrap();
    contract
        .vote(proposal_id, "voter2".to_string(), VoteType::Against, 50, 100)
        .unwrap();

    contract.finalize_voting(proposal_id, 1200).unwrap();

    let proposal = contract.get_proposal(proposal_id).unwrap();
    assert_eq!(proposal.status, ProposalStatus::Failed);
}

#[test]
fn test_execute_proposal_success() {
    let mut contract = GovernanceContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        GovernanceConfig {
            quorum_percentage: 40,
            voting_period_blocks: 1000,
            timelock_blocks: 100,
            min_proposal_balance: 1000,
            proposal_threshold: 1,
        },
    );

    contract.update_total_holders(100);

    let proposal_id = contract
        .create_proposal(
            "owner".to_string(),
            ProposalAction::Transfer {
                to: "recipient".to_string(),
                amount: 100,
            },
            100,
            1000,
        )
        .unwrap();

    contract.activate_proposal(proposal_id, 100).unwrap();

    contract
        .vote(proposal_id, "voter1".to_string(), VoteType::For, 60, 100)
        .unwrap();
    contract
        .vote(proposal_id, "voter2".to_string(), VoteType::Against, 30, 100)
        .unwrap();

    contract.finalize_voting(proposal_id, 1200).unwrap();

    // Execute after timelock
    let result = contract.execute_proposal(proposal_id, 1300).unwrap();
    assert!(result.contains("Transfer executed"));

    let proposal = contract.get_proposal(proposal_id).unwrap();
    assert_eq!(proposal.status, ProposalStatus::Executed);
    assert_eq!(contract.executed_proposals, 1);
}

#[test]
fn test_execute_proposal_before_timelock() {
    let mut contract = GovernanceContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        GovernanceConfig {
            quorum_percentage: 40,
            voting_period_blocks: 1000,
            timelock_blocks: 100,
            min_proposal_balance: 1000,
            proposal_threshold: 1,
        },
    );

    contract.update_total_holders(100);

    let proposal_id = contract
        .create_proposal(
            "owner".to_string(),
            ProposalAction::TextProposal("Test".to_string()),
            100,
            1000,
        )
        .unwrap();

    contract.activate_proposal(proposal_id, 100).unwrap();

    contract
        .vote(proposal_id, "voter1".to_string(), VoteType::For, 60, 100)
        .unwrap();

    contract.finalize_voting(proposal_id, 1200).unwrap();

    // Try to execute before timelock ends (at block 1299, needs 1200)
    let result = contract.execute_proposal(proposal_id, 1199);

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("cannot be executed yet"));
}

#[test]
fn test_execute_proposal_not_succeeded() {
    let mut contract = GovernanceContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        GovernanceConfig::default(),
    );

    contract.update_total_holders(100);

    let proposal_id = contract
        .create_proposal(
            "owner".to_string(),
            ProposalAction::TextProposal("Test".to_string()),
            100,
            1000,
        )
        .unwrap();

    contract.activate_proposal(proposal_id, 100).unwrap();

    contract
        .vote(proposal_id, "voter1".to_string(), VoteType::Against, 80, 100)
        .unwrap();

    contract.finalize_voting(proposal_id, 1200).unwrap();

    let result = contract.execute_proposal(proposal_id, 1400);

    assert!(result.is_err());
}

#[test]
fn test_cancel_proposal() {
    let mut contract = GovernanceContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        GovernanceConfig::default(),
    );

    contract.update_total_holders(100);

    let proposal_id = contract
        .create_proposal(
            "owner".to_string(),
            ProposalAction::TextProposal("Test".to_string()),
            100,
            1000,
        )
        .unwrap();

    contract.cancel_proposal(proposal_id, "owner").unwrap();

    let proposal = contract.get_proposal(proposal_id).unwrap();
    assert_eq!(proposal.status, ProposalStatus::Cancelled);
}

#[test]
fn test_cancel_proposal_not_owner() {
    let mut contract = GovernanceContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        GovernanceConfig::default(),
    );

    contract.update_total_holders(100);

    let proposal_id = contract
        .create_proposal(
            "owner".to_string(),
            ProposalAction::TextProposal("Test".to_string()),
            100,
            1000,
        )
        .unwrap();

    let result = contract.cancel_proposal(proposal_id, "not_owner");

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Only owner can cancel"));
}

#[test]
fn test_cancel_executed_proposal() {
    let mut contract = GovernanceContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        GovernanceConfig {
            quorum_percentage: 40,
            voting_period_blocks: 1000,
            timelock_blocks: 100,
            min_proposal_balance: 1000,
            proposal_threshold: 1,
        },
    );

    contract.update_total_holders(100);

    let proposal_id = contract
        .create_proposal(
            "owner".to_string(),
            ProposalAction::TextProposal("Test".to_string()),
            100,
            1000,
        )
        .unwrap();

    contract.activate_proposal(proposal_id, 100).unwrap();

    contract
        .vote(proposal_id, "voter1".to_string(), VoteType::For, 60, 100)
        .unwrap();

    contract.finalize_voting(proposal_id, 1200).unwrap();
    contract.execute_proposal(proposal_id, 1300).unwrap();

    let result = contract.cancel_proposal(proposal_id, "owner");

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Cannot cancel executed proposal"));
}

#[test]
fn test_proposal_voting_progress() {
    let proposal = Proposal::new(
        1,
        "proposer".to_string(),
        ProposalAction::TextProposal("Test".to_string()),
        100,
        1000,
        100,
    );

    assert_eq!(proposal.voting_progress(100), 0);
    assert_eq!(proposal.voting_progress(600), 50);
    assert_eq!(proposal.voting_progress(1100), 100);
}

#[test]
fn test_update_governance_config() {
    let mut contract = GovernanceContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        GovernanceConfig::default(),
    );

    let new_config = GovernanceConfig {
        quorum_percentage: 60,
        voting_period_blocks: 2000,
        timelock_blocks: 200,
        min_proposal_balance: 2000,
        proposal_threshold: 2,
    };

    contract.update_config("owner", new_config.clone()).unwrap();

    assert_eq!(contract.config.quorum_percentage, 60);
    assert_eq!(contract.config.voting_period_blocks, 2000);
}

#[test]
fn test_update_governance_config_not_owner() {
    let mut contract = GovernanceContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        GovernanceConfig::default(),
    );

    let result = contract.update_config("not_owner", GovernanceConfig::default());

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Only owner can update config"));
}

#[test]
fn test_update_governance_config_invalid_quorum() {
    let mut contract = GovernanceContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        GovernanceConfig::default(),
    );

    let invalid_config = GovernanceConfig {
        quorum_percentage: 150, // Invalid
        voting_period_blocks: 1000,
        timelock_blocks: 100,
        min_proposal_balance: 1000,
        proposal_threshold: 1,
    };

    let result = contract.update_config("owner", invalid_config);

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .contains("Quorum percentage cannot exceed 100"));
}

#[test]
fn test_custom_action_proposal() {
    let mut contract = GovernanceContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        GovernanceConfig {
            quorum_percentage: 40,
            voting_period_blocks: 1000,
            timelock_blocks: 100,
            min_proposal_balance: 1000,
            proposal_threshold: 1,
        },
    );

    contract.update_total_holders(100);

    let proposal_id = contract
        .create_proposal(
            "owner".to_string(),
            ProposalAction::Custom {
                action_type: "upgrade_contract".to_string(),
                params: vec!["new_address".to_string(), "v2".to_string()],
            },
            100,
            1000,
        )
        .unwrap();

    contract.activate_proposal(proposal_id, 100).unwrap();

    contract
        .vote(proposal_id, "voter1".to_string(), VoteType::For, 60, 100)
        .unwrap();

    contract.finalize_voting(proposal_id, 1200).unwrap();

    let result = contract.execute_proposal(proposal_id, 1300).unwrap();
    assert!(result.contains("Custom action executed"));
    assert!(result.contains("upgrade_contract"));
}

#[test]
fn test_abstain_votes() {
    let mut contract = GovernanceContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        GovernanceConfig {
            quorum_percentage: 40,
            voting_period_blocks: 1000,
            timelock_blocks: 100,
            min_proposal_balance: 1000,
            proposal_threshold: 1,
        },
    );

    contract.update_total_holders(100);

    let proposal_id = contract
        .create_proposal(
            "owner".to_string(),
            ProposalAction::TextProposal("Test".to_string()),
            100,
            1000,
        )
        .unwrap();

    contract.activate_proposal(proposal_id, 100).unwrap();

    contract
        .vote(proposal_id, "voter1".to_string(), VoteType::For, 40, 100)
        .unwrap();
    contract
        .vote(proposal_id, "voter2".to_string(), VoteType::Against, 30, 100)
        .unwrap();
    contract
        .vote(proposal_id, "voter3".to_string(), VoteType::Abstain, 30, 100)
        .unwrap();

    contract.finalize_voting(proposal_id, 1200).unwrap();

    let proposal = contract.get_proposal(proposal_id).unwrap();
    assert_eq!(proposal.votes_for, 40);
    assert_eq!(proposal.votes_against, 30);
    assert_eq!(proposal.votes_abstain, 30);
    assert_eq!(proposal.status, ProposalStatus::Succeeded);
}

#[test]
fn test_finalize_voting_early() {
    let mut contract = GovernanceContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        GovernanceConfig::default(),
    );

    contract.update_total_holders(100);

    let proposal_id = contract
        .create_proposal(
            "owner".to_string(),
            ProposalAction::TextProposal("Test".to_string()),
            100,
            1000,
        )
        .unwrap();

    contract.activate_proposal(proposal_id, 100).unwrap();

    // Try to finalize before voting period ends
    let result = contract.finalize_voting(proposal_id, 500);

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Voting period has not ended"));
}

#[test]
fn test_execute_already_executed() {
    let mut contract = GovernanceContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        GovernanceConfig {
            quorum_percentage: 40,
            voting_period_blocks: 1000,
            timelock_blocks: 100,
            min_proposal_balance: 1000,
            proposal_threshold: 1,
        },
    );

    contract.update_total_holders(100);

    let proposal_id = contract
        .create_proposal(
            "owner".to_string(),
            ProposalAction::TextProposal("Test".to_string()),
            100,
            1000,
        )
        .unwrap();

    contract.activate_proposal(proposal_id, 100).unwrap();

    contract
        .vote(proposal_id, "voter1".to_string(), VoteType::For, 60, 100)
        .unwrap();

    contract.finalize_voting(proposal_id, 1200).unwrap();
    contract.execute_proposal(proposal_id, 1300).unwrap();

    // Try to execute again
    let result = contract.execute_proposal(proposal_id, 1400);

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("already executed"));
}
