//! Governable protocol parameters — a typed registry of settings that can
//! be modified through governance proposals.

use std::collections::HashMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

/// A governable parameter value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParamValue {
    U64(u64),
    Bool(bool),
    String(String),
}

impl std::fmt::Display for ParamValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParamValue::U64(v) => write!(f, "{v}"),
            ParamValue::Bool(v) => write!(f, "{v}"),
            ParamValue::String(v) => write!(f, "{v}"),
        }
    }
}

/// Well-known parameter keys.
pub mod keys {
    pub const MAX_BLOCK_SIZE: &str = "max_block_size";
    pub const MIN_TX_FEE: &str = "min_tx_fee";
    pub const BASE_FEE_ADJUSTMENT: &str = "base_fee_adjustment_factor";
    pub const SLASH_PERCENT: &str = "slash_percent";
    pub const MIN_STAKE: &str = "min_stake";
    pub const PROPOSAL_DEPOSIT: &str = "proposal_deposit";
    pub const VOTING_PERIOD_BLOCKS: &str = "voting_period_blocks";
    pub const TIMELOCK_BLOCKS: &str = "timelock_blocks";
    pub const QUORUM_PERCENT: &str = "quorum_percent";
    pub const PASS_THRESHOLD_PERCENT: &str = "pass_threshold_percent";
}

/// Registry of governable protocol parameters with defaults.
pub struct ParamRegistry {
    params: Mutex<HashMap<String, ParamValue>>,
}

impl ParamRegistry {
    /// Create with protocol defaults.
    pub fn with_defaults() -> Self {
        let mut params = HashMap::new();
        params.insert(keys::MAX_BLOCK_SIZE.into(), ParamValue::U64(500));
        params.insert(keys::MIN_TX_FEE.into(), ParamValue::U64(1));
        params.insert(keys::BASE_FEE_ADJUSTMENT.into(), ParamValue::U64(8));
        params.insert(keys::SLASH_PERCENT.into(), ParamValue::U64(5));
        params.insert(keys::MIN_STAKE.into(), ParamValue::U64(1000));
        params.insert(keys::PROPOSAL_DEPOSIT.into(), ParamValue::U64(10_000));
        params.insert(keys::VOTING_PERIOD_BLOCKS.into(), ParamValue::U64(17_280)); // ~3 days at 15s
        params.insert(keys::TIMELOCK_BLOCKS.into(), ParamValue::U64(5_760)); // ~1 day at 15s
        params.insert(keys::QUORUM_PERCENT.into(), ParamValue::U64(33));
        params.insert(keys::PASS_THRESHOLD_PERCENT.into(), ParamValue::U64(67));
        Self {
            params: Mutex::new(params),
        }
    }

    /// Get a parameter value.
    pub fn get(&self, key: &str) -> Option<ParamValue> {
        self.params.lock().unwrap().get(key).cloned()
    }

    /// Get a u64 parameter or a default.
    pub fn get_u64(&self, key: &str, default: u64) -> u64 {
        match self.get(key) {
            Some(ParamValue::U64(v)) => v,
            _ => default,
        }
    }

    /// Set a parameter (used by governance execution).
    pub fn set(&self, key: &str, value: ParamValue) {
        self.params.lock().unwrap().insert(key.to_string(), value);
    }

    /// List all parameters.
    pub fn list(&self) -> Vec<(String, ParamValue)> {
        self.params
            .lock()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }
}

impl Default for ParamRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_populated() {
        let reg = ParamRegistry::with_defaults();
        assert_eq!(reg.get_u64(keys::MAX_BLOCK_SIZE, 0), 500);
        assert_eq!(reg.get_u64(keys::MIN_TX_FEE, 0), 1);
        assert_eq!(reg.get_u64(keys::QUORUM_PERCENT, 0), 33);
    }

    #[test]
    fn set_overrides_default() {
        let reg = ParamRegistry::with_defaults();
        reg.set(keys::MIN_TX_FEE, ParamValue::U64(10));
        assert_eq!(reg.get_u64(keys::MIN_TX_FEE, 0), 10);
    }

    #[test]
    fn get_unknown_returns_none() {
        let reg = ParamRegistry::with_defaults();
        assert!(reg.get("nonexistent").is_none());
    }

    #[test]
    fn get_u64_returns_default_for_missing() {
        let reg = ParamRegistry::with_defaults();
        assert_eq!(reg.get_u64("nonexistent", 42), 42);
    }

    #[test]
    fn list_returns_all() {
        let reg = ParamRegistry::with_defaults();
        let all = reg.list();
        assert!(all.len() >= 10);
    }

    #[test]
    fn param_value_display() {
        assert_eq!(format!("{}", ParamValue::U64(100)), "100");
        assert_eq!(format!("{}", ParamValue::Bool(true)), "true");
        assert_eq!(format!("{}", ParamValue::String("hi".into())), "hi");
    }

    #[test]
    fn param_value_serde_roundtrip() {
        let val = ParamValue::U64(42);
        let json = serde_json::to_string(&val).unwrap();
        let back: ParamValue = serde_json::from_str(&json).unwrap();
        assert_eq!(val, back);
    }
}
