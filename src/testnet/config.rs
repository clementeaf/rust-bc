//! Testnet configuration — genesis parameters, network identity, and
//! bootstrapping for public test networks.

use serde::{Deserialize, Serialize};

use crate::consensus::dpos::{DposConfig, ValidatorStake};
use crate::tokenomics::economics;

/// Well-known network identifiers.
pub mod network_ids {
    pub const MAINNET: &str = "rust-bc-mainnet";
    pub const TESTNET: &str = "rust-bc-testnet";
    pub const DEVNET: &str = "rust-bc-devnet";
}

/// Genesis block configuration for a network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisConfig {
    /// Network identifier.
    pub network_id: String,
    /// Chain ID (numeric, for EVM compatibility).
    pub chain_id: u64,
    /// Genesis timestamp (UNIX seconds).
    pub timestamp: u64,
    /// Initial token allocations.
    pub allocations: Vec<GenesisAllocation>,
    /// Initial validator set.
    pub validators: Vec<ValidatorStake>,
    /// DPoS configuration.
    pub dpos: DposConfig,
    /// Block time target in seconds.
    pub block_time_secs: u64,
    /// Maximum supply (from tokenomics).
    pub max_supply: u64,
    /// Initial block reward.
    pub initial_block_reward: u64,
    /// Consensus mode: "raft" or "bft".
    pub consensus_mode: String,
    /// Faucet enabled (testnet/devnet only).
    pub faucet_enabled: bool,
    /// Faucet drip amount per request.
    pub faucet_drip: u64,
}

/// An initial token allocation in the genesis block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenesisAllocation {
    pub address: String,
    pub amount: u64,
    pub label: String,
}

impl GenesisConfig {
    /// Testnet genesis with sensible defaults.
    pub fn testnet() -> Self {
        Self {
            network_id: network_ids::TESTNET.into(),
            chain_id: 9999,
            timestamp: 0,
            allocations: vec![
                GenesisAllocation {
                    address: "faucet".into(),
                    amount: 10_000_000, // 10M for faucet
                    label: "Testnet Faucet".into(),
                },
                GenesisAllocation {
                    address: "treasury".into(),
                    amount: 5_000_000, // 5M for testing
                    label: "Testnet Treasury".into(),
                },
            ],
            validators: vec![
                ValidatorStake { address: "testnet-v0".into(), stake: 10_000, active: true },
                ValidatorStake { address: "testnet-v1".into(), stake: 10_000, active: true },
                ValidatorStake { address: "testnet-v2".into(), stake: 10_000, active: true },
                ValidatorStake { address: "testnet-v3".into(), stake: 10_000, active: true },
            ],
            dpos: DposConfig {
                max_validators: 21, // Smaller committee for testnet
                min_stake: 100,     // Lower barrier for testing
            },
            block_time_secs: 5,
            max_supply: economics::MAX_SUPPLY,
            initial_block_reward: economics::INITIAL_BLOCK_REWARD,
            consensus_mode: "bft".into(),
            faucet_enabled: true,
            faucet_drip: 1000,
        }
    }

    /// Devnet genesis — faster blocks, more faucet funds, single validator.
    pub fn devnet() -> Self {
        Self {
            network_id: network_ids::DEVNET.into(),
            chain_id: 9998,
            timestamp: 0,
            allocations: vec![GenesisAllocation {
                address: "faucet".into(),
                amount: 50_000_000,
                label: "Devnet Faucet".into(),
            }],
            validators: vec![ValidatorStake {
                address: "devnet-v0".into(),
                stake: 10_000,
                active: true,
            }],
            dpos: DposConfig {
                max_validators: 4,
                min_stake: 10,
            },
            block_time_secs: 2,
            max_supply: economics::MAX_SUPPLY,
            initial_block_reward: economics::INITIAL_BLOCK_REWARD,
            consensus_mode: "raft".into(),
            faucet_enabled: true,
            faucet_drip: 10_000,
        }
    }

