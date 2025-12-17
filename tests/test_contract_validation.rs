use rust_bc::contract_validation::*;
use std::collections::HashMap;

#[test]
fn test_validation_result_success() {
    let result = ValidationResult::success();
    assert!(result.valid);
    assert_eq!(result.severity, ValidationSeverity::Info);
    assert!(result.errors.is_empty());
}

#[test]
fn test_validation_result_failure() {
    let error = ValidationError::InsufficientBalance {
        required: 100,
        available: 50,
    };
    let result = ValidationResult::failure(error);
    assert!(!result.valid);
    assert_eq!(result.severity, ValidationSeverity::Critical);
    assert_eq!(result.errors.len(), 1);
}

#[test]
fn test_validation_result_with_warning() {
    let result = ValidationResult::success().with_warning("Test warning".to_string());
    assert!(result.valid);
    assert_eq!(result.severity, ValidationSeverity::Warning);
    assert_eq!(result.warnings.len(), 1);
}

#[test]
fn test_validation_result_with_metadata() {
    let result = ValidationResult::success()
        .with_metadata("key1".to_string(), "value1".to_string())
        .with_metadata("key2".to_string(), "value2".to_string());
    assert_eq!(result.metadata.len(), 2);
}

#[test]
fn test_state_validator_creation() {
    let validator = StateValidator::new(1000);
    assert_eq!(validator.total_supply, 1000);
    assert_eq!(validator.total_in_circulation, 0);
    assert_eq!(validator.total_locked, 0);
    assert_eq!(validator.total_reserved, 0);
}

#[test]
fn test_state_validator_balance_integrity_valid() {
    let validator = StateValidator {
        total_supply: 1000,
        total_in_circulation: 600,
        total_locked: 300,
        total_reserved: 100,
    };

    let result = validator.validate_balance_integrity();
    assert!(result.valid);
}

#[test]
fn test_state_validator_balance_integrity_mismatch() {
    let validator = StateValidator {
        total_supply: 1000,
        total_in_circulation: 600,
        total_locked: 300,
        total_reserved: 200, // Total = 1100 > 1000
    };

    let result = validator.validate_balance_integrity();
    assert!(!result.valid);
    assert_eq!(result.errors.len(), 1);
}

#[test]
fn test_state_validator_balance_integrity_leak_warning() {
    let validator = StateValidator {
        total_supply: 1000,
        total_in_circulation: 600,
        total_locked: 300,
        total_reserved: 50, // Total = 950 < 1000
    };

    let result = validator.validate_balance_integrity();
    assert!(result.valid);
    assert_eq!(result.severity, ValidationSeverity::Warning);
    assert!(!result.warnings.is_empty());
}

#[test]
fn test_state_validator_transfer_success() {
    let validator = StateValidator::new(1000);
    let result = validator.validate_transfer(500, 100);
    assert!(result.valid);
}

#[test]
fn test_state_validator_transfer_insufficient_balance() {
    let validator = StateValidator::new(1000);
    let result = validator.validate_transfer(50, 100);
    assert!(!result.valid);
}

#[test]
fn test_state_validator_transfer_zero_amount() {
    let validator = StateValidator::new(1000);
    let result = validator.validate_transfer(100, 0);
    assert!(!result.valid);
}

#[test]
fn test_rate_limiter_creation() {
    let limiter = RateLimiter::new(10, 1000);
    assert_eq!(limiter.max_ops_per_period, 10);
    assert_eq!(limiter.period_ms, 1000);
}

#[test]
fn test_rate_limiter_check_and_record_allowed() {
    let mut limiter = RateLimiter::new(5, 1000);

    for i in 0..5 {
        let result = limiter.check_and_record("transfer".to_string(), 100 + i * 10);
        assert_eq!(result.valid, true);
    }
}

#[test]
fn test_rate_limiter_check_and_record_exceeded() {
    let mut limiter = RateLimiter::new(2, 1000);

    let _r1 = limiter.check_and_record("transfer".to_string(), 100);
    let _r2 = limiter.check_and_record("transfer".to_string(), 110);

    let result = limiter.check_and_record("transfer".to_string(), 120);
    assert!(!result.valid);
}

