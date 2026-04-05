//! Identity Tier (Tier 3): Decentralized Identity (DID) and Credentials
//!
//! Responsibilities:
//! - DID document generation and management
//! - Key derivation and rotation
//! - Signature generation and verification

pub mod did;
pub mod hsm;
pub mod keys;
pub mod signing;

pub use did::{DidDocument, DidStatus, DidMetadata};
pub use keys::{KeyManager, PublicKeyInfo, KeyPair};

/// Identity configuration
#[derive(Clone, Debug)]
pub struct IdentityConfig {
    pub key_derivation_path: String,
    pub credential_ttl_days: u32,
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
