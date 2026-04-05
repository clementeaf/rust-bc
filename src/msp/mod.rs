pub mod identity;
pub mod ou;

use serde::{Deserialize, Serialize};
use crate::storage::errors::StorageResult;

/// Persistent CRL store — abstracts over MemoryStore / RocksDb.
/// key = `msp_id`, value = list of revoked serials.
pub trait CrlStore: Send + Sync {
    fn write_crl(&self, msp_id: &str, serials: &[String]) -> StorageResult<()>;
    fn read_crl(&self, msp_id: &str) -> StorageResult<Vec<String>>;
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum MspError {
    #[error("public key is not registered under this MSP")]
    UnknownKey,
    #[error("key with serial {serial} has been revoked")]
    Revoked { serial: String },
}

/// Membership Service Provider: represents one organization's trust anchor.
/// `root_public_keys` holds Ed25519 pubkeys ([u8; 32]) — same representation
/// as `identity/keys.rs`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Msp {
    pub msp_id: String,
    pub root_public_keys: Vec<[u8; 32]>,
    pub revoked_serials: Vec<String>,
    pub org_id: String,
}

impl Msp {
    pub fn new(msp_id: impl Into<String>, org_id: impl Into<String>) -> Self {
        Self {
            msp_id: msp_id.into(),
            org_id: org_id.into(),
            root_public_keys: Vec::new(),
            revoked_serials: Vec::new(),
        }
    }

    /// Revoke a key by its serial (hex-encoded public key bytes).
    pub fn revoke(&mut self, serial: &str) {
        if !self.revoked_serials.iter().any(|s| s == serial) {
            self.revoked_serials.push(serial.to_string());
        }
    }

    /// Validate that `public_key` is a registered root key for this MSP and has
    /// not been revoked. The serial is the lowercase hex encoding of the key bytes.
    pub fn validate_identity(&self, public_key: &[u8; 32]) -> Result<(), MspError> {
        if !self.root_public_keys.contains(public_key) {
            return Err(MspError::UnknownKey);
        }
        let serial = hex::encode(public_key);
        if self.revoked_serials.contains(&serial) {
            return Err(MspError::Revoked { serial });
        }
        Ok(())
    }
}

/// Role a principal holds within an MSP.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MspRole {
    Admin,
    Member,
    Client,
    Peer,
    Orderer,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_msp() {
        let msp = Msp::new("Org1MSP", "Org1");
        assert_eq!(msp.msp_id, "Org1MSP");
        assert_eq!(msp.org_id, "Org1");
        assert!(msp.root_public_keys.is_empty());
        assert!(msp.revoked_serials.is_empty());
    }

    #[test]
    fn create_msp_with_keys() {
        let key = [1u8; 32];
        let mut msp = Msp::new("Org1MSP", "Org1");
        msp.root_public_keys.push(key);
        assert_eq!(msp.root_public_keys.len(), 1);
        assert_eq!(msp.root_public_keys[0], [1u8; 32]);
    }

    #[test]
    fn serde_roundtrip() {
        let mut msp = Msp::new("Org2MSP", "Org2");
        msp.root_public_keys.push([42u8; 32]);
        msp.revoked_serials.push("serial-001".to_string());

        let json = serde_json::to_string(&msp).unwrap();
        let decoded: Msp = serde_json::from_str(&json).unwrap();
        assert_eq!(msp, decoded);
    }

    #[test]
    fn revoke_then_validate_fails() {
        let key = [9u8; 32];
        let serial = hex::encode(key);
        let mut msp = Msp::new("Org1MSP", "Org1");
        msp.root_public_keys.push(key);

        assert!(msp.validate_identity(&key).is_ok());
        msp.revoke(&serial);
        assert_eq!(
            msp.validate_identity(&key),
            Err(MspError::Revoked { serial })
        );
    }

    #[test]
    fn revoke_is_idempotent() {
        let mut msp = Msp::new("Org1MSP", "Org1");
        msp.revoke("serial-abc");
        msp.revoke("serial-abc");
        assert_eq!(msp.revoked_serials.len(), 1);
    }

    #[test]
    fn validate_identity_known_key_passes() {
        let key = [1u8; 32];
        let mut msp = Msp::new("Org1MSP", "Org1");
        msp.root_public_keys.push(key);
        assert!(msp.validate_identity(&key).is_ok());
    }

    #[test]
    fn validate_identity_unknown_key_fails() {
        let msp = Msp::new("Org1MSP", "Org1");
        let foreign_key = [2u8; 32];
        assert_eq!(msp.validate_identity(&foreign_key), Err(MspError::UnknownKey));
    }

    #[test]
    fn validate_identity_revoked_key_fails() {
        let key = [3u8; 32];
        let serial = hex::encode(key);
        let mut msp = Msp::new("Org1MSP", "Org1");
        msp.root_public_keys.push(key);
        msp.revoked_serials.push(serial.clone());
        assert_eq!(
            msp.validate_identity(&key),
            Err(MspError::Revoked { serial })
        );
    }

    #[test]
    fn msp_role_serde_roundtrip() {
        let roles = [
            MspRole::Admin,
            MspRole::Member,
            MspRole::Client,
            MspRole::Peer,
            MspRole::Orderer,
        ];
        for role in roles {
            let json = serde_json::to_string(&role).unwrap();
            let decoded: MspRole = serde_json::from_str(&json).unwrap();
            assert_eq!(role, decoded);
        }
    }

    #[test]
    fn msp_role_serializes_snake_case() {
        assert_eq!(serde_json::to_string(&MspRole::Admin).unwrap(), "\"admin\"");
        assert_eq!(serde_json::to_string(&MspRole::Orderer).unwrap(), "\"orderer\"");
    }
}
