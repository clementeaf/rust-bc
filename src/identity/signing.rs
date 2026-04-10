//! Pluggable signing provider abstraction.
//!
//! `SigningProvider` decouples cryptographic operations from key storage.
//! Implementations exist for Ed25519 (`SoftwareSigningProvider`) and
//! ML-DSA (`MlDsaSigningProvider`) for post-quantum readiness.
//!
//! Signatures and public keys are variable-length (`Vec<u8>`) to support
//! algorithms with different output sizes (Ed25519: 64-byte sig / 32-byte pk,
//! ML-DSA-65: 3309-byte sig / 1952-byte pk).

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

/// Identifies the cryptographic algorithm used by a `SigningProvider`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum SigningAlgorithm {
    Ed25519,
    MlDsa65,
}

impl std::fmt::Display for SigningAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ed25519 => write!(f, "Ed25519"),
            Self::MlDsa65 => write!(f, "ML-DSA-65"),
        }
    }
}

/// Abstraction over cryptographic signing operations.
///
/// Signatures and public keys are returned as `Vec<u8>` to accommodate
/// algorithms with different output sizes.
pub trait SigningProvider: Send + Sync {
    /// The algorithm this provider uses.
    fn algorithm(&self) -> SigningAlgorithm;

    /// Sign `data` and return the signature bytes.
    fn sign(&self, data: &[u8]) -> Result<Vec<u8>, SigningError>;

    /// Return the public key bytes.
    fn public_key(&self) -> Vec<u8>;

    /// Verify `sig` over `data` using the provider's public key.
    fn verify(&self, data: &[u8], sig: &[u8]) -> Result<bool, SigningError>;
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
    fn algorithm(&self) -> SigningAlgorithm {
        SigningAlgorithm::Ed25519
    }

    fn sign(&self, data: &[u8]) -> Result<Vec<u8>, SigningError> {
        use ed25519_dalek::Signer;
        let sig = self.signing_key.sign(data);
        Ok(sig.to_bytes().to_vec())
    }

    fn public_key(&self) -> Vec<u8> {
        self.signing_key.verifying_key().to_bytes().to_vec()
    }

    fn verify(&self, data: &[u8], sig: &[u8]) -> Result<bool, SigningError> {
        use ed25519_dalek::{Signature, Verifier};
        let sig_bytes: [u8; 64] = sig
            .try_into()
            .map_err(|_| SigningError::VerifyFailed("Ed25519 signature must be 64 bytes".into()))?;
        let signature = Signature::from_bytes(&sig_bytes);
        Ok(self
            .signing_key
            .verifying_key()
            .verify(data, &signature)
            .is_ok())
    }
}

/// Post-quantum signing provider using ML-DSA-65 (FIPS 204, security level 3).
///
/// Key and signature sizes:
/// - Public key: 1952 bytes
/// - Secret key: 4032 bytes
/// - Signature:  3309 bytes
pub struct MlDsaSigningProvider {
    public_key: pqcrypto_mldsa::mldsa65::PublicKey,
    secret_key: pqcrypto_mldsa::mldsa65::SecretKey,
}

impl MlDsaSigningProvider {
    /// Generate a new random ML-DSA-65 keypair.
    pub fn generate() -> Self {
        let (pk, sk) = pqcrypto_mldsa::mldsa65::keypair();
        Self {
            public_key: pk,
            secret_key: sk,
        }
    }

    /// Create a provider from existing key bytes.
    pub fn from_keys(pk_bytes: &[u8], sk_bytes: &[u8]) -> Result<Self, SigningError> {
        use pqcrypto_traits::sign::PublicKey as PqPk;
        use pqcrypto_traits::sign::SecretKey as PqSk;
        let pk = pqcrypto_mldsa::mldsa65::PublicKey::from_bytes(pk_bytes)
            .map_err(|e| SigningError::KeyNotAvailable(format!("invalid ML-DSA-65 public key: {e}")))?;
        let sk = pqcrypto_mldsa::mldsa65::SecretKey::from_bytes(sk_bytes)
            .map_err(|e| SigningError::KeyNotAvailable(format!("invalid ML-DSA-65 secret key: {e}")))?;
        Ok(Self {
            public_key: pk,
            secret_key: sk,
        })
    }
}

impl SigningProvider for MlDsaSigningProvider {
    fn algorithm(&self) -> SigningAlgorithm {
        SigningAlgorithm::MlDsa65
    }

    fn sign(&self, data: &[u8]) -> Result<Vec<u8>, SigningError> {
        use pqcrypto_traits::sign::DetachedSignature;
        let sig = pqcrypto_mldsa::mldsa65::detached_sign(data, &self.secret_key);
        Ok(sig.as_bytes().to_vec())
    }

    fn public_key(&self) -> Vec<u8> {
        use pqcrypto_traits::sign::PublicKey;
        self.public_key.as_bytes().to_vec()
    }