#[test]
fn test_rate_limiter_cleanup_old_operations() {
    let mut limiter = RateLimiter::new(2, 100);

    let _r1 = limiter.check_and_record("transfer".to_string(), 100);
    let _r2 = limiter.check_and_record("transfer".to_string(), 110);

    // After 100ms window, old operation should be cleaned
    let result = limiter.check_and_record("transfer".to_string(), 250);
    assert!(result.valid); // Should be allowed because old operation is cleaned
}

#[test]
fn test_rate_limiter_current_count() {
    let mut limiter = RateLimiter::new(10, 1000);

    let _r1 = limiter.check_and_record("transfer".to_string(), 100);
    let _r2 = limiter.check_and_record("transfer".to_string(), 110);
    let _r3 = limiter.check_and_record("transfer".to_string(), 120);

    let count = limiter.current_count(150);
    assert_eq!(count, 3);
}

#[test]
fn test_access_control_grant_role() {
    let mut ac = AccessControl::new();
    ac.grant_role("admin".to_string(), "user1".to_string());

    assert!(ac.has_role("admin", "user1"));
}

#[test]
fn test_access_control_revoke_role() {
    let mut ac = AccessControl::new();
    ac.grant_role("admin".to_string(), "user1".to_string());
    ac.revoke_role("admin", "user1");

    assert!(!ac.has_role("admin", "user1"));
}

#[test]
fn test_access_control_validate_access_allowed() {
    let mut ac = AccessControl::new();
    ac.grant_role("admin".to_string(), "user1".to_string());

    let result = ac.validate_access("user1", "admin");
    assert!(result.valid);
}

#[test]
fn test_access_control_validate_access_denied() {
    let ac = AccessControl::new();

    let result = ac.validate_access("user1", "admin");
    assert!(!result.valid);
}

#[test]
fn test_transaction_validator_creation() {
    let validator = TransactionValidator::new(100, 10000);
    assert_eq!(validator.min_transaction_amount, 100);
    assert_eq!(validator.max_transaction_amount, 10000);
}

#[test]
fn test_transaction_validator_valid_amount() {
    let validator = TransactionValidator::new(100, 10000);
    let result = validator.validate_transaction(500, "recipient");
    assert!(result.valid);
}

#[test]
fn test_transaction_validator_amount_too_low() {
    let validator = TransactionValidator::new(100, 10000);
    let result = validator.validate_transaction(50, "recipient");
    assert!(!result.valid);
}

#[test]
fn test_transaction_validator_amount_too_high() {
    let validator = TransactionValidator::new(100, 10000);
    let result = validator.validate_transaction(20000, "recipient");
    assert!(!result.valid);
}

#[test]
fn test_audit_trail_creation() {
    let trail = AuditTrail::new(100);
    assert_eq!(trail.max_entries, 100);
    assert!(trail.entries.is_empty());
}

#[test]
fn test_audit_trail_record() {
    let mut trail = AuditTrail::new(100);
    let mut details = HashMap::new();
    details.insert("key".to_string(), "value".to_string());

    trail.record(1000, 100, "transfer".to_string(), "user1".to_string(), details, true);

    assert_eq!(trail.entries.len(), 1);
    assert_eq!(trail.entries[0].actor, "user1");
}

#[test]
fn test_audit_trail_max_entries() {
    let mut trail = AuditTrail::new(3);

    for i in 0..5 {
        let details = HashMap::new();
        trail.record(
            1000 + i,
            100 + i,
            "transfer".to_string(),
            format!("user{}", i),
            details,
            true,
        );
    }

    assert_eq!(trail.entries.len(), 3);
}

#[test]
fn test_audit_trail_get_actor_history() {
    let mut trail = AuditTrail::new(100);

    for i in 0..3 {
        let details = HashMap::new();
        trail.record(
            1000 + i,
            100 + i,
            "transfer".to_string(),
            "user1".to_string(),
            details,
            true,
        );
    }

    for i in 0..2 {
        let details = HashMap::new();
        trail.record(
            1100 + i,
            200 + i,
            "transfer".to_string(),
            "user2".to_string(),
            details,
            true,
        );
    }

    let user1_history = trail.get_actor_history("user1");
    assert_eq!(user1_history.len(), 3);

    let user2_history = trail.get_actor_history("user2");
    assert_eq!(user2_history.len(), 2);
}

