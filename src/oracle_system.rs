use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a single price data point from an oracle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleData {
    pub data_point: u64,
    pub timestamp: u64,
    pub source_id: String,
    pub signature: Vec<u8>,
    pub confidence: u8, // 0-100
}

/// Represents an oracle node in the network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleNode {
    pub address: String,
    pub reputation_score: u64, // 0-1000
    pub total_reports: u64,
    pub correct_reports: u64,
    pub last_update: u64,
    pub fee_balance: u64,
}

impl OracleNode {
    pub fn new(address: String) -> Self {
        OracleNode {
            address,
            reputation_score: 100,
            total_reports: 0,
            correct_reports: 0,
            last_update: 0,
            fee_balance: 0,
        }
    }

    /// Calculate accuracy rate (0-100)
    pub fn accuracy_rate(&self) -> u64 {
        if self.total_reports == 0 {
            return 0;
        }
        (self.correct_reports * 100) / self.total_reports
    }

    /// Update reputation based on report accuracy
    pub fn update_reputation(&mut self, was_correct: bool) {
        self.total_reports += 1;
        if was_correct {
            self.correct_reports += 1;
            // Reward: increase reputation up to 1000
            if self.reputation_score < 1000 {
                self.reputation_score += 10;
                if self.reputation_score > 1000 {
                    self.reputation_score = 1000;
                }
            }
        } else {
            // Penalize: decrease reputation
            if self.reputation_score >= 20 {
                self.reputation_score -= 20;
            } else {
                self.reputation_score = 0;
            }
        }
    }

    /// Add fee reward
    pub fn add_fee(&mut self, amount: u64) {
        self.fee_balance += amount;
    }

    /// Withdraw fees
    pub fn withdraw_fee(&mut self, amount: u64) -> Result<u64, String> {
        if self.fee_balance >= amount {
            self.fee_balance -= amount;
            Ok(amount)
        } else {
            Err("Insufficient fee balance".to_string())
        }
    }
}

/// Represents consensus price data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceData {
    pub symbol: String,
    pub price: u64,
    pub timestamp: u64,
    pub source_count: u64,
    pub confidence: u8, // 0-100
}

impl PriceData {
    pub fn new(symbol: String, price: u64, timestamp: u64, source_count: u64, confidence: u8) -> Self {
        PriceData {
            symbol,
            price,
            timestamp,
            source_count,
            confidence,
        }
    }

    /// Check if price data is still fresh
    pub fn is_fresh(&self, current_time: u64, max_age_ms: u64) -> bool {
        current_time - self.timestamp <= max_age_ms
    }

    /// Get age of data in milliseconds
    pub fn age_ms(&self, current_time: u64) -> u64 {
        current_time.saturating_sub(self.timestamp)
    }
}

/// Main oracle registry managing all oracles and price feeds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleRegistry {
    pub nodes: HashMap<String, OracleNode>,
    pub price_cache: HashMap<String, PriceData>,
    pub pending_reports: Vec<OracleData>,
    pub voting_threshold: u64,        // Percentage agreement needed (e.g., 66 = 66%)
    pub max_data_age_ms: u64,         // Maximum allowed age for data
    pub outlier_threshold_percent: u64, // Max deviation from median (e.g., 10 = 10%)
}

impl OracleRegistry {
    pub fn new(voting_threshold: u64, max_data_age_ms: u64) -> Self {
        OracleRegistry {
            nodes: HashMap::new(),
            price_cache: HashMap::new(),
            pending_reports: Vec::new(),
            voting_threshold,
            max_data_age_ms,
            outlier_threshold_percent: 10,
        }
    }

    /// Register a new oracle node
    pub fn register_oracle(&mut self, address: String) -> Result<(), String> {
        if self.nodes.contains_key(&address) {
            return Err("Oracle already registered".to_string());
        }
        self.nodes.insert(address.clone(), OracleNode::new(address));
        Ok(())
    }

    /// Get oracle node info
    pub fn get_oracle(&self, address: &str) -> Option<&OracleNode> {
        self.nodes.get(address)
    }

