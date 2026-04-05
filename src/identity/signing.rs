//! Pluggable signing provider abstraction.
//!
//! `SigningProvider` decouples cryptographic operations from key storage.
//! The default `SoftwareSigningProvider` uses in-memory Ed25519 keys.
//! An HSM-backed provider can be swapped in via the `hsm` feature.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SigningError {
    #[error("signing failed: {0}")]
    SignFailed(String),
    #[error("verification failed: {0}")]
    VerifyFailed(String),
    #[error("key not available: {0}")]
    KeyNotAvailable(String),
}

/// Abstraction over cryptographic signing operations.
pub trait SigningProvider: Send + Sync {
    /// Sign `data` and return a 64-byte Ed25519 signature.
    fn sign(&self, data: &[u8]) -> Result<[u8; 64], SigningError>;

    /// Return the 32-byte public key.
    fn public_key(&self) -> [u8; 32];

    /// Verify `sig` over `data` using the provider's public key.
    fn verify(&self, data: &[u8], sig: &[u8; 64]) -> Result<bool, SigningError>;
}

/// Software-based signing provider using in-memory Ed25519 keys.
pub struct SoftwareSigningProvider {
    signing_key: ed25519_dalek::SigningKey,
}

impl SoftwareSigningProvider {
    /// Create a provider from an existing signing key.
    pub fn from_key(signing_key: ed25519_dalek::SigningKey) -> Self {
        Self { signing_key }
    }

    /// Generate a new random signing key.
    pub fn generate() -> Self {
        use rand::rngs::OsRng;
        Self {
            signing_key: ed25519_dalek::SigningKey::generate(&mut OsRng),
        }
    }
}

impl SigningProvider for SoftwareSigningProvider {
    fn sign(&self, data: &[u8]) -> Result<[u8; 64], SigningError> {
        use ed25519_dalek::Signer;
        let sig = self.signing_key.sign(data);
        Ok(sig.to_bytes())
    }

    fn public_key(&self) -> [u8; 32] {
        self.signing_key.verifying_key().to_bytes()
    }

    fn verify(&self, data: &[u8], sig: &[u8; 64]) -> Result<bool, SigningError> {
        use ed25519_dalek::{Signature, Verifier};
        let signature = Signature::from_bytes(sig);
        Ok(self.signing_key.verifying_key().verify(data, &signature).is_ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_and_verify_roundtrip() {
        let provider = SoftwareSigningProvider::generate();
        let data = b"hello world";

        let sig = provider.sign(data).unwrap();
        assert!(provider.verify(data, &sig).unwrap());
    }

    #[test]
    fn verify_wrong_data_fails() {
        let provider = SoftwareSigningProvider::generate();
        let sig = provider.sign(b"correct").unwrap();
        assert!(!provider.verify(b"wrong", &sig).unwrap());
    }

    #[test]
    fn public_key_is_32_bytes() {
        let provider = SoftwareSigningProvider::generate();
        let pk = provider.public_key();
        assert_eq!(pk.len(), 32);
    }

    #[test]
    fn from_known_key() {
        let key = ed25519_dalek::SigningKey::from_bytes(&[42u8; 32]);
        let provider = SoftwareSigningProvider::from_key(key);
        let sig = provider.sign(b"test").unwrap();
        assert!(provider.verify(b"test", &sig).unwrap());
    }

    #[test]
    fn trait_object_usage() {
        let provider: Box<dyn SigningProvider> = Box::new(SoftwareSigningProvider::generate());
        let sig = provider.sign(b"data").unwrap();
        assert!(provider.verify(b"data", &sig).unwrap());
    }
}
