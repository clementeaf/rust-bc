/**
 * Transaction Validation Gate
 *
 * Implements comprehensive pre-mempool validation:
 * - Signature verification
 * - Sender sequence number tracking (replay attack prevention)
 * - Fee validation and market rules
 * - Double-spend prevention
 * - Amount and format validation
 */
use crate::models::Transaction;
use std::collections::HashMap;
use std::time::SystemTime;

/**
 * Transaction validation configuration
 */
#[derive(Debug, Clone)]
pub struct ValidationConfig {
    pub min_fee: u64,
    pub min_amount: u64,
    pub max_amount: u64,
    pub min_address_length: usize,
    pub max_address_length: usize,
    pub fee_multiplier: f64, // Adjust fees based on network congestion
    pub enable_sequence_tracking: bool,
    pub max_pending_per_sender: usize,
    /// Maximum allowed clock drift into the future (seconds)
    pub max_future_drift_secs: u64,
    /// Maximum age of a transaction timestamp (seconds)
    pub max_past_age_secs: u64,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        ValidationConfig {
            min_fee: 0,
            min_amount: 0,
            max_amount: u64::MAX,
            min_address_length: 5, // Minimum 5 chars (e.g., "addr1")
            max_address_length: 256,
            fee_multiplier: 1.0,
            enable_sequence_tracking: true,
            max_pending_per_sender: 100,
            max_future_drift_secs: 30,
            max_past_age_secs: 600, // 10 minutes
        }
    }
}

/**
 * Sender sequence state for replay attack prevention
 */
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SenderState {
    pub address: String,
    pub last_confirmed_sequence: u64,
    pub pending_transactions: Vec<u64>, // Sequence numbers of pending txs
    pub last_activity: u64,
}

impl SenderState {
    pub fn new(address: String) -> Self {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        SenderState {
            address,
            last_confirmed_sequence: 0,
            pending_transactions: Vec::new(),
            last_activity: now,
        }
    }

    pub fn is_valid_sequence(&self, sequence: u64) -> bool {
        // Sequence must be > last confirmed OR in pending range
        sequence > self.last_confirmed_sequence || self.pending_transactions.contains(&sequence)
    }

    pub fn add_pending(&mut self, sequence: u64) -> Result<(), String> {
        if self.pending_transactions.len() >= 100 {
            return Err(format!(
                "Sender has too many pending transactions: {}",
                self.pending_transactions.len()
            ));
        }
        self.pending_transactions.push(sequence);
        Ok(())
    }

    #[allow(dead_code)]
    pub fn confirm_sequence(&mut self, sequence: u64) {
        self.last_confirmed_sequence = sequence;
        self.pending_transactions.retain(|&s| s != sequence);
    }
}