#[test]
fn test_audit_trail_get_operation_history() {
    let mut trail = AuditTrail::new(100);

    let details = HashMap::new();
    trail.record(
        1000,
        100,
        "transfer".to_string(),
        "user1".to_string(),
        details.clone(),
        true,
    );

    let details = HashMap::new();
    trail.record(
        1100,
        200,
        "mint".to_string(),
        "user2".to_string(),
        details,
        true,
    );

    let transfer_ops = trail.get_operation_history("transfer");
    assert_eq!(transfer_ops.len(), 1);

    let mint_ops = trail.get_operation_history("mint");
    assert_eq!(mint_ops.len(), 1);
}

#[test]
fn test_audit_trail_get_failed_operations() {
    let mut trail = AuditTrail::new(100);

    let details = HashMap::new();
    trail.record(
        1000,
        100,
        "transfer".to_string(),
        "user1".to_string(),
        details.clone(),
        true,
    );

    let details = HashMap::new();
    trail.record(
        1100,
        200,
        "transfer".to_string(),
        "user2".to_string(),
        details,
        false,
    );

    let failed = trail.get_failed_operations();
    assert_eq!(failed.len(), 1);
    assert_eq!(failed[0].actor, "user2");
}

#[test]
fn test_dependency_graph_add_edge() {
    let mut graph = DependencyGraph::new();
    graph.add_edge("A".to_string(), "B".to_string());
    graph.add_edge("B".to_string(), "C".to_string());

    assert_eq!(graph.edges.get("A").unwrap().len(), 1);
}

#[test]
fn test_dependency_graph_no_cycle() {
    let mut graph = DependencyGraph::new();
    graph.add_edge("A".to_string(), "B".to_string());
    graph.add_edge("B".to_string(), "C".to_string());

    let result = graph.validate_no_cycles();
    assert!(result.valid);
}

#[test]
fn test_dependency_graph_cycle_detection() {
    let mut graph = DependencyGraph::new();
    graph.add_edge("A".to_string(), "B".to_string());
    graph.add_edge("B".to_string(), "C".to_string());
    graph.add_edge("C".to_string(), "A".to_string()); // Creates cycle

    let result = graph.validate_no_cycles();
    assert!(!result.valid);
}

#[test]
fn test_dependency_graph_would_create_cycle() {
    let mut graph = DependencyGraph::new();
    graph.add_edge("A".to_string(), "B".to_string());
    graph.add_edge("B".to_string(), "C".to_string());

    // D->A wouldn't create a cycle (D not in chain)
    assert!(!graph.would_create_cycle("D", "A"));
    // C->A would create a cycle since A->B->C exists
    assert!(graph.would_create_cycle("C", "A"));
    // B->A would create a cycle since A->B exists
    assert!(graph.would_create_cycle("B", "A"));
}

#[test]
fn test_comprehensive_validator_creation() {
    let validator = ComprehensiveValidator::new(10000, 100, 1000, 10, 5000);
    assert_eq!(validator.state_validator.total_supply, 10000);
    assert_eq!(validator.rate_limiter.max_ops_per_period, 100);
    assert_eq!(validator.transaction_validator.min_transaction_amount, 10);
}

#[test]
fn test_comprehensive_validator_no_role() {
    let mut validator = ComprehensiveValidator::new(10000, 100, 1000, 10, 5000);

    let result = validator.validate_transaction_comprehensive(
        "user1", "recipient", 100, "admin", 1000, 100,
    );

    assert!(!result.valid);
}

#[test]
fn test_comprehensive_validator_with_role() {
    let mut validator = ComprehensiveValidator::new(10000, 100, 1000, 10, 5000);
    validator
        .access_control
        .grant_role("user".to_string(), "user1".to_string());

    let result = validator.validate_transaction_comprehensive(
        "user1", "recipient", 100, "user", 1000, 100,
    );

    assert!(result.valid);
}