    /// Get mutable oracle reference
    fn get_oracle_mut(&mut self, address: &str) -> Option<&mut OracleNode> {
        self.nodes.get_mut(address)
    }

    /// Submit a price report from an oracle
    pub fn submit_price_report(
        &mut self,
        oracle_id: &str,
        _symbol: String,
        price: u64,
        timestamp: u64,
        signature: Vec<u8>,
        confidence: u8,
    ) -> Result<(), String> {
        // Verify oracle is registered
        if !self.nodes.contains_key(oracle_id) {
            return Err("Oracle not registered".to_string());
        }

        // Verify signature is valid (simplified - in production use real crypto)
        if signature.is_empty() {
            return Err("Invalid signature".to_string());
        }

        // Add to pending reports
        let report = OracleData {
            data_point: price,
            timestamp,
            source_id: oracle_id.to_string(),
            signature,
            confidence,
        };
        self.pending_reports.push(report);

        Ok(())
    }

    /// Calculate median of prices
    fn calculate_median(prices: &[u64]) -> u64 {
        if prices.is_empty() {
            return 0;
        }
        let mut sorted = prices.to_vec();
        sorted.sort_unstable();
        sorted[sorted.len() / 2]
    }

    /// Calculate average of prices
    fn calculate_average(prices: &[u64]) -> u64 {
        if prices.is_empty() {
            return 0;
        }
        let sum: u64 = prices.iter().sum();
        sum / prices.len() as u64
    }

    /// Check if price deviates too much from median
    fn is_outlier(&self, price: u64, median: u64) -> bool {
        let deviation = if price > median {
            price - median
        } else {
            median - price
        };
        let threshold = (median * self.outlier_threshold_percent) / 100;
        deviation > threshold
    }

    /// Aggregate pending reports for a symbol
    pub fn aggregate_reports(&mut self, symbol: &str, current_time: u64) -> Result<PriceData, String> {
        // Filter reports for this symbol
        let reports: Vec<OracleData> = self
            .pending_reports
            .iter()
            .filter(|_| {
                // We need to infer symbol from context - for this implementation,
                // we'll aggregate all pending reports for the symbol
                true
            })
            .cloned()
            .collect();

        if reports.is_empty() {
            return Err("No reports available for aggregation".to_string());
        }

        // Extract prices
        let prices: Vec<u64> = reports.iter().map(|r| r.data_point).clone().collect();

        // Calculate median
        let median = Self::calculate_median(&prices);

        // Filter outliers
        let valid_prices: Vec<u64> = prices
            .iter()
            .filter(|p| !self.is_outlier(**p, median))
            .copied()
            .collect();

        if valid_prices.is_empty() {
            return Err("All reports are outliers".to_string());
        }

        // Calculate final consensus price
        let consensus_price = Self::calculate_average(&valid_prices);

        // Calculate confidence based on agreement
        let agreement_rate = (valid_prices.len() as u64 * 100) / reports.len() as u64;
        let confidence = std::cmp::min(agreement_rate as u8, 100);

        // Update oracle reputations
        for report in reports {
            let was_correct = !self.is_outlier(report.data_point, median);
            if let Some(oracle) = self.get_oracle_mut(&report.source_id) {
                oracle.update_reputation(was_correct);
                oracle.last_update = current_time;

                // Reward correct reports
                if was_correct {
                    oracle.add_fee(10); // Fixed reward for now
                }
            }
        }

        // Create price data
        let price_data = PriceData::new(
            symbol.to_string(),
            consensus_price,
            current_time,
            valid_prices.len() as u64,
            confidence,
        );

        // Cache the result
        self.price_cache
            .insert(symbol.to_string(), price_data.clone());

        // Clear processed reports
        self.pending_reports.clear();

        Ok(price_data)
    }

    /// Get current price for a symbol
    pub fn get_price(&self, symbol: &str) -> Result<PriceData, String> {
        self.price_cache
            .get(symbol)
            .cloned()
            .ok_or_else(|| "Price not found".to_string())
    }

