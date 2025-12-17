/**
 * Advanced Contract Validation Framework
 *
 * Enhanced security validation for smart contracts:
 * - State consistency checks
 * - Operation limit enforcement
 * - Risk assessment
 * - Audit trail
 */

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Validation rule severity levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ValidationSeverity {
    Info,
    Warning,
    Critical,
}

/// Validation error types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ValidationError {
    BalanceMismatch { expected: u64, actual: u64 },
    InsufficientBalance { required: u64, available: u64 },
    OperationLimitExceeded { limit: u64, current: u64 },
    UnauthorizedAccess { caller: String, required_role: String },
    InvalidTransition { from_state: String, to_state: String },
    RateLimitExceeded { limit: u64, period_ms: u64 },
    CircularDependency { from: String, to: String },
    InvalidAmount { amount: u64, min: u64, max: u64 },
    ExternalDataMismatch { source: String, expected: String, actual: String },
    Custom { code: u32, message: String },
}

/// Validation result with detailed information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub severity: ValidationSeverity,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<String>,
    pub metadata: HashMap<String, String>,
}

impl ValidationResult {
    pub fn success() -> Self {
        ValidationResult {
            valid: true,
            severity: ValidationSeverity::Info,
            errors: Vec::new(),
            warnings: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn failure(error: ValidationError) -> Self {
        ValidationResult {
            valid: false,
            severity: ValidationSeverity::Critical,
            errors: vec![error],
            warnings: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_warning(mut self, warning: String) -> Self {
        if self.severity < ValidationSeverity::Warning {
            self.severity = ValidationSeverity::Warning;
        }
        self.warnings.push(warning);
        self
    }

    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// State consistency validator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateValidator {
    pub total_supply: u64,
    pub total_in_circulation: u64,
    pub total_locked: u64,
    pub total_reserved: u64,
}

impl StateValidator {
    pub fn new(total_supply: u64) -> Self {
        StateValidator {
            total_supply,
            total_in_circulation: 0,
            total_locked: 0,
            total_reserved: 0,
        }
    }

    /// Validate that all tokens are accounted for
    pub fn validate_balance_integrity(&self) -> ValidationResult {
        let accounted = self.total_in_circulation + self.total_locked + self.total_reserved;

        if accounted == self.total_supply {
            ValidationResult::success()
        } else if accounted > self.total_supply {
            ValidationResult::failure(ValidationError::BalanceMismatch {
                expected: self.total_supply,
                actual: accounted,
            })
        } else {
            let leaked = self.total_supply - accounted;
            ValidationResult::success().with_warning(format!(
                "Unaccounted tokens detected: {} tokens not tracked",
                leaked
            ))
        }
    }

    /// Validate a transfer operation
    pub fn validate_transfer(
        &self,
        from_balance: u64,
        amount: u64,
    ) -> ValidationResult {
        if amount == 0 {
            return ValidationResult::failure(ValidationError::InvalidAmount {
                amount: 0,
                min: 1,
                max: u64::MAX,
            });
        }

        if from_balance < amount {
            return ValidationResult::failure(ValidationError::InsufficientBalance {
                required: amount,
                available: from_balance,
            });
        }

        ValidationResult::success()
    }

    /// Update state after successful operation
    pub fn update_state(
        &mut self,
        circulation_delta: i64,
        locked_delta: i64,
        reserved_delta: i64,
    ) -> ValidationResult {
        let new_circulation = (self.total_in_circulation as i64 + circulation_delta) as u64;
        let new_locked = (self.total_locked as i64 + locked_delta) as u64;
        let new_reserved = (self.total_reserved as i64 + reserved_delta) as u64;

        let validator = StateValidator {
            total_supply: self.total_supply,
            total_in_circulation: new_circulation,
            total_locked: new_locked,
            total_reserved: new_reserved,
        };

        validator.validate_balance_integrity()
    }
}

/// Operation rate limiter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimiter {
    pub operations: Vec<(u64, String)>, // (timestamp_ms, operation_type)
    pub max_ops_per_period: u64,
    pub period_ms: u64,
}

impl RateLimiter {
    pub fn new(max_ops_per_period: u64, period_ms: u64) -> Self {
        RateLimiter {
            operations: Vec::new(),
            max_ops_per_period,
            period_ms,
        }
    }

    /// Check if operation is allowed and record it
    pub fn check_and_record(
        &mut self,
        op_type: String,
        current_time_ms: u64,
    ) -> ValidationResult {
        // Clean up old operations
        let cutoff_time = current_time_ms.saturating_sub(self.period_ms);
        self.operations.retain(|(timestamp, _)| *timestamp > cutoff_time);

        // Check if limit exceeded
        if self.operations.len() >= self.max_ops_per_period as usize {
            return ValidationResult::failure(ValidationError::RateLimitExceeded {
                limit: self.max_ops_per_period,
                period_ms: self.period_ms,
            });
        }

        // Record operation
        self.operations.push((current_time_ms, op_type));

        ValidationResult::success()
    }

    /// Get current operation count in window
    pub fn current_count(&self, current_time_ms: u64) -> u64 {
        let cutoff_time = current_time_ms.saturating_sub(self.period_ms);
        self.operations
            .iter()
            .filter(|(timestamp, _)| *timestamp > cutoff_time)
            .count() as u64
    }
}

/// Access control validator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessControl {
    pub roles: HashMap<String, Vec<String>>, // role -> list of addresses
    pub permissions: HashMap<String, Vec<String>>, // address -> list of permissions
}

impl AccessControl {
    pub fn new() -> Self {
        AccessControl {
            roles: HashMap::new(),
            permissions: HashMap::new(),
        }
    }

    /// Grant role to address
    pub fn grant_role(&mut self, role: String, address: String) {
        self.roles
            .entry(role)
            .or_insert_with(Vec::new)
            .push(address);
    }

    /// Revoke role from address
    pub fn revoke_role(&mut self, role: &str, address: &str) {
        if let Some(addresses) = self.roles.get_mut(role) {
            addresses.retain(|a| a != address);
        }
    }

    /// Check if address has role
    pub fn has_role(&self, role: &str, address: &str) -> bool {
        self.roles
            .get(role)
            .map(|addresses| addresses.contains(&address.to_string()))
            .unwrap_or(false)
    }

    /// Validate access
    pub fn validate_access(
        &self,
        caller: &str,
        required_role: &str,
    ) -> ValidationResult {
        if self.has_role(required_role, caller) {
            ValidationResult::success()
        } else {
            ValidationResult::failure(ValidationError::UnauthorizedAccess {
                caller: caller.to_string(),
                required_role: required_role.to_string(),
            })
        }
    }
}

/// Transaction validator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionValidator {
    pub max_transaction_amount: u64,
    pub min_transaction_amount: u64,
    pub allowed_recipients_whitelist: Option<Vec<String>>,
}

impl TransactionValidator {
    pub fn new(min: u64, max: u64) -> Self {
        TransactionValidator {
            max_transaction_amount: max,
            min_transaction_amount: min,
            allowed_recipients_whitelist: None,
        }
    }

