//! Identity Tier (Tier 3): Decentralized Identity (DID) and Credentials
//!
//! Responsibilities:
//! - DID document generation and management
//! - Credential issuance, verification, revocation
//! - Key derivation and rotation
//! - Signature generation and verification
//! - eIDAS attribute mapping

use thiserror::Error;

pub mod did;
pub mod credentials;
pub mod keys;
pub mod signatures;
pub mod errors;
pub mod traits;
pub mod eidas;

pub use did::DidDocument;
pub use credentials::{Credential, CredentialStatus};
pub use errors::{IdentityError, IdentityResult};
pub use keys::KeyManager;
pub use traits::IdentityService;

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