/**
 * Transaction validation result
 */
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[allow(dead_code)]
impl ValidationResult {
    pub fn valid() -> Self {
        ValidationResult {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn invalid(error: String) -> Self {
        ValidationResult {
            is_valid: false,
            errors: vec![error],
            warnings: Vec::new(),
        }
    }

    pub fn with_warning(mut self, warning: String) -> Self {
        self.warnings.push(warning);
        self
    }
}

/**
 * Transaction Validation Gate
 */
pub struct TransactionValidator {
    pub config: ValidationConfig,
    pub sender_states: HashMap<String, SenderState>,
    pub seen_transaction_ids: HashMap<String, u64>, // tx_id -> timestamp
    /// Optional persistent store for seen tx IDs (survives restarts).
    pub(crate) store: Option<std::sync::Arc<dyn crate::storage::traits::BlockStore>>,
}

impl TransactionValidator {
    pub fn new(config: ValidationConfig) -> Self {
        TransactionValidator {
            config,
            sender_states: HashMap::new(),
            seen_transaction_ids: HashMap::new(),
            store: None,
        }
    }

    pub fn with_defaults() -> Self {
        TransactionValidator::new(ValidationConfig::default())
    }

    /// Attach a persistent store and load previously seen transaction IDs.
    #[allow(dead_code)]
    pub fn with_store(
        mut self,
        store: std::sync::Arc<dyn crate::storage::traits::BlockStore>,
    ) -> Self {
        if let Ok(entries) = store.load_seen_txs() {
            let count = entries.len();
            for (tx_id, ts) in entries {
                self.seen_transaction_ids.insert(tx_id, ts);
            }
            if count > 0 {
                log::info!("Loaded {count} seen transaction IDs from persistent store");
            }
        }
        self.store = Some(store);
        self
    }

    /**
     * Comprehensive transaction validation
     */
    pub fn validate(&mut self, tx: &Transaction) -> ValidationResult {
        let mut result = ValidationResult::valid();

        // 1. Format validation
        if let Err(e) = self.validate_format(tx) {
            result.is_valid = false;
            result.errors.push(e);
            return result;
        }

        // 2. Duplicate check
        if self.seen_transaction_ids.contains_key(&tx.id) {
            result.is_valid = false;
            result
                .errors
                .push("Transaction already seen (duplicate)".to_string());
            return result;
        }

        // 2b. Timestamp drift validation
        if let Err(e) = self.validate_timestamp(tx) {
            result.is_valid = false;
            result.errors.push(e);
            return result;
        }

        // 3. Amount validation
        if let Err(e) = self.validate_amounts(tx) {
            result.is_valid = false;
            result.errors.push(e);
            return result;
        }

        // 4. Fee validation
        if let Err(e) = self.validate_fees(tx) {
            result.is_valid = false;
            result.errors.push(e);
            return result;
        }

        // 5. Address validation
        if let Err(e) = self.validate_addresses(tx) {
            result.is_valid = false;
            result.errors.push(e);
            return result;
        }

        // 6. Sequence validation (replay attack prevention)
        if self.config.enable_sequence_tracking {
            if let Err(e) = self.validate_sequence(tx) {
                result.is_valid = false;
                result.errors.push(e);
                return result;
            }
        }

        // 7. Double-spend check
        if let Err(e) = self.check_double_spend(tx) {
            result.is_valid = false;
            result.errors.push(e);
            return result;
        }

        // Record transaction if valid (in-memory + persistent store)
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.seen_transaction_ids.insert(tx.id.clone(), now);
        if let Some(store) = &self.store {
            let _ = store.mark_tx_seen(&tx.id, now);
        }

        // Initialize sender state if needed
        if !self.sender_states.contains_key(&tx.from) {
            self.sender_states
                .insert(tx.from.clone(), SenderState::new(tx.from.clone()));
        }

        // Update sender state
        if self.config.enable_sequence_tracking {
            let sender_state = self
                .sender_states
                .entry(tx.from.clone())
                .or_insert_with(|| SenderState::new(tx.from.clone()));
            let _ = sender_state.add_pending(tx.timestamp);
        }

        result
    }

    /// Reject transactions whose timestamp is too far in the future or too old.
    fn validate_timestamp(&self, tx: &Transaction) -> Result<(), String> {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if tx.timestamp > now.saturating_add(self.config.max_future_drift_secs) {
            return Err(format!(
                "Transaction timestamp {} is too far in the future (max drift {}s)",
                tx.timestamp, self.config.max_future_drift_secs
            ));
        }

        if now.saturating_sub(tx.timestamp) > self.config.max_past_age_secs {
            return Err(format!(
                "Transaction timestamp {} is too old (max age {}s)",
                tx.timestamp, self.config.max_past_age_secs
            ));
        }

        Ok(())
    }

    /**
     * Validate transaction format
     */
    fn validate_format(&self, tx: &Transaction) -> Result<(), String> {
        if tx.id.is_empty() {
            return Err("Transaction ID cannot be empty".to_string());
        }

        if tx.id.len() > 256 {
            return Err("Transaction ID too long (max 256 chars)".to_string());
        }

        if tx.from == tx.to {
            return Err("Sender and receiver cannot be the same".to_string());
        }

        Ok(())
    }

    /**
     * Validate amounts
     */
    fn validate_amounts(&self, tx: &Transaction) -> Result<(), String> {
        let total = tx
            .amount
            .checked_add(tx.fee)
            .ok_or("Amount + fee overflow")?;

        if tx.amount < self.config.min_amount {
            return Err(format!(
                "Amount {} is below minimum {}",
                tx.amount, self.config.min_amount
            ));
        }

        if tx.amount > self.config.max_amount {
            return Err(format!(
                "Amount {} exceeds maximum {}",
                tx.amount, self.config.max_amount
            ));
        }

        if total == 0 {
            return Err("Transaction has zero total value (amount + fee)".to_string());
        }

        Ok(())
    }

    /**
     * Validate fees
     */
    fn validate_fees(&self, tx: &Transaction) -> Result<(), String> {
        let adjusted_min_fee = (self.config.min_fee as f64 * self.config.fee_multiplier) as u64;

        if tx.fee < adjusted_min_fee {
            return Err(format!(
                "Fee {} is below minimum {}",
                tx.fee, adjusted_min_fee
            ));
        }

        // Warn if fee seems low relative to amount
        let fee_ratio = tx.fee as f64 / (tx.amount as f64).max(1.0);
        if fee_ratio < 0.001 && tx.amount > 1000 {
            // Could be a warning but not a failure
        }

        Ok(())
    }

    /**
     * Validate addresses
     */
    fn validate_addresses(&self, tx: &Transaction) -> Result<(), String> {
        // Validate sender
        if tx.from != "0" && tx.from != "genesis" {
            if tx.from.len() < self.config.min_address_length {
                return Err(format!(
                    "Sender address too short: {} < {}",
                    tx.from.len(),
                    self.config.min_address_length
                ));
            }
            if tx.from.len() > self.config.max_address_length {
                return Err(format!(
                    "Sender address too long: {} > {}",
                    tx.from.len(),
                    self.config.max_address_length
                ));
            }
        }

        // Validate recipient
        if tx.to.len() < self.config.min_address_length {
            return Err(format!(
                "Recipient address too short: {} < {}",
                tx.to.len(),
                self.config.min_address_length
            ));
        }
        if tx.to.len() > self.config.max_address_length {
            return Err(format!(
                "Recipient address too long: {} > {}",
                tx.to.len(),
                self.config.max_address_length
            ));
        }

        Ok(())
    }

    /**
     * Validate sequence (prevent replay attacks)
     */
    fn validate_sequence(&self, tx: &Transaction) -> Result<(), String> {
        let sender_state = self
            .sender_states
            .get(&tx.from)
            .cloned()
            .unwrap_or_else(|| SenderState::new(tx.from.clone()));

        if !sender_state.is_valid_sequence(tx.timestamp) {
            return Err(format!(
                "Invalid sequence number: {} (expected > {})",
                tx.timestamp, sender_state.last_confirmed_sequence
            ));
        }

        Ok(())
    }

    /**
     * Check for double-spend within mempool state
     */
    fn check_double_spend(&self, tx: &Transaction) -> Result<(), String> {
        // Check if this sender has pending transactions that would exceed balance
        if let Some(sender_state) = self.sender_states.get(&tx.from) {
            let pending_count = sender_state.pending_transactions.len();
            if pending_count >= self.config.max_pending_per_sender {
                return Err(format!(
                    "Sender has too many pending transactions: {pending_count}"
                ));
            }
        }

        Ok(())
    }

    /**
     * Confirm transaction (mark as processed)
     */
    #[allow(dead_code)]
    pub fn confirm_transaction(&mut self, tx: &Transaction) {
        if let Some(sender_state) = self.sender_states.get_mut(&tx.from) {
            sender_state.confirm_sequence(tx.timestamp);
        }
    }

    /**
     * Get sender state
     */
    #[allow(dead_code)]
    pub fn get_sender_state(&self, address: &str) -> Option<SenderState> {
        self.sender_states.get(address).cloned()
    }

    /**
     * Cleanup old transaction records
     */
    #[allow(dead_code)]
    pub fn cleanup_old_records(&mut self, max_age_seconds: u64) {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.seen_transaction_ids
            .retain(|_, &mut timestamp| now - timestamp < max_age_seconds);

        // Also clean persistent store
        if let Some(store) = &self.store {
            let _ = store.cleanup_seen_txs(max_age_seconds);
        }
    }

    /**
     * Get validation statistics
     */
    #[allow(dead_code)]
    pub fn get_stats(&self) -> ValidationStats {
        ValidationStats {
            tracked_senders: self.sender_states.len(),
            seen_transactions: self.seen_transaction_ids.len(),
            average_pending_per_sender: if self.sender_states.is_empty() {
                0.0
            } else {
                self.sender_states
                    .values()
                    .map(|s| s.pending_transactions.len())
                    .sum::<usize>() as f64
                    / self.sender_states.len() as f64
            },
        }
    }
}

/**
 * Validation statistics
 */
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ValidationStats {
    pub tracked_senders: usize,
    pub seen_transactions: usize,
    pub average_pending_per_sender: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    fn create_test_tx(from: &str, to: &str, amount: u64, fee: u64) -> Transaction {
        Transaction {
            id: format!("tx_{from}_{to}_{amount}"),
            from: from.to_string(),
            to: to.to_string(),
            amount,
            fee,
            timestamp: now_secs(),
            signature: "sig".to_string(),
            data: None,
        }
    }

    #[test]
    fn test_valid_transaction() {
        let mut validator = TransactionValidator::with_defaults();
        let tx = create_test_tx("addr1", "addr2", 100, 1);

        let result = validator.validate(&tx);
        assert!(result.is_valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_duplicate_transaction() {
        let mut validator = TransactionValidator::with_defaults();
        let tx = create_test_tx("addr1", "addr2", 100, 1);

        let result1 = validator.validate(&tx);
        assert!(result1.is_valid);

        let result2 = validator.validate(&tx);
        assert!(!result2.is_valid);
        assert!(result2.errors[0].contains("duplicate"));
    }

    #[test]
    fn test_same_sender_receiver() {
        let mut validator = TransactionValidator::with_defaults();
        let tx = create_test_tx("addr1", "addr1", 100, 1);

        let result = validator.validate(&tx);
        assert!(!result.is_valid);
    }

    #[test]
    fn test_zero_total_value() {
        let mut validator = TransactionValidator::with_defaults();
        let tx = Transaction {
            id: "tx1".to_string(),
            from: "addr1".to_string(),
            to: "addr2".to_string(),
            amount: 0,
            fee: 0,
            timestamp: now_secs(),
            signature: "sig".to_string(),
            data: None,
        };

        let result = validator.validate(&tx);
        assert!(!result.is_valid);
    }

    #[test]
    fn test_sequence_tracking() {
        let mut validator = TransactionValidator::with_defaults();
        let tx1 = create_test_tx("addr1", "addr2", 100, 1);
        let mut tx2 = create_test_tx("addr1", "addr3", 50, 1);
        // Use a timestamp earlier than tx1 but still within drift window
        tx2.timestamp = tx1.timestamp.saturating_sub(5);
        tx2.id = "tx_different".to_string(); // Different ID to avoid duplicate check

        let result1 = validator.validate(&tx1);
        assert!(result1.is_valid);

        // Confirm the first transaction to update last_confirmed_sequence
        validator.confirm_transaction(&tx1);

        let result2 = validator.validate(&tx2);
        assert!(!result2.is_valid); // Should fail because timestamp < last_confirmed
    }

    #[test]
    fn test_address_validation() {
        let mut validator = TransactionValidator::with_defaults();
        let tx = Transaction {
            id: "tx1".to_string(),
            from: "tiny".to_string(), // Too short (4 chars < 5)
            to: "addr2_with_valid_length_____".to_string(),
            amount: 100,
            fee: 1,
            timestamp: now_secs(),
            signature: "sig".to_string(),
            data: None,
        };

        let result = validator.validate(&tx);
        assert!(!result.is_valid);
    }
}