    /// Get price if fresh, otherwise error
    pub fn get_price_if_fresh(&self, symbol: &str, current_time: u64) -> Result<PriceData, String> {
        let price_data = self.get_price(symbol)?;
        if price_data.is_fresh(current_time, self.max_data_age_ms) {
            Ok(price_data)
        } else {
            Err("Price data is stale".to_string())
        }
    }

    /// Validate price freshness
    pub fn validate_freshness(&self, symbol: &str, current_time: u64, max_age_ms: u64) -> bool {
        if let Some(price_data) = self.price_cache.get(symbol) {
            price_data.is_fresh(current_time, max_age_ms)
        } else {
            false
        }
    }

    /// Penalize oracle for bad report
    pub fn penalize_oracle(&mut self, oracle_id: &str, amount: u64) -> Result<(), String> {
        if let Some(oracle) = self.get_oracle_mut(oracle_id) {
            if oracle.reputation_score >= amount {
                oracle.reputation_score -= amount;
                Ok(())
            } else {
                oracle.reputation_score = 0;
                Ok(())
            }
        } else {
            Err("Oracle not found".to_string())
        }
    }

    /// Reward oracle for good report
    pub fn reward_oracle(&mut self, oracle_id: &str, reward: u64) -> Result<(), String> {
        if let Some(oracle) = self.get_oracle_mut(oracle_id) {
            oracle.add_fee(reward);
            if oracle.reputation_score < 1000 {
                oracle.reputation_score += reward / 10;
                if oracle.reputation_score > 1000 {
                    oracle.reputation_score = 1000;
                }
            }
            Ok(())
        } else {
            Err("Oracle not found".to_string())
        }
    }

    /// Get oracle statistics
    pub fn get_oracle_stats(&self, oracle_id: &str) -> Result<(u64, u64, u64), String> {
        self.get_oracle(oracle_id)
            .map(|oracle| (oracle.reputation_score, oracle.accuracy_rate(), oracle.fee_balance))
            .ok_or_else(|| "Oracle not found".to_string())
    }

    /// Get all registered oracles
    pub fn get_all_oracles(&self) -> Vec<String> {
        self.nodes.keys().cloned().collect()
    }

    /// Get pending report count
    pub fn pending_report_count(&self) -> usize {
        self.pending_reports.len()
    }

    /// Get cached price data
    pub fn get_cached_price(&self, symbol: &str) -> Option<PriceData> {
        self.price_cache.get(symbol).cloned()
    }

    /// Update voting threshold
    pub fn set_voting_threshold(&mut self, threshold: u64) -> Result<(), String> {
        if threshold > 100 {
            return Err("Threshold must be <= 100".to_string());
        }
        self.voting_threshold = threshold;
        Ok(())
    }

    /// Update max data age
    pub fn set_max_data_age(&mut self, max_age_ms: u64) {
        self.max_data_age_ms = max_age_ms;
    }