    /// Validate transaction parameters
    pub fn validate_transaction(
        &self,
        amount: u64,
        recipient: &str,
    ) -> ValidationResult {
        if amount < self.min_transaction_amount || amount > self.max_transaction_amount {
            return ValidationResult::failure(ValidationError::InvalidAmount {
                amount,
                min: self.min_transaction_amount,
                max: self.max_transaction_amount,
            });
        }

        if let Some(ref whitelist) = self.allowed_recipients_whitelist {
            if !whitelist.contains(&recipient.to_string()) {
                return ValidationResult::failure(ValidationError::UnauthorizedAccess {
                    caller: recipient.to_string(),
                    required_role: "whitelisted_recipient".to_string(),
                });
            }
        }

        ValidationResult::success()
    }
}

/// Audit trail for tracking operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditTrail {
    pub entries: Vec<AuditEntry>,
    pub max_entries: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp: u64,
    pub block: u64,
    pub operation: String,
    pub actor: String,
    pub details: HashMap<String, String>,
    pub result: bool,
}

impl AuditTrail {
    pub fn new(max_entries: u64) -> Self {
        AuditTrail {
            entries: Vec::new(),
            max_entries,
        }
    }

    /// Record an operation
    pub fn record(
        &mut self,
        timestamp: u64,
        block: u64,
        operation: String,
        actor: String,
        details: HashMap<String, String>,
        result: bool,
    ) {
        let entry = AuditEntry {
            timestamp,
            block,
            operation,
            actor,
            details,
            result,
        };

        self.entries.push(entry);

        // Maintain max size
        if self.entries.len() > self.max_entries as usize {
            self.entries.remove(0);
        }
    }

    /// Get entries for specific actor
    pub fn get_actor_history(&self, actor: &str) -> Vec<AuditEntry> {
        self.entries
            .iter()
            .filter(|e| e.actor == actor)
            .cloned()
            .collect()
    }

    /// Get entries for specific operation type
    pub fn get_operation_history(&self, operation: &str) -> Vec<AuditEntry> {
        self.entries
            .iter()
            .filter(|e| e.operation == operation)
            .cloned()
            .collect()
    }

