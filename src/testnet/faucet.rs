//! Testnet faucet — rate-limited token drip for test accounts.
//!
//! Provides free tokens to developers testing on testnet/devnet.
//! Rate-limited per address to prevent abuse.

use std::collections::HashMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

/// Faucet configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaucetConfig {
    /// Tokens per drip request.
    pub drip_amount: u64,
    /// Minimum blocks between drips for the same address.
    pub cooldown_blocks: u64,
    /// Maximum total tokens the faucet can distribute (0 = unlimited).
    pub max_total: u64,
    /// Whether the faucet is active.
    pub enabled: bool,
}

impl Default for FaucetConfig {
    fn default() -> Self {
        Self {
            drip_amount: 1000,
            cooldown_blocks: 100,
            max_total: 10_000_000,
            enabled: true,
        }
    }
}

/// Faucet drip result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DripResult {
    pub recipient: String,
    pub amount: u64,
    pub remaining_balance: u64,
    pub next_drip_at: u64,
}

/// Faucet errors.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FaucetError {
    #[error("faucet disabled")]
    Disabled,
    #[error("cooldown active for {address}: next drip at block {next_at}")]
    Cooldown { address: String, next_at: u64 },
    #[error("faucet depleted: remaining {remaining}, requested {requested}")]
    Depleted { remaining: u64, requested: u64 },
    #[error("invalid address")]
    InvalidAddress,
}

/// Testnet faucet with rate limiting.
pub struct Faucet {
    config: FaucetConfig,
    /// Total tokens distributed so far.
    total_distributed: Mutex<u64>,
    /// Last drip block per address.
    last_drip: Mutex<HashMap<String, u64>>,
}

impl Faucet {
    pub fn new(config: FaucetConfig) -> Self {
        Self {
            config,
            total_distributed: Mutex::new(0),
            last_drip: Mutex::new(HashMap::new()),
        }
    }

    /// Request tokens from the faucet.
    ///
    /// Returns the drip result on success. The caller is responsible for
    /// actually crediting the tokens to the recipient's balance.
    pub fn drip(
        &self,
        recipient: &str,
        current_block: u64,
    ) -> Result<DripResult, FaucetError> {
        if !self.config.enabled {
            return Err(FaucetError::Disabled);
        }

        if recipient.is_empty() {
            return Err(FaucetError::InvalidAddress);
        }

        // Check cooldown.
        {
            let last = self.last_drip.lock().unwrap();
            if let Some(&last_block) = last.get(recipient) {
                let next_at = last_block + self.config.cooldown_blocks;
                if current_block < next_at {
                    return Err(FaucetError::Cooldown {
                        address: recipient.into(),
                        next_at,
                    });
                }
            }
        }

        // Check balance.
        let mut total = self.total_distributed.lock().unwrap();
        if self.config.max_total > 0 {
            let remaining = self.config.max_total.saturating_sub(*total);
            if remaining < self.config.drip_amount {
                return Err(FaucetError::Depleted {
                    remaining,
                    requested: self.config.drip_amount,
                });
            }
        }

        // Drip.
        *total += self.config.drip_amount;
        let remaining = if self.config.max_total > 0 {
            self.config.max_total - *total
        } else {
            u64::MAX
        };

        self.last_drip
            .lock()
            .unwrap()
            .insert(recipient.to_string(), current_block);

        Ok(DripResult {
            recipient: recipient.into(),
            amount: self.config.drip_amount,
            remaining_balance: remaining,
            next_drip_at: current_block + self.config.cooldown_blocks,
        })
    }

    /// Total tokens distributed.
    pub fn total_distributed(&self) -> u64 {
        *self.total_distributed.lock().unwrap()
    }

    /// Remaining faucet balance (0 if unlimited).
    pub fn remaining(&self) -> u64 {
        if self.config.max_total == 0 {
            return u64::MAX;
        }
        self.config
            .max_total
            .saturating_sub(*self.total_distributed.lock().unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn faucet() -> Faucet {
        Faucet::new(FaucetConfig {
            drip_amount: 100,
            cooldown_blocks: 10,
            max_total: 1000,
            enabled: true,
        })
    }

    #[test]
    fn drip_succeeds() {
        let f = faucet();
        let result = f.drip("alice", 0).unwrap();
        assert_eq!(result.amount, 100);
        assert_eq!(result.remaining_balance, 900);
        assert_eq!(result.next_drip_at, 10);
        assert_eq!(f.total_distributed(), 100);
    }

    #[test]
    fn drip_cooldown_enforced() {
        let f = faucet();
        f.drip("alice", 0).unwrap();
        let err = f.drip("alice", 5).unwrap_err();
        assert!(matches!(err, FaucetError::Cooldown { next_at: 10, .. }));
    }

    #[test]
    fn drip_cooldown_expires() {
        let f = faucet();
        f.drip("alice", 0).unwrap();
        let result = f.drip("alice", 10).unwrap();
        assert_eq!(result.amount, 100);
    }

    #[test]
    fn drip_different_addresses_independent() {
        let f = faucet();
        f.drip("alice", 0).unwrap();
        f.drip("bob", 0).unwrap();
        assert_eq!(f.total_distributed(), 200);
    }

    #[test]
    fn drip_depleted() {
        let f = faucet();
        // Drain: 10 drips of 100 = 1000 (max_total).
        for i in 0..10 {
            f.drip(&format!("user_{i}"), 0).unwrap();
        }
        let err = f.drip("new_user", 0).unwrap_err();
        assert!(matches!(err, FaucetError::Depleted { remaining: 0, .. }));
    }

    #[test]
    fn drip_disabled() {
        let f = Faucet::new(FaucetConfig {
            enabled: false,
            ..Default::default()
        });
        let err = f.drip("alice", 0).unwrap_err();
        assert!(matches!(err, FaucetError::Disabled));
    }

    #[test]
    fn drip_empty_address_rejected() {
        let f = faucet();
        let err = f.drip("", 0).unwrap_err();
        assert!(matches!(err, FaucetError::InvalidAddress));
    }

    #[test]
    fn remaining_tracks_balance() {
        let f = faucet();
        assert_eq!(f.remaining(), 1000);
        f.drip("alice", 0).unwrap();
        assert_eq!(f.remaining(), 900);
    }

    #[test]
    fn unlimited_faucet() {
        let f = Faucet::new(FaucetConfig {
            drip_amount: 100,
            cooldown_blocks: 0,
            max_total: 0, // Unlimited
            enabled: true,
        });
        for i in 0..1000 {
            f.drip(&format!("u{i}"), 0).unwrap();
        }
        assert_eq!(f.total_distributed(), 100_000);
        assert_eq!(f.remaining(), u64::MAX);
    }

    #[test]
    fn stress_1000_drips() {
        let f = Faucet::new(FaucetConfig {
            drip_amount: 10,
            cooldown_blocks: 1,
            max_total: 100_000,
            enabled: true,
        });
        for i in 0..1000 {
            f.drip(&format!("user_{i}"), i as u64).unwrap();
        }
        assert_eq!(f.total_distributed(), 10_000);
    }
}
