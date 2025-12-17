use rust_bc::multisig_contracts::*;

#[test]
fn test_multisig_contract_creation() {
    let members = vec![
        "member1".to_string(),
        "member2".to_string(),
        "member3".to_string(),
    ];

    let contract = MultiSigContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        members.clone(),
        2,
        1000,
    )
    .unwrap();

    assert_eq!(contract.contract_address, "0x123");
    assert_eq!(contract.owner, "owner");
    assert_eq!(contract.members, members);
    assert_eq!(contract.signature_threshold, 2);
}

#[test]
fn test_multisig_contract_invalid_threshold() {
    let members = vec!["member1".to_string(), "member2".to_string()];

    let result = MultiSigContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        members,
        3, // Threshold higher than members
        1000,
    );

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Threshold"));
}

#[test]
fn test_multisig_contract_empty_members() {
    let members = vec![];

    let result = MultiSigContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        members,
        1,
        1000,
    );

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("at least one member"));
}

#[test]
fn test_propose_operation() {
    let members = vec!["member1".to_string(), "member2".to_string()];

    let mut contract = MultiSigContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        members,
        2,
        1000,
    )
    .unwrap();

    let op_id = contract
        .propose_operation(
            "owner",
            MultiSigOperation::Transfer {
                to: "recipient".to_string(),
                amount: 100,
            },
            100,
        )
        .unwrap();

    assert_eq!(op_id, 1);
    assert_eq!(contract.next_operation_id, 2);

    let operation = contract.get_operation(op_id).unwrap();
    assert_eq!(operation.id, 1);
    assert!(!operation.executed);
}

#[test]
fn test_propose_operation_not_owner() {
    let members = vec!["member1".to_string(), "member2".to_string()];

    let mut contract = MultiSigContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        members,
        2,
        1000,
    )
    .unwrap();

    let result = contract.propose_operation(
        "not_owner",
        MultiSigOperation::Transfer {
            to: "recipient".to_string(),
            amount: 100,
        },
        100,
    );

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Only owner can propose"));
}

#[test]
fn test_sign_operation() {
    let members = vec!["member1".to_string(), "member2".to_string()];

    let mut contract = MultiSigContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        members,
        2,
        1000,
    )
    .unwrap();

    let op_id = contract
        .propose_operation(
            "owner",
            MultiSigOperation::Transfer {
                to: "recipient".to_string(),
                amount: 100,
            },
            100,
        )
        .unwrap();

    contract
        .sign_operation(op_id, "member1".to_string(), "sig1".to_string(), 100)
        .unwrap();

    let operation = contract.get_operation(op_id).unwrap();
    assert_eq!(operation.signatures.len(), 1);
    assert!(operation.signatures.contains_key("member1"));
}

#[test]
fn test_sign_operation_non_member() {
    let members = vec!["member1".to_string(), "member2".to_string()];

    let mut contract = MultiSigContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        members,
        2,
        1000,
    )
    .unwrap();

    let op_id = contract
        .propose_operation(
            "owner",
            MultiSigOperation::Transfer {
                to: "recipient".to_string(),
                amount: 100,
            },
            100,
        )
        .unwrap();

    let result = contract.sign_operation(op_id, "non_member".to_string(), "sig1".to_string(), 100);

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not a member"));
}

#[test]
fn test_sign_operation_duplicate() {
    let members = vec!["member1".to_string(), "member2".to_string()];

    let mut contract = MultiSigContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        members,
        2,
        1000,
    )
    .unwrap();

    let op_id = contract
        .propose_operation(
            "owner",
            MultiSigOperation::Transfer {
                to: "recipient".to_string(),
                amount: 100,
            },
            100,
        )
        .unwrap();

    contract
        .sign_operation(op_id, "member1".to_string(), "sig1".to_string(), 100)
        .unwrap();

    let result = contract.sign_operation(op_id, "member1".to_string(), "sig2".to_string(), 100);

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("already signed"));
}

#[test]
fn test_execute_operation_success() {
    let members = vec!["member1".to_string(), "member2".to_string()];

    let mut contract = MultiSigContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        members,
        2,
        1000,
    )
    .unwrap();

    let op_id = contract
        .propose_operation(
            "owner",
            MultiSigOperation::Transfer {
                to: "recipient".to_string(),
                amount: 100,
            },
            100,
        )
        .unwrap();

    contract
        .sign_operation(op_id, "member1".to_string(), "sig1".to_string(), 100)
        .unwrap();
    contract
        .sign_operation(op_id, "member2".to_string(), "sig2".to_string(), 100)
        .unwrap();

    let result = contract.execute_operation(op_id, 100).unwrap();
    assert!(result.contains("Transfer executed"));

    let operation = contract.get_operation(op_id).unwrap();
    assert!(operation.executed);
    assert_eq!(contract.executed_count, 1);
}

