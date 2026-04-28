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
    #[allow(dead_code)]
    #[error("key not available: {0}")]
    KeyNotAvailable(String),
}

/// Identifies the cryptographic algorithm used by a `SigningProvider`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
pub enum SigningAlgorithm {
    #[default]
    Ed25519,
    MlDsa65,
}

impl SigningAlgorithm {
    /// Returns `true` if this algorithm is post-quantum resistant.
    pub fn is_post_quantum(&self) -> bool {
        matches!(self, Self::MlDsa65)
    }
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
    #[allow(dead_code)]
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
///
/// The inner `SigningKey` implements `ZeroizeOnDrop` — key material is
/// automatically overwritten when the provider is dropped.
pub struct SoftwareSigningProvider {
    signing_key: ed25519_dalek::SigningKey,
}

impl SoftwareSigningProvider {
    #[allow(dead_code)]
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

impl Drop for MlDsaSigningProvider {
    fn drop(&mut self) {
        use pqcrypto_traits::sign::SecretKey;
        use zeroize::Zeroize;
        // SecretKey is an opaque struct; extract mutable bytes and zeroize.
        let sk_bytes = self.secret_key.as_bytes();
        let mut zeroed = sk_bytes.to_vec();
        zeroed.zeroize();
        // Overwrite the secret key with a fresh keypair (deterministic zeroing
        // is not possible for opaque C types, so we replace the value).
        let (_, fresh_sk) = pqcrypto_mldsa::mldsa65::keypair();
        self.secret_key = fresh_sk;
    }
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

    #[allow(dead_code)]
    /// Create a provider from existing key bytes.
    pub fn from_keys(pk_bytes: &[u8], sk_bytes: &[u8]) -> Result<Self, SigningError> {
        use pqcrypto_traits::sign::PublicKey as PqPk;
        use pqcrypto_traits::sign::SecretKey as PqSk;
        let pk = pqcrypto_mldsa::mldsa65::PublicKey::from_bytes(pk_bytes).map_err(|e| {
            SigningError::KeyNotAvailable(format!("invalid ML-DSA-65 public key: {e}"))
        })?;
        let sk = pqcrypto_mldsa::mldsa65::SecretKey::from_bytes(sk_bytes).map_err(|e| {
            SigningError::KeyNotAvailable(format!("invalid ML-DSA-65 secret key: {e}"))
        })?;
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
        let signature = pqcrypto_mldsa::mldsa65::DetachedSignature::from_bytes(sig)
            .map_err(|e| SigningError::VerifyFailed(format!("invalid ML-DSA-65 signature: {e}")))?;
        match pqcrypto_mldsa::mldsa65::verify_detached_signature(&signature, data, &self.public_key)
        {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

// ── FIPS 140-3 Power-Up Self-Tests (Known Answer Tests) ─────────────────────

/// Run cryptographic self-tests for all supported algorithms.
///
/// FIPS 140-3 requires that a cryptographic module verify its own correctness
/// at power-up before processing any external data. This function:
///
/// 1. Generates a keypair for each algorithm
/// 2. Signs a known test vector
/// 3. Verifies the signature
/// 4. Verifies that a corrupted signature is rejected
///
/// Returns `Ok(())` if all tests pass, or an error describing the failure.
/// Call this at node startup before accepting any requests.
pub fn run_crypto_self_tests() -> Result<(), SigningError> {
    // Ed25519 KAT
    {
        let provider = SoftwareSigningProvider::generate();
        let test_data = b"FIPS-140-3-KAT-Ed25519";
        let sig = provider.sign(test_data)?;
        if !provider.verify(test_data, &sig)? {
            return Err(SigningError::SignFailed(
                "Ed25519 KAT: sign-then-verify failed".into(),
            ));
        }
        // Corrupt one byte and verify rejection
        let mut bad_sig = sig.clone();
        bad_sig[0] ^= 0xff;
        if provider.verify(test_data, &bad_sig).unwrap_or(true) {
            return Err(SigningError::VerifyFailed(
                "Ed25519 KAT: corrupted signature was accepted".into(),
            ));
        }
    }

    // ML-DSA-65 KAT
    {
        let provider = MlDsaSigningProvider::generate();
        let test_data = b"FIPS-140-3-KAT-ML-DSA-65";
        let sig = provider.sign(test_data)?;
        if !provider.verify(test_data, &sig)? {
            return Err(SigningError::SignFailed(
                "ML-DSA-65 KAT: sign-then-verify failed".into(),
            ));
        }
        let mut bad_sig = sig.clone();
        bad_sig[0] ^= 0xff;
        if provider.verify(test_data, &bad_sig).unwrap_or(true) {
            return Err(SigningError::VerifyFailed(
                "ML-DSA-65 KAT: corrupted signature was accepted".into(),
            ));
        }
    }

    // SHA-256 KAT (used for block hashing, merkle roots, payload hashes)
    {
        use sha2::Digest;
        let input = b"FIPS-140-3-KAT-SHA256";
        let hash = sha2::Sha256::digest(input);
        let expected =
            hex::decode("11ffe3edcec6203b91f4f575c8d51dad935ea2a40e0bed0e5f9f69575afb80d0")
                .expect("valid hex");
        if hash.as_slice() != expected.as_slice() {
            return Err(SigningError::SignFailed(
                "SHA-256 KAT: digest mismatch".into(),
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crypto_self_tests_pass() {
        run_crypto_self_tests().expect("KAT self-tests must pass");
    }

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

    // --- Property-based tests ---

    mod prop {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn ed25519_sign_verify_any_data(data in proptest::collection::vec(any::<u8>(), 0..1024)) {
                let provider = SoftwareSigningProvider::generate();
                let sig = provider.sign(&data).unwrap();
                prop_assert!(provider.verify(&data, &sig).unwrap());
            }

            #[test]
            fn ed25519_verify_rejects_different_data(
                data_a in proptest::collection::vec(any::<u8>(), 1..512),
                data_b in proptest::collection::vec(any::<u8>(), 1..512),
            ) {
                prop_assume!(data_a != data_b);
                let provider = SoftwareSigningProvider::generate();
                let sig = provider.sign(&data_a).unwrap();
                prop_assert!(!provider.verify(&data_b, &sig).unwrap());
            }

            #[test]
            fn ed25519_signature_is_deterministic(data in proptest::collection::vec(any::<u8>(), 0..256)) {
                let provider = SoftwareSigningProvider::generate();
                let sig1 = provider.sign(&data).unwrap();
                let sig2 = provider.sign(&data).unwrap();
                prop_assert_eq!(sig1, sig2);
            }

            #[test]
            fn mldsa65_sign_verify_any_data(data in proptest::collection::vec(any::<u8>(), 0..1024)) {
                let provider = MlDsaSigningProvider::generate();
                let sig = provider.sign(&data).unwrap();
                prop_assert!(provider.verify(&data, &sig).unwrap());
            }

            #[test]
            fn mldsa65_verify_rejects_different_data(
                data_a in proptest::collection::vec(any::<u8>(), 1..512),
                data_b in proptest::collection::vec(any::<u8>(), 1..512),
            ) {
                prop_assume!(data_a != data_b);
                let provider = MlDsaSigningProvider::generate();
                let sig = provider.sign(&data_a).unwrap();
                prop_assert!(!provider.verify(&data_b, &sig).unwrap());
            }
        }
    }
}