    fn verify(&self, data: &[u8], sig: &[u8]) -> Result<bool, SigningError> {
        use pqcrypto_traits::sign::DetachedSignature;
        let signature =
            pqcrypto_mldsa::mldsa65::DetachedSignature::from_bytes(sig)
                .map_err(|e| SigningError::VerifyFailed(format!("invalid ML-DSA-65 signature: {e}")))?;
        match pqcrypto_mldsa::mldsa65::verify_detached_signature(&signature, data, &self.public_key) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
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
    fn ed25519_public_key_is_32_bytes() {
        let provider = SoftwareSigningProvider::generate();
        let pk = provider.public_key();
        assert_eq!(pk.len(), 32);
    }

    #[test]
    fn ed25519_signature_is_64_bytes() {
        let provider = SoftwareSigningProvider::generate();
        let sig = provider.sign(b"test").unwrap();
        assert_eq!(sig.len(), 64);
    }

    #[test]
    fn ed25519_algorithm_identifier() {
        let provider = SoftwareSigningProvider::generate();
        assert_eq!(provider.algorithm(), SigningAlgorithm::Ed25519);
    }

    #[test]
    fn from_known_key() {
        let key = ed25519_dalek::SigningKey::from_bytes(&[42u8; 32]);
        let provider = SoftwareSigningProvider::from_key(key);
        let sig = provider.sign(b"test").unwrap();
        assert!(provider.verify(b"test", &sig).unwrap());
    }

    #[test]
    fn verify_rejects_wrong_length_signature() {
        let provider = SoftwareSigningProvider::generate();
        let bad_sig = vec![0u8; 32]; // wrong length
        assert!(provider.verify(b"data", &bad_sig).is_err());
    }

    #[test]
    fn trait_object_usage() {
        let provider: Box<dyn SigningProvider> = Box::new(SoftwareSigningProvider::generate());
        let sig = provider.sign(b"data").unwrap();
        assert!(provider.verify(b"data", &sig).unwrap());
    }

    // --- ML-DSA-65 tests ---

    #[test]
    fn mldsa65_sign_and_verify_roundtrip() {
        let provider = MlDsaSigningProvider::generate();
        let data = b"post-quantum hello";
        let sig = provider.sign(data).unwrap();
        assert!(provider.verify(data, &sig).unwrap());
    }

    #[test]
    fn mldsa65_verify_wrong_data_fails() {
        let provider = MlDsaSigningProvider::generate();
        let sig = provider.sign(b"correct").unwrap();
        assert!(!provider.verify(b"wrong", &sig).unwrap());
    }

    #[test]
    fn mldsa65_algorithm_identifier() {
        let provider = MlDsaSigningProvider::generate();
        assert_eq!(provider.algorithm(), SigningAlgorithm::MlDsa65);
    }

    #[test]
    fn mldsa65_signature_is_3309_bytes() {
        let provider = MlDsaSigningProvider::generate();
        let sig = provider.sign(b"test").unwrap();
        assert_eq!(sig.len(), 3309);
    }

    #[test]
    fn mldsa65_public_key_is_1952_bytes() {
        let provider = MlDsaSigningProvider::generate();
        assert_eq!(provider.public_key().len(), 1952);
    }

    #[test]
    fn mldsa65_verify_rejects_wrong_signature() {
        let provider = MlDsaSigningProvider::generate();
        // A wrong-length or garbage signature must not verify successfully
        let bad_sig = vec![0u8; 64];
        let result = provider.verify(b"data", &bad_sig);
        assert!(result.is_err() || matches!(result, Ok(false)));
    }

    #[test]
    fn mldsa65_from_keys_roundtrip() {
        let provider = MlDsaSigningProvider::generate();
        let pk = provider.public_key();
        use pqcrypto_traits::sign::SecretKey;
        let sk = provider.secret_key.as_bytes().to_vec();
        let restored = MlDsaSigningProvider::from_keys(&pk, &sk).unwrap();
        let sig = restored.sign(b"roundtrip").unwrap();
        assert!(restored.verify(b"roundtrip", &sig).unwrap());
    }

    #[test]
    fn mldsa65_trait_object_usage() {
        let provider: Box<dyn SigningProvider> = Box::new(MlDsaSigningProvider::generate());
        let sig = provider.sign(b"pqc data").unwrap();
        assert!(provider.verify(b"pqc data", &sig).unwrap());
    }

    #[test]
    fn cross_provider_signatures_incompatible() {
        let ed = SoftwareSigningProvider::generate();
        let pqc = MlDsaSigningProvider::generate();
        let ed_sig = ed.sign(b"data").unwrap();
        let pqc_sig = pqc.sign(b"data").unwrap();
        // Ed25519 sig on ML-DSA provider: wrong size or fails verification
        let pqc_result = pqc.verify(b"data", &ed_sig);
        assert!(pqc_result.is_err() || matches!(pqc_result, Ok(false)));
        // ML-DSA sig on Ed25519 provider: wrong size (must be exactly 64 bytes)
        assert!(ed.verify(b"data", &pqc_sig).is_err());
    }
}