    /// Get failed operations
    pub fn get_failed_operations(&self) -> Vec<AuditEntry> {
        self.entries
            .iter()
            .filter(|e| !e.result)
            .cloned()
            .collect()
    }
}

/// Dependency graph for detecting circular references
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyGraph {
    pub edges: HashMap<String, Vec<String>>, // from -> [to]
}

impl DependencyGraph {
    pub fn new() -> Self {
        DependencyGraph {
            edges: HashMap::new(),
        }
    }

    /// Add edge from -> to
    pub fn add_edge(&mut self, from: String, to: String) {
        self.edges.entry(from).or_insert_with(Vec::new).push(to);
    }

    /// Detect if adding edge would create cycle
    pub fn would_create_cycle(&self, from: &str, to: &str) -> bool {
        // Simple cycle detection: if 'to' can reach 'from', adding from->to creates cycle
        self.can_reach(to, from)
    }

    /// Check if 'from' can reach 'to'
    fn can_reach(&self, from: &str, to: &str) -> bool {
        if from == to {
            return true;
        }

        if let Some(neighbors) = self.edges.get(from) {
            for neighbor in neighbors {
                if self.can_reach(neighbor, to) {
                    return true;
                }
            }
        }

        false
    }

    /// Validate no cycles exist
    pub fn validate_no_cycles(&self) -> ValidationResult {
        for (from, neighbors) in &self.edges {
            for to in neighbors {
                // Check if 'to' can reach back to 'from', which would create a cycle
                if from != to && self.can_reach(to, from) {
                    return ValidationResult::failure(ValidationError::CircularDependency {
                        from: from.clone(),
                        to: to.clone(),
                    });
                }
            }
        }

        ValidationResult::success()
    }
}

/// Comprehensive validator that combines all checks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComprehensiveValidator {
    pub state_validator: StateValidator,
    pub rate_limiter: RateLimiter,
    pub access_control: AccessControl,
    pub transaction_validator: TransactionValidator,
    pub audit_trail: AuditTrail,
    pub dependency_graph: DependencyGraph,
}

impl ComprehensiveValidator {
    pub fn new(
        total_supply: u64,
        max_ops: u64,
        rate_limit_period_ms: u64,
        min_tx: u64,
        max_tx: u64,
    ) -> Self {
        ComprehensiveValidator {
            state_validator: StateValidator::new(total_supply),
            rate_limiter: RateLimiter::new(max_ops, rate_limit_period_ms),
            access_control: AccessControl::new(),
            transaction_validator: TransactionValidator::new(min_tx, max_tx),
            audit_trail: AuditTrail::new(1000),
            dependency_graph: DependencyGraph::new(),
        }
    }

    /// Run all validations for a transaction
    pub fn validate_transaction_comprehensive(
        &mut self,
        caller: &str,
        recipient: &str,
        amount: u64,
        required_role: &str,
        current_time_ms: u64,
        current_block: u64,
    ) -> ValidationResult {
        // Check access control
        if !self.access_control.has_role(required_role, caller) {
            return ValidationResult::failure(ValidationError::UnauthorizedAccess {
                caller: caller.to_string(),
                required_role: required_role.to_string(),
            });
        }

        // Check rate limiting
        let rate_result = self
            .rate_limiter
            .check_and_record("transfer".to_string(), current_time_ms);
        if !rate_result.valid {
            return rate_result;
        }

        // Check transaction validity
        let tx_result = self.transaction_validator.validate_transaction(amount, recipient);
        if !tx_result.valid {
            return tx_result;
        }

        // Record in audit trail
        let mut details = HashMap::new();
        details.insert("recipient".to_string(), recipient.to_string());
        details.insert("amount".to_string(), amount.to_string());

        self.audit_trail.record(
            current_time_ms,
            current_block,
            "transfer".to_string(),
            caller.to_string(),
            details,
            true,
        );

        ValidationResult::success()
    }

    /// Get security report
    pub fn generate_security_report(&self) -> SecurityReport {
        let state_integrity = self.state_validator.validate_balance_integrity();
        let no_cycles = self.dependency_graph.validate_no_cycles();
        let failed_ops = self.audit_trail.get_failed_operations();

        SecurityReport {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            state_integrity_valid: state_integrity.valid,
            no_circular_dependencies: no_cycles.valid,
            failed_operations_count: failed_ops.len() as u64,
            total_audit_entries: self.audit_trail.entries.len() as u64,
            current_rate_limit_usage: self.rate_limiter.current_count(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
            ),
        }
    }
}

/// Security report summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityReport {
    pub timestamp: u64,
    pub state_integrity_valid: bool,
    pub no_circular_dependencies: bool,
    pub failed_operations_count: u64,
    pub total_audit_entries: u64,
    pub current_rate_limit_usage: u64,
}