    /// Mainnet genesis — production parameters.
    pub fn mainnet() -> Self {
        Self {
            network_id: network_ids::MAINNET.into(),
            chain_id: 1,
            timestamp: 0,
            allocations: vec![
                GenesisAllocation {
                    address: "treasury".into(),
                    amount: 20_000_000, // 20% of supply
                    label: "Protocol Treasury".into(),
                },
                GenesisAllocation {
                    address: "ecosystem".into(),
                    amount: 10_000_000, // 10% for ecosystem grants
                    label: "Ecosystem Fund".into(),
                },
            ],
            validators: vec![], // Populated at launch
            dpos: DposConfig::default(),
            block_time_secs: 15,
            max_supply: economics::MAX_SUPPLY,
            initial_block_reward: economics::INITIAL_BLOCK_REWARD,
            consensus_mode: "bft".into(),
            faucet_enabled: false,
            faucet_drip: 0,
        }
    }

    /// Total initial allocation.
    pub fn total_allocated(&self) -> u64 {
        self.allocations.iter().map(|a| a.amount).sum()
    }

    /// Validate genesis configuration.
    pub fn validate(&self) -> Result<(), GenesisError> {
        if self.network_id.is_empty() {
            return Err(GenesisError::EmptyNetworkId);
        }

        let total = self.total_allocated();
        if total > self.max_supply {
            return Err(GenesisError::AllocationExceedsSupply {
                allocated: total,
                max: self.max_supply,
            });
        }

        if self.consensus_mode == "bft" && self.validators.len() < 4 {
            return Err(GenesisError::InsufficientValidators {
                have: self.validators.len(),
                min: 4,
            });
        }

        Ok(())
    }
}

/// Genesis validation errors.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GenesisError {
    #[error("network_id cannot be empty")]
    EmptyNetworkId,
    #[error("allocation {allocated} exceeds max supply {max}")]
    AllocationExceedsSupply { allocated: u64, max: u64 },
    #[error("BFT requires at least {min} validators, have {have}")]
    InsufficientValidators { have: usize, min: usize },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn testnet_config_valid() {
        let config = GenesisConfig::testnet();
        assert!(config.validate().is_ok());
        assert_eq!(config.network_id, network_ids::TESTNET);
        assert!(config.faucet_enabled);
        assert_eq!(config.validators.len(), 4);
    }

    #[test]
    fn devnet_config_valid() {
        let config = GenesisConfig::devnet();
        // Devnet uses raft, so BFT validator check doesn't apply.
        assert!(config.validate().is_ok());
        assert_eq!(config.block_time_secs, 2);
        assert_eq!(config.faucet_drip, 10_000);
    }

    #[test]
    fn mainnet_config_valid() {
        let mut config = GenesisConfig::mainnet();
        // Mainnet needs validators for BFT.
        config.validators = vec![
            ValidatorStake { address: "v0".into(), stake: 100_000, active: true },
            ValidatorStake { address: "v1".into(), stake: 100_000, active: true },
            ValidatorStake { address: "v2".into(), stake: 100_000, active: true },
            ValidatorStake { address: "v3".into(), stake: 100_000, active: true },
        ];
        assert!(config.validate().is_ok());
        assert!(!config.faucet_enabled);
    }

    #[test]
    fn mainnet_without_validators_invalid() {
        let config = GenesisConfig::mainnet();
        let err = config.validate().unwrap_err();
        assert!(matches!(err, GenesisError::InsufficientValidators { .. }));
    }

    #[test]
    fn allocation_exceeds_supply_invalid() {
        let mut config = GenesisConfig::testnet();
        config.allocations.push(GenesisAllocation {
            address: "whale".into(),
            amount: economics::MAX_SUPPLY,
            label: "too much".into(),
        });
        let err = config.validate().unwrap_err();
        assert!(matches!(err, GenesisError::AllocationExceedsSupply { .. }));
    }

    #[test]
    fn total_allocated_computed() {
        let config = GenesisConfig::testnet();
        assert_eq!(config.total_allocated(), 15_000_000);
    }

    #[test]
    fn genesis_serde_roundtrip() {
        let config = GenesisConfig::testnet();
        let json = serde_json::to_string(&config).unwrap();
        let back: GenesisConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.network_id, config.network_id);
        assert_eq!(back.chain_id, config.chain_id);
    }

    #[test]
    fn empty_network_id_invalid() {
        let mut config = GenesisConfig::testnet();
        config.network_id = String::new();
        assert!(matches!(config.validate(), Err(GenesisError::EmptyNetworkId)));
    }
}
