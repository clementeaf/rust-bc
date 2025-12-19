//! Key management for identity system
//!
//! Supports Ed25519 keypair generation, storage, and rotation

use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::rngs::OsRng;

/// Public key information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicKeyInfo {
    /// Public key bytes (32 bytes for Ed25519)
    pub public_key: [u8; 32],
    /// Key creation timestamp
    pub created_at: u64,
    /// Key expiration (optional)
    pub expires_at: Option<u64>,
    /// Whether this key is currently active
    pub is_active: bool,
}

/// Key pair for signing operations
#[derive(Debug, Clone)]
pub struct KeyPair {
    /// Active signing key
    pub signing_key: SigningKey,
    /// Public key bytes
    pub public_key: [u8; 32],
    /// Creation timestamp
    pub created_at: u64,
}

/// Key manager for identity
pub struct KeyManager {
    /// Active keypair
    active_key: KeyPair,
    /// Previous (retired) keypairs for verification
    retired_keys: Vec<PublicKeyInfo>,
}

impl KeyManager {
    /// Create a new key manager with generated keypair
    pub fn new(timestamp: u64) -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        let public_key = verifying_key.to_bytes();

        let active_key = KeyPair {
            signing_key,
            public_key,
            created_at: timestamp,
        };

        KeyManager {
            active_key,
            retired_keys: Vec::new(),
        }
    }

    /// Get the current public key
    pub fn public_key(&self) -> [u8; 32] {
        self.active_key.public_key
    }

    /// Get all public keys (active + retired)
    pub fn all_public_keys(&self) -> Vec<PublicKeyInfo> {
        let mut keys = vec![PublicKeyInfo {
            public_key: self.active_key.public_key,
            created_at: self.active_key.created_at,
            expires_at: None,
            is_active: true,
        }];

        keys.extend(self.retired_keys.clone());
        keys
    }

    /// Rotate to a new keypair
    pub fn rotate_key(&mut self, timestamp: u64) {
        // Archive current key as retired
        let retired = PublicKeyInfo {
            public_key: self.active_key.public_key,
            created_at: self.active_key.created_at,
            expires_at: Some(timestamp),
            is_active: false,
        };
        self.retired_keys.push(retired);

        // Generate new active key
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        let public_key = verifying_key.to_bytes();

        self.active_key = KeyPair {
            signing_key,
            public_key,
            created_at: timestamp,
        };
    }

    /// Get the active signing key for creating signatures
    pub fn signing_key(&self) -> &SigningKey {
        &self.active_key.signing_key
    }

    /// Sign data with the active key
    pub fn sign(&self, data: &[u8]) -> [u8; 64] {
        let signature = self.active_key.signing_key.sign(data);
        signature.to_bytes()
    }

    /// Verify a signature with the active key
    pub fn verify(&self, data: &[u8], signature: &[u8; 64]) -> bool {
        let verifying_key = self.active_key.signing_key.verifying_key();
        verifying_key.verify(data, signature).is_ok()
    }

    /// Verify a signature with any available key (including retired)
    pub fn verify_with_any_key(&self, data: &[u8], signature: &[u8; 64]) -> bool {
        // Try active key first
        if self.verify(data, signature) {
            return true;
        }

        // Try retired keys
        for retired_key in &self.retired_keys {
            let verifying_key = VerifyingKey::from_bytes(&retired_key.public_key).ok()?;
            if verifying_key.verify(data, signature).is_ok() {
                return true;
            }
        }

        false
    }

    /// Get key creation timestamp
    pub fn key_created_at(&self) -> u64 {
        self.active_key.created_at
    }

    /// Number of retired keys
    pub fn retired_key_count(&self) -> usize {
        self.retired_keys.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let km = KeyManager::new(1000);
        assert_eq!(km.key_created_at(), 1000);
        assert_eq!(km.public_key().len(), 32);
    }

    #[test]
    fn test_sign_and_verify() {
        let km = KeyManager::new(1000);
        let data = b"test message";
        let signature = km.sign(data);
        assert!(km.verify(data, &signature));
    }

    #[test]
    fn test_verify_fails_with_wrong_data() {
        let km = KeyManager::new(1000);
        let data = b"test message";
        let signature = km.sign(data);
        let wrong_data = b"different message";
        assert!(!km.verify(wrong_data, &signature));
    }

    #[test]
    fn test_key_rotation() {
        let mut km = KeyManager::new(1000);
        let old_key = km.public_key();
        km.rotate_key(2000);
        let new_key = km.public_key();
        assert_ne!(old_key, new_key);
        assert_eq!(km.retired_key_count(), 1);
    }

    #[test]
    fn test_verify_after_rotation() {
        let mut km = KeyManager::new(1000);
        let data = b"test message";
        let signature = km.sign(data);
        km.rotate_key(2000);
        // Verify with any key should still work
        assert!(km.verify_with_any_key(data, &signature));
    }

    #[test]
    fn test_all_public_keys() {
        let mut km = KeyManager::new(1000);
        km.rotate_key(2000);
        km.rotate_key(3000);
        let keys = km.all_public_keys();
        assert_eq!(keys.len(), 3); // 1 active + 2 retired
        assert!(keys[0].is_active);
        assert!(!keys[1].is_active);
        assert!(!keys[2].is_active);
    }

    #[test]
    fn test_signature_format() {
        let km = KeyManager::new(1000);
        let data = b"test";
        let signature = km.sign(data);
        assert_eq!(signature.len(), 64); // Ed25519 signature is 64 bytes
    }

    #[test]
    fn test_multiple_rotations() {
        let mut km = KeyManager::new(1000);
        for i in 1..=5 {
            km.rotate_key(1000 + i as u64 * 1000);
        }
        assert_eq!(km.retired_key_count(), 5);
    }
}