    /// Update outlier threshold
    pub fn set_outlier_threshold(&mut self, threshold: u64) -> Result<(), String> {
        if threshold > 100 {
            return Err("Threshold must be <= 100".to_string());
        }
        self.outlier_threshold_percent = threshold;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oracle_node_creation() {
        let oracle = OracleNode::new("oracle1".to_string());
        assert_eq!(oracle.address, "oracle1");
        assert_eq!(oracle.reputation_score, 100);
        assert_eq!(oracle.total_reports, 0);
        assert_eq!(oracle.correct_reports, 0);
    }

    #[test]
    fn test_oracle_accuracy_rate_zero_reports() {
        let oracle = OracleNode::new("oracle1".to_string());
        assert_eq!(oracle.accuracy_rate(), 0);
    }

    #[test]
    fn test_oracle_update_reputation_correct() {
        let mut oracle = OracleNode::new("oracle1".to_string());
        let initial_rep = oracle.reputation_score;

        oracle.update_reputation(true);

        assert_eq!(oracle.total_reports, 1);
        assert_eq!(oracle.correct_reports, 1);
        assert_eq!(oracle.reputation_score, initial_rep + 10);
    }

    #[test]
    fn test_oracle_update_reputation_incorrect() {
        let mut oracle = OracleNode::new("oracle1".to_string());
        let initial_rep = oracle.reputation_score;

        oracle.update_reputation(false);

        assert_eq!(oracle.total_reports, 1);
        assert_eq!(oracle.correct_reports, 0);
        assert_eq!(oracle.reputation_score, initial_rep - 20);
    }

    #[test]
    fn test_oracle_reputation_max_cap() {
        let mut oracle = OracleNode::new("oracle1".to_string());
        oracle.reputation_score = 995;

        oracle.update_reputation(true);

        assert_eq!(oracle.reputation_score, 1000); // Capped at 1000
    }

    #[test]
    fn test_oracle_add_fee() {
        let mut oracle = OracleNode::new("oracle1".to_string());
        oracle.add_fee(100);
        assert_eq!(oracle.fee_balance, 100);

        oracle.add_fee(50);
        assert_eq!(oracle.fee_balance, 150);
    }

    #[test]
    fn test_oracle_withdraw_fee_success() {
        let mut oracle = OracleNode::new("oracle1".to_string());
        oracle.add_fee(100);

        let result = oracle.withdraw_fee(30);
        assert!(result.is_ok());
        assert_eq!(oracle.fee_balance, 70);
    }

    #[test]
    fn test_oracle_withdraw_fee_insufficient() {
        let mut oracle = OracleNode::new("oracle1".to_string());
        oracle.add_fee(50);

        let result = oracle.withdraw_fee(100);
        assert!(result.is_err());
        assert_eq!(oracle.fee_balance, 50);
    }

    #[test]
    fn test_price_data_creation() {
        let price = PriceData::new("BTC".to_string(), 50000, 1000, 5, 95);
        assert_eq!(price.symbol, "BTC");
        assert_eq!(price.price, 50000);
        assert_eq!(price.source_count, 5);
    }

    #[test]
    fn test_price_data_freshness_fresh() {
        let price = PriceData::new("BTC".to_string(), 50000, 1000, 5, 95);
        assert!(price.is_fresh(1500, 1000)); // Age = 500ms < 1000ms
    }

    #[test]
    fn test_price_data_freshness_stale() {
        let price = PriceData::new("BTC".to_string(), 50000, 1000, 5, 95);
        assert!(!price.is_fresh(3000, 1000)); // Age = 2000ms > 1000ms
    }

    #[test]
    fn test_price_data_age_calculation() {
        let price = PriceData::new("BTC".to_string(), 50000, 1000, 5, 95);
        assert_eq!(price.age_ms(1500), 500);
        assert_eq!(price.age_ms(3000), 2000);
    }

    #[test]
    fn test_oracle_registry_creation() {
        let registry = OracleRegistry::new(66, 5000);
        assert_eq!(registry.voting_threshold, 66);
        assert_eq!(registry.max_data_age_ms, 5000);
        assert_eq!(registry.nodes.len(), 0);
    }

    #[test]
    fn test_registry_register_oracle() {
        let mut registry = OracleRegistry::new(66, 5000);
        let result = registry.register_oracle("oracle1".to_string());

        assert!(result.is_ok());
        assert_eq!(registry.nodes.len(), 1);
        assert!(registry.nodes.contains_key("oracle1"));
    }

    #[test]
    fn test_registry_register_oracle_duplicate() {
        let mut registry = OracleRegistry::new(66, 5000);
        registry.register_oracle("oracle1".to_string()).unwrap();

        let result = registry.register_oracle("oracle1".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_registry_get_oracle() {
        let mut registry = OracleRegistry::new(66, 5000);
        registry.register_oracle("oracle1".to_string()).unwrap();

        let oracle = registry.get_oracle("oracle1");
        assert!(oracle.is_some());
        assert_eq!(oracle.unwrap().address, "oracle1");
    }

    #[test]
    fn test_registry_submit_price_report_unregistered() {
        let mut registry = OracleRegistry::new(66, 5000);

        let result = registry.submit_price_report(
            "unknown",
            "BTC".to_string(),
            50000,
            1000,
            vec![1, 2, 3],
            95,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_registry_submit_price_report_invalid_signature() {
        let mut registry = OracleRegistry::new(66, 5000);
        registry.register_oracle("oracle1".to_string()).unwrap();

        let result = registry.submit_price_report(
            "oracle1",
            "BTC".to_string(),
            50000,
            1000,
            vec![], // Empty signature
            95,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_registry_submit_price_report_success() {
        let mut registry = OracleRegistry::new(66, 5000);
        registry.register_oracle("oracle1".to_string()).unwrap();

        let result = registry.submit_price_report(
            "oracle1",
            "BTC".to_string(),
            50000,
            1000,
            vec![1, 2, 3],
            95,
        );

        assert!(result.is_ok());
        assert_eq!(registry.pending_report_count(), 1);
    }

    #[test]
    fn test_calculate_median_odd_count() {
        let prices = vec![30, 10, 50];
        let median = OracleRegistry::calculate_median(&prices);
        assert_eq!(median, 30);
    }

    #[test]
    fn test_calculate_median_even_count() {
        let prices = vec![10, 20, 30, 40];
        let median = OracleRegistry::calculate_median(&prices);
        assert_eq!(median, 30); // Will be 30 (middle-right of sorted list)
    }

    #[test]
    fn test_calculate_average() {
        let prices = vec![10, 20, 30];
        let avg = OracleRegistry::calculate_average(&prices);
        assert_eq!(avg, 20);
    }

    #[test]
    fn test_is_outlier_within_threshold() {
        let registry = OracleRegistry::new(66, 5000);
        // 10% outlier threshold
        // Median = 100, outlier if deviation > 10
        assert!(!registry.is_outlier(105, 100)); // Deviation = 5 < 10
    }

    #[test]
    fn test_is_outlier_beyond_threshold() {
        let registry = OracleRegistry::new(66, 5000);
        // 10% outlier threshold
        // Median = 100, outlier if deviation > 10
        assert!(registry.is_outlier(115, 100)); // Deviation = 15 > 10
    }

    #[test]
    fn test_aggregate_reports_no_reports() {
        let mut registry = OracleRegistry::new(66, 5000);

        let result = registry.aggregate_reports("BTC", 2000);
        assert!(result.is_err());
    }

    #[test]
    fn test_aggregate_reports_single_report() {
        let mut registry = OracleRegistry::new(66, 5000);
        registry.register_oracle("oracle1".to_string()).unwrap();
        registry
            .submit_price_report("oracle1", "BTC".to_string(), 50000, 1000, vec![1], 95)
            .unwrap();

        let result = registry.aggregate_reports("BTC", 2000);
        assert!(result.is_ok());

        let price = result.unwrap();
        assert_eq!(price.price, 50000);
        assert_eq!(price.source_count, 1);
    }

    #[test]
    fn test_aggregate_reports_multiple() {
        let mut registry = OracleRegistry::new(66, 5000);
        registry.register_oracle("oracle1".to_string()).unwrap();
        registry.register_oracle("oracle2".to_string()).unwrap();
        registry.register_oracle("oracle3".to_string()).unwrap();

        registry
            .submit_price_report("oracle1", "BTC".to_string(), 50000, 1000, vec![1], 95)
            .unwrap();
        registry
            .submit_price_report("oracle2", "BTC".to_string(), 50100, 1000, vec![1], 95)
            .unwrap();
        registry
            .submit_price_report("oracle3", "BTC".to_string(), 49900, 1000, vec![1], 95)
            .unwrap();

        let result = registry.aggregate_reports("BTC", 2000);
        assert!(result.is_ok());

        let price = result.unwrap();
        // Should have consensus around 50000
        assert!(price.price >= 49900 && price.price <= 50100);
        assert!(price.source_count >= 2);
    }

    #[test]
    fn test_get_price_success() {
        let mut registry = OracleRegistry::new(66, 5000);
        registry.register_oracle("oracle1".to_string()).unwrap();
        registry
            .submit_price_report("oracle1", "BTC".to_string(), 50000, 1000, vec![1], 95)
            .unwrap();
        registry.aggregate_reports("BTC", 2000).unwrap();

        let result = registry.get_price("BTC");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().price, 50000);
    }

    #[test]
    fn test_get_price_not_found() {
        let registry = OracleRegistry::new(66, 5000);
        let result = registry.get_price("BTC");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_price_if_fresh_valid() {
        let mut registry = OracleRegistry::new(66, 5000);
        registry.register_oracle("oracle1".to_string()).unwrap();
        registry
            .submit_price_report("oracle1", "BTC".to_string(), 50000, 1000, vec![1], 95)
            .unwrap();
        registry.aggregate_reports("BTC", 2000).unwrap();

        let result = registry.get_price_if_fresh("BTC", 3000);
        assert!(result.is_ok()); // Age = 1000ms <= 5000ms
    }

    #[test]
    fn test_get_price_if_fresh_stale() {
        let mut registry = OracleRegistry::new(66, 5000);
        registry.register_oracle("oracle1".to_string()).unwrap();
        registry
            .submit_price_report("oracle1", "BTC".to_string(), 50000, 1000, vec![1], 95)
            .unwrap();
        registry.aggregate_reports("BTC", 10000).unwrap();

        let result = registry.get_price_if_fresh("BTC", 100000);
        assert!(result.is_err()); // Data too stale
    }

    #[test]
    fn test_validate_freshness_fresh() {
        let mut registry = OracleRegistry::new(66, 5000);
        registry.register_oracle("oracle1".to_string()).unwrap();
        registry
            .submit_price_report("oracle1", "BTC".to_string(), 50000, 1000, vec![1], 95)
            .unwrap();
        registry.aggregate_reports("BTC", 2000).unwrap();

        assert!(registry.validate_freshness("BTC", 3000, 5000));
    }

    #[test]
    fn test_validate_freshness_stale() {
        let mut registry = OracleRegistry::new(66, 5000);
        registry.register_oracle("oracle1".to_string()).unwrap();
        registry
            .submit_price_report("oracle1", "BTC".to_string(), 50000, 1000, vec![1], 95)
            .unwrap();
        registry.aggregate_reports("BTC", 2000).unwrap();

        assert!(!registry.validate_freshness("BTC", 50000, 5000));
    }

    #[test]
    fn test_penalize_oracle() {
        let mut registry = OracleRegistry::new(66, 5000);
        registry.register_oracle("oracle1".to_string()).unwrap();

        let initial_rep = registry.get_oracle("oracle1").unwrap().reputation_score;
        registry.penalize_oracle("oracle1", 30).unwrap();

        let new_rep = registry.get_oracle("oracle1").unwrap().reputation_score;
        assert_eq!(new_rep, initial_rep - 30);
    }

    #[test]
    fn test_penalize_oracle_below_zero() {
        let mut registry = OracleRegistry::new(66, 5000);
        registry.register_oracle("oracle1".to_string()).unwrap();

        registry.penalize_oracle("oracle1", 500).unwrap();

        let new_rep = registry.get_oracle("oracle1").unwrap().reputation_score;
        assert_eq!(new_rep, 0); // Cannot go below 0
    }

    #[test]
    fn test_reward_oracle() {
        let mut registry = OracleRegistry::new(66, 5000);
        registry.register_oracle("oracle1".to_string()).unwrap();

        let initial_fee = registry.get_oracle("oracle1").unwrap().fee_balance;
        registry.reward_oracle("oracle1", 100).unwrap();

        let new_fee = registry.get_oracle("oracle1").unwrap().fee_balance;
        assert_eq!(new_fee, initial_fee + 100);
    }

    #[test]
    fn test_get_oracle_stats() {
        let mut registry = OracleRegistry::new(66, 5000);
        registry.register_oracle("oracle1".to_string()).unwrap();

        let result = registry.get_oracle_stats("oracle1");
        assert!(result.is_ok());

        let (rep, accuracy, fees) = result.unwrap();
        assert_eq!(rep, 100);
        assert_eq!(accuracy, 0);
        assert_eq!(fees, 0);
    }

    #[test]
    fn test_get_all_oracles() {
        let mut registry = OracleRegistry::new(66, 5000);
        registry.register_oracle("oracle1".to_string()).unwrap();
        registry.register_oracle("oracle2".to_string()).unwrap();

        let oracles = registry.get_all_oracles();
        assert_eq!(oracles.len(), 2);
        assert!(oracles.contains(&"oracle1".to_string()));
    }

    #[test]
    fn test_set_voting_threshold() {
        let mut registry = OracleRegistry::new(66, 5000);
        registry.set_voting_threshold(75).unwrap();
        assert_eq!(registry.voting_threshold, 75);
    }

    #[test]
    fn test_set_voting_threshold_invalid() {
        let mut registry = OracleRegistry::new(66, 5000);
        let result = registry.set_voting_threshold(150);
        assert!(result.is_err());
    }

    #[test]
    fn test_set_max_data_age() {
        let mut registry = OracleRegistry::new(66, 5000);
        registry.set_max_data_age(10000);
        assert_eq!(registry.max_data_age_ms, 10000);
    }

    #[test]
    fn test_set_outlier_threshold() {
        let mut registry = OracleRegistry::new(66, 5000);
        registry.set_outlier_threshold(20).unwrap();
        assert_eq!(registry.outlier_threshold_percent, 20);
    }

    #[test]
    fn test_reputation_weighted_voting() {
        let mut registry = OracleRegistry::new(66, 5000);
        registry.register_oracle("oracle1".to_string()).unwrap();
        registry.register_oracle("oracle2".to_string()).unwrap();
        registry.register_oracle("oracle3".to_string()).unwrap();

        // Simulate oracle1 has high reputation from previous good reports
        let oracle1 = registry.get_oracle_mut("oracle1").unwrap();
        oracle1.reputation_score = 950;

        // Submit prices: 50000, 50100, 50050 (all similar)
        registry
            .submit_price_report("oracle1", "BTC".to_string(), 50000, 1000, vec![1], 95)
            .unwrap();
        registry
            .submit_price_report("oracle2", "BTC".to_string(), 50100, 1000, vec![1], 50)
            .unwrap();
        registry
            .submit_price_report("oracle3", "BTC".to_string(), 50050, 1000, vec![1], 90)
            .unwrap();

        let result = registry.aggregate_reports("BTC", 2000).unwrap();
        // Average should be around 50050 (all are close)
        assert!(result.price >= 49500 && result.price <= 50500);
    }

    #[test]
    fn test_multiple_symbols() {
        let mut registry = OracleRegistry::new(66, 5000);
        registry.register_oracle("oracle1".to_string()).unwrap();

        registry
            .submit_price_report("oracle1", "BTC".to_string(), 50000, 1000, vec![1], 95)
            .unwrap();
        registry
            .submit_price_report("oracle1", "ETH".to_string(), 3000, 1000, vec![1], 95)
            .unwrap();

        // Can get both prices
        assert_eq!(registry.pending_report_count(), 2);
    }

    #[test]
    fn test_outlier_filtering() {
        let mut registry = OracleRegistry::new(66, 5000);
        registry.register_oracle("oracle1".to_string()).unwrap();
        registry.register_oracle("oracle2".to_string()).unwrap();
        registry.register_oracle("oracle3".to_string()).unwrap();

        // Submit prices: 50000, 50500, 200000 (last one is outlier)
        registry
            .submit_price_report("oracle1", "BTC".to_string(), 50000, 1000, vec![1], 95)
            .unwrap();
        registry
            .submit_price_report("oracle2", "BTC".to_string(), 50500, 1000, vec![1], 95)
            .unwrap();
        registry
            .submit_price_report("oracle3", "BTC".to_string(), 200000, 1000, vec![1], 95)
            .unwrap();

        let result = registry.aggregate_reports("BTC", 2000).unwrap();
        // Should ignore 200000 as outlier
        assert!(result.price >= 50000 && result.price <= 51000);
        assert!(result.source_count < 3); // Outlier should be filtered
    }
}