#[test]
fn test_execute_operation_insufficient_signatures() {
    let members = vec!["member1".to_string(), "member2".to_string()];

    let mut contract = MultiSigContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        members,
        2,
        1000,
    )
    .unwrap();

    let op_id = contract
        .propose_operation(
            "owner",
            MultiSigOperation::Transfer {
                to: "recipient".to_string(),
                amount: 100,
            },
            100,
        )
        .unwrap();

    contract
        .sign_operation(op_id, "member1".to_string(), "sig1".to_string(), 100)
        .unwrap();

    let result = contract.execute_operation(op_id, 100);

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Not enough signatures"));
}

#[test]
fn test_execute_operation_expired() {
    let members = vec!["member1".to_string(), "member2".to_string()];

    let mut contract = MultiSigContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        members,
        2,
        1000,
    )
    .unwrap();

    let op_id = contract
        .propose_operation(
            "owner",
            MultiSigOperation::Transfer {
                to: "recipient".to_string(),
                amount: 100,
            },
            100,
        )
        .unwrap();

    contract
        .sign_operation(op_id, "member1".to_string(), "sig1".to_string(), 100)
        .unwrap();
    contract
        .sign_operation(op_id, "member2".to_string(), "sig2".to_string(), 100)
        .unwrap();

    let result = contract.execute_operation(op_id, 1200); // Past timeout

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("expired"));
}

#[test]
fn test_revoke_signature() {
    let members = vec!["member1".to_string(), "member2".to_string()];

    let mut contract = MultiSigContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        members,
        2,
        1000,
    )
    .unwrap();

    let op_id = contract
        .propose_operation(
            "owner",
            MultiSigOperation::Transfer {
                to: "recipient".to_string(),
                amount: 100,
            },
            100,
        )
        .unwrap();

    contract
        .sign_operation(op_id, "member1".to_string(), "sig1".to_string(), 100)
        .unwrap();

    contract.revoke_signature(op_id, "member1").unwrap();

    let operation = contract.get_operation(op_id).unwrap();
    assert_eq!(operation.signatures.len(), 0);
}

#[test]
fn test_revoke_signature_not_signed() {
    let members = vec!["member1".to_string(), "member2".to_string()];

    let mut contract = MultiSigContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        members,
        2,
        1000,
    )
    .unwrap();

    let op_id = contract
        .propose_operation(
            "owner",
            MultiSigOperation::Transfer {
                to: "recipient".to_string(),
                amount: 100,
            },
            100,
        )
        .unwrap();

    let result = contract.revoke_signature(op_id, "member1");

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("has not signed"));
}

#[test]
fn test_add_member() {
    let members = vec!["member1".to_string(), "member2".to_string()];

    let mut contract = MultiSigContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        members,
        2,
        1000,
    )
    .unwrap();

    contract.add_member("owner", "member3".to_string()).unwrap();

    assert_eq!(contract.members.len(), 3);
    assert!(contract.is_member("member3"));
}

#[test]
fn test_add_member_not_owner() {
    let members = vec!["member1".to_string(), "member2".to_string()];

    let mut contract = MultiSigContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        members,
        2,
        1000,
    )
    .unwrap();

    let result = contract.add_member("member1", "member3".to_string());

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Only owner can add"));
}

#[test]
fn test_add_member_duplicate() {
    let members = vec!["member1".to_string(), "member2".to_string()];

    let mut contract = MultiSigContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        members,
        2,
        1000,
    )
    .unwrap();

    let result = contract.add_member("owner", "member1".to_string());

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("already exists"));
}

#[test]
fn test_remove_member() {
    let members = vec!["member1".to_string(), "member2".to_string()];

    let mut contract = MultiSigContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        members,
        2,
        1000,
    )
    .unwrap();

    contract.remove_member("owner", "member1").unwrap();

    assert_eq!(contract.members.len(), 1);
    assert!(!contract.is_member("member1"));
}