#[test]
fn test_comprehensive_validator_rate_limited() {
    let mut validator = ComprehensiveValidator::new(10000, 2, 1000, 10, 5000);
    validator
        .access_control
        .grant_role("user".to_string(), "user1".to_string());

    // Fill rate limit
    let _r1 = validator.validate_transaction_comprehensive(
        "user1", "recipient", 100, "user", 1000, 100,
    );

    let _r2 = validator.validate_transaction_comprehensive(
        "user1", "recipient", 100, "user", 1010, 100,
    );

    // This should exceed rate limit
    let result = validator.validate_transaction_comprehensive(
        "user1", "recipient", 100, "user", 1020, 100,
    );

    assert!(!result.valid);
}

#[test]
fn test_comprehensive_validator_invalid_amount() {
    let mut validator = ComprehensiveValidator::new(10000, 100, 1000, 100, 5000);
    validator
        .access_control
        .grant_role("user".to_string(), "user1".to_string());

    let result = validator.validate_transaction_comprehensive(
        "user1", "recipient", 50, "user", 1000, 100, // 50 < min 100
    );

    assert!(!result.valid);
}

#[test]
fn test_comprehensive_validator_audit_trail_recorded() {
    let mut validator = ComprehensiveValidator::new(10000, 100, 1000, 10, 5000);
    validator
        .access_control
        .grant_role("user".to_string(), "user1".to_string());

    validator.validate_transaction_comprehensive(
        "user1", "recipient", 100, "user", 1000, 100,
    );

    let history = validator.audit_trail.get_actor_history("user1");
    assert_eq!(history.len(), 1);
}

#[test]
fn test_validation_severity_ordering() {
    assert!(ValidationSeverity::Info < ValidationSeverity::Warning);
    assert!(ValidationSeverity::Warning < ValidationSeverity::Critical);
    assert!(ValidationSeverity::Info < ValidationSeverity::Critical);
}

#[test]
fn test_state_validator_update_state() {
    let mut validator = StateValidator {
        total_supply: 1000,
        total_in_circulation: 600,
        total_locked: 300,
        total_reserved: 100,
    };

    let result = validator.update_state(100, -100, 0);
    assert!(result.valid);
}

#[test]
fn test_multiple_role_grants() {
    let mut ac = AccessControl::new();
    ac.grant_role("admin".to_string(), "user1".to_string());
    ac.grant_role("admin".to_string(), "user2".to_string());
    ac.grant_role("user".to_string(), "user1".to_string());

    assert!(ac.has_role("admin", "user1"));
    assert!(ac.has_role("admin", "user2"));
    assert!(ac.has_role("user", "user1"));
    assert!(!ac.has_role("admin", "user3"));
}

#[test]
fn test_security_report_generation() {
    let validator = ComprehensiveValidator::new(10000, 100, 1000, 10, 5000);
    let report = validator.generate_security_report();

    assert!(report.state_integrity_valid);
    assert!(report.no_circular_dependencies);
    assert_eq!(report.failed_operations_count, 0);
}

#[test]
fn test_transaction_validator_with_whitelist() {
    let mut validator = TransactionValidator::new(100, 10000);
    validator.allowed_recipients_whitelist = Some(vec!["recipient1".to_string(), "recipient2".to_string()]);

    let result_allowed = validator.validate_transaction(500, "recipient1");
    assert!(result_allowed.valid);

    let result_denied = validator.validate_transaction(500, "recipient3");
    assert!(!result_denied.valid);
}

#[test]
fn test_comprehensive_dependency_checking() {
    let mut graph = DependencyGraph::new();
    graph.add_edge("contract_a".to_string(), "contract_b".to_string());
    graph.add_edge("contract_b".to_string(), "contract_c".to_string());
    graph.add_edge("contract_c".to_string(), "contract_d".to_string());

    let result = graph.validate_no_cycles();
    assert!(result.valid);

    // Adding a back edge would create a cycle
    let would_cycle = graph.would_create_cycle("contract_d", "contract_a");
    assert!(would_cycle);
}

#[test]
fn test_rate_limiter_multiple_operation_types() {
    let mut limiter = RateLimiter::new(5, 1000);

    let _r1 = limiter.check_and_record("transfer".to_string(), 100);
    let _r2 = limiter.check_and_record("mint".to_string(), 110);
    let _r3 = limiter.check_and_record("burn".to_string(), 120);

    assert_eq!(limiter.operations.len(), 3);
}
