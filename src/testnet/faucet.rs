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
    /// Maximum drips per IP per day (0 = unlimited).
    #[serde(default)]
    pub max_drips_per_ip_per_day: u32,
    /// Maximum tokens distributed per day across all addresses (0 = unlimited).
    #[serde(default)]
    pub max_daily_total: u64,
}

impl Default for FaucetConfig {
    fn default() -> Self {
        Self {
            drip_amount: 1000,
            cooldown_blocks: 100,
            max_total: 10_000_000,
            enabled: true,
            max_drips_per_ip_per_day: 10,
            max_daily_total: 100_000,
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
    #[error("IP {ip} exceeded daily drip limit ({max} per day)")]
    IpLimitExceeded { ip: String, max: u32 },
    #[error("daily distribution cap reached ({cap} NOTA)")]
    DailyCapReached { cap: u64 },
}

/// Testnet faucet with rate limiting.
pub struct Faucet {
    config: FaucetConfig,
    /// Total tokens distributed so far.
    total_distributed: Mutex<u64>,
    /// Last drip block per address.
    last_drip: Mutex<HashMap<String, u64>>,
    /// Per-IP drip count for the current day (ip → (day, count)).
    ip_drips: Mutex<HashMap<String, (u64, u32)>>,
    /// Daily distribution (day → total distributed that day).
    daily_totals: Mutex<HashMap<u64, u64>>,
}

impl Faucet {
    pub fn new(config: FaucetConfig) -> Self {
        Self {
            config,
            total_distributed: Mutex::new(0),
            last_drip: Mutex::new(HashMap::new()),
            ip_drips: Mutex::new(HashMap::new()),
            daily_totals: Mutex::new(HashMap::new()),
        }
    }

    /// Access the faucet configuration.
    pub fn config(&self) -> &FaucetConfig {
        &self.config
    }

    /// Request tokens from the faucet.
    ///
    /// Returns the drip result on success. The caller is responsible for
    /// actually crediting the tokens to the recipient's balance.
    pub fn drip(&self, recipient: &str, current_block: u64) -> Result<DripResult, FaucetError> {
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

    /// Drip with IP-based rate limiting and daily cap.
    ///
    /// `day` is a monotonic day counter (e.g. `current_block / blocks_per_day`).
    pub fn drip_with_ip(
        &self,
        recipient: &str,
        current_block: u64,
        ip: &str,
        day: u64,
    ) -> Result<DripResult, FaucetError> {
        // IP rate limit
        if self.config.max_drips_per_ip_per_day > 0 {
            let mut ip_map = self.ip_drips.lock().unwrap_or_else(|e| e.into_inner());
            let entry = ip_map.entry(ip.to_string()).or_insert((day, 0));
            if entry.0 != day {
                // New day, reset counter
                *entry = (day, 0);
            }
            if entry.1 >= self.config.max_drips_per_ip_per_day {
                return Err(FaucetError::IpLimitExceeded {
                    ip: ip.to_string(),
                    max: self.config.max_drips_per_ip_per_day,
                });
            }
            entry.1 += 1;
        }

        // Daily total cap
        if self.config.max_daily_total > 0 {
            let mut daily = self.daily_totals.lock().unwrap_or_else(|e| e.into_inner());
            let today = daily.entry(day).or_insert(0);
            if *today + self.config.drip_amount > self.config.max_daily_total {
                return Err(FaucetError::DailyCapReached {
                    cap: self.config.max_daily_total,
                });
            }
            *today += self.config.drip_amount;
        }

        // Delegate to base drip (address cooldown + total cap)
        self.drip(recipient, current_block)
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
            max_drips_per_ip_per_day: 0,
            max_daily_total: 0,
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
            max_drips_per_ip_per_day: 0,
            max_daily_total: 0,
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
            max_drips_per_ip_per_day: 0,
            max_daily_total: 0,
        });
        for i in 0..1000 {
            f.drip(&format!("user_{i}"), i as u64).unwrap();
        }
        assert_eq!(f.total_distributed(), 10_000);
    }
}