#[test]
fn test_remove_member_owner() {
    let members = vec!["owner".to_string(), "member1".to_string()];

    let mut contract = MultiSigContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        members,
        2,
        1000,
    )
    .unwrap();

    let result = contract.remove_member("owner", "owner");

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Cannot remove owner"));
}

#[test]
fn test_change_threshold() {
    let members = vec!["member1".to_string(), "member2".to_string(), "member3".to_string()];

    let mut contract = MultiSigContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        members,
        2,
        1000,
    )
    .unwrap();

    contract.change_threshold("owner", 3).unwrap();

    assert_eq!(contract.signature_threshold, 3);
}

#[test]
fn test_change_threshold_not_owner() {
    let members = vec!["member1".to_string(), "member2".to_string()];

    let mut contract = MultiSigContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        members,
        2,
        1000,
    )
    .unwrap();

    let result = contract.change_threshold("member1", 2);

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Only owner can change"));
}

#[test]
fn test_change_threshold_invalid() {
    let members = vec!["member1".to_string(), "member2".to_string()];

    let mut contract = MultiSigContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        members,
        2,
        1000,
    )
    .unwrap();

    let result = contract.change_threshold("owner", 5);

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Threshold"));
}

#[test]
fn test_cleanup_expired() {
    let members = vec!["member1".to_string(), "member2".to_string()];

    let mut contract = MultiSigContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        members,
        2,
        100,
    )
    .unwrap();

    let op_id1 = contract
        .propose_operation(
            "owner",
            MultiSigOperation::Transfer {
                to: "recipient".to_string(),
                amount: 100,
            },
            100,
        )
        .unwrap();

    let op_id2 = contract
        .propose_operation(
            "owner",
            MultiSigOperation::Transfer {
                to: "recipient".to_string(),
                amount: 200,
            },
            100,
        )
        .unwrap();

    // Sign and execute first operation before timeout
    contract
        .sign_operation(op_id1, "member1".to_string(), "sig1".to_string(), 100)
        .unwrap();
    contract
        .sign_operation(op_id1, "member2".to_string(), "sig2".to_string(), 100)
        .unwrap();
    contract.execute_operation(op_id1, 100).unwrap();

    // Clean up expired operations
    let removed = contract.cleanup_expired(250);

    assert_eq!(removed, 1); // Only op_id2 should be removed
    assert!(contract.get_operation(op_id1).is_some()); // op_id1 still there (executed)
    assert!(contract.get_operation(op_id2).is_none()); // op_id2 removed
}

#[test]
fn test_config_update_operation() {
    let members = vec!["member1".to_string(), "member2".to_string()];

    let mut contract = MultiSigContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        members,
        2,
        1000,
    )
    .unwrap();

    let op_id = contract
        .propose_operation(
            "owner",
            MultiSigOperation::ConfigUpdate {
                key: "rate".to_string(),
                value: "5".to_string(),
            },
            100,
        )
        .unwrap();

    contract
        .sign_operation(op_id, "member1".to_string(), "sig1".to_string(), 100)
        .unwrap();
    contract
        .sign_operation(op_id, "member2".to_string(), "sig2".to_string(), 100)
        .unwrap();

    let result = contract.execute_operation(op_id, 100).unwrap();
    assert!(result.contains("Config updated"));
    assert!(result.contains("rate"));
}

#[test]
fn test_member_add_operation() {
    let members = vec!["member1".to_string(), "member2".to_string()];

    let mut contract = MultiSigContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        members,
        2,
        1000,
    )
    .unwrap();

    let op_id = contract
        .propose_operation(
            "owner",
            MultiSigOperation::MemberAdd {
                address: "member3".to_string(),
            },
            100,
        )
        .unwrap();

    contract
        .sign_operation(op_id, "member1".to_string(), "sig1".to_string(), 100)
        .unwrap();
    contract
        .sign_operation(op_id, "member2".to_string(), "sig2".to_string(), 100)
        .unwrap();

    let result = contract.execute_operation(op_id, 100).unwrap();
    assert!(result.contains("Member added"));
}

#[test]
fn test_custom_operation() {
    let members = vec!["member1".to_string(), "member2".to_string()];

    let mut contract = MultiSigContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        members,
        2,
        1000,
    )
    .unwrap();

    let op_id = contract
        .propose_operation(
            "owner",
            MultiSigOperation::Custom {
                op_type: "deploy_contract".to_string(),
                params: vec!["bytecode".to_string(), "0x789".to_string()],
            },
            100,
        )
        .unwrap();

    contract
        .sign_operation(op_id, "member1".to_string(), "sig1".to_string(), 100)
        .unwrap();
    contract
        .sign_operation(op_id, "member2".to_string(), "sig2".to_string(), 100)
        .unwrap();

    let result = contract.execute_operation(op_id, 100).unwrap();
    assert!(result.contains("Custom operation executed"));
    assert!(result.contains("deploy_contract"));
}

