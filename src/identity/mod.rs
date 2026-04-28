//! Identity Tier (Tier 3): Decentralized Identity (DID) and Credentials
//!
//! Responsibilities:
//! - DID document generation and management
//! - Key derivation and rotation
//! - Signature generation and verification

pub mod did;
pub mod dual_signing;
pub mod hsm;
pub mod keys;
pub mod pqc_policy;
pub mod signing;

/// Identity configuration
#[derive(Clone, Debug)]
pub struct IdentityConfig {
    #[allow(dead_code)]
    pub key_derivation_path: String,
    #[allow(dead_code)]
    pub credential_ttl_days: u32,
    #[allow(dead_code)]
    pub revocation_check_enabled: bool,
}

impl Default for IdentityConfig {
    fn default() -> Self {
        Self {
            key_derivation_path: "m/44'/0'/0'".to_string(),
            credential_ttl_days: 365,
            revocation_check_enabled: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let cfg = IdentityConfig::default();
        assert_eq!(cfg.credential_ttl_days, 365);
    }
}