#[test]
fn test_get_pending_operations() {
    let members = vec!["member1".to_string(), "member2".to_string()];

    let mut contract = MultiSigContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        members,
        2,
        1000,
    )
    .unwrap();

    let op_id1 = contract
        .propose_operation(
            "owner",
            MultiSigOperation::Transfer {
                to: "recipient1".to_string(),
                amount: 100,
            },
            100,
        )
        .unwrap();

    let op_id2 = contract
        .propose_operation(
            "owner",
            MultiSigOperation::Transfer {
                to: "recipient2".to_string(),
                amount: 200,
            },
            100,
        )
        .unwrap();

    // Execute first operation
    contract
        .sign_operation(op_id1, "member1".to_string(), "sig1".to_string(), 100)
        .unwrap();
    contract
        .sign_operation(op_id1, "member2".to_string(), "sig2".to_string(), 100)
        .unwrap();
    contract.execute_operation(op_id1, 100).unwrap();

    let pending = contract.get_pending_operations();

    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].id, op_id2);
}

#[test]
fn test_get_statistics() {
    let members = vec!["member1".to_string(), "member2".to_string(), "member3".to_string()];

    let mut contract = MultiSigContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        members,
        2,
        1000,
    )
    .unwrap();

    let op_id = contract
        .propose_operation(
            "owner",
            MultiSigOperation::Transfer {
                to: "recipient".to_string(),
                amount: 100,
            },
            100,
        )
        .unwrap();

    contract
        .sign_operation(op_id, "member1".to_string(), "sig1".to_string(), 100)
        .unwrap();
    contract
        .sign_operation(op_id, "member2".to_string(), "sig2".to_string(), 100)
        .unwrap();

    contract.execute_operation(op_id, 100).unwrap();

    let stats = contract.get_statistics();

    assert_eq!(stats.members, 3);
    assert_eq!(stats.threshold, 2);
    assert_eq!(stats.total_operations, 1);
    assert_eq!(stats.executed_operations, 1);
    assert_eq!(stats.pending_operations, 0);
}

#[test]
fn test_threshold_adjustment_on_member_removal() {
    let members = vec!["member1".to_string(), "member2".to_string(), "member3".to_string()];

    let mut contract = MultiSigContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        members,
        3,
        1000,
    )
    .unwrap();

    contract.remove_member("owner", "member1").unwrap();
    contract.remove_member("owner", "member2").unwrap();

    // Threshold should be adjusted to 1 since only 1 member left
    assert_eq!(contract.signature_threshold, 1);
}

#[test]
fn test_signature_expiry() {
    let members = vec!["member1".to_string(), "member2".to_string()];

    let mut contract = MultiSigContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        members,
        2,
        100,
    )
    .unwrap();

    let op_id = contract
        .propose_operation(
            "owner",
            MultiSigOperation::Transfer {
                to: "recipient".to_string(),
                amount: 100,
            },
            100,
        )
        .unwrap();

    contract
        .sign_operation(op_id, "member1".to_string(), "sig1".to_string(), 100)
        .unwrap();

    // Try to sign after operation expiration
    let result = contract.sign_operation(op_id, "member2".to_string(), "sig2".to_string(), 300);

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("expired"));
}

#[test]
fn test_execute_already_executed() {
    let members = vec!["member1".to_string(), "member2".to_string()];

    let mut contract = MultiSigContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        members,
        2,
        1000,
    )
    .unwrap();

    let op_id = contract
        .propose_operation(
            "owner",
            MultiSigOperation::Transfer {
                to: "recipient".to_string(),
                amount: 100,
            },
            100,
        )
        .unwrap();

    contract
        .sign_operation(op_id, "member1".to_string(), "sig1".to_string(), 100)
        .unwrap();
    contract
        .sign_operation(op_id, "member2".to_string(), "sig2".to_string(), 100)
        .unwrap();

    contract.execute_operation(op_id, 100).unwrap();

    // Try to execute again
    let result = contract.execute_operation(op_id, 100);

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("already executed"));
}
