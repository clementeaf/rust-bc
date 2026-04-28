//! ML-DSA-65 signing and verification (aligned with FIPS 204).

use pqcrypto_mldsa::mldsa65;
use pqcrypto_traits::sign::{DetachedSignature, PublicKey, SecretKey};

use crate::approved_mode::require_approved;
use crate::errors::CryptoError;
use crate::types::{MldsaKeyPair, MldsaPrivateKey, MldsaPublicKey, MldsaSignature};

/// Generate a new ML-DSA-65 keypair. Requires approved mode.
pub fn generate_keypair() -> Result<MldsaKeyPair, CryptoError> {
    require_approved()?;
    Ok(generate_keypair_raw())
}

/// Internal keygen without approved-mode check (for self-tests).
pub(crate) fn generate_keypair_raw() -> MldsaKeyPair {
    let (pk, sk) = mldsa65::keypair();
    let private_key = MldsaPrivateKey(sk.as_bytes().to_vec());
    private_key.mlock();
    MldsaKeyPair {
        public_key: MldsaPublicKey(pk.as_bytes().to_vec()),
        private_key,
    }
}

/// Sign a message with ML-DSA-65. Requires approved mode.
pub fn sign_message(
    private_key: &MldsaPrivateKey,
    message: &[u8],
) -> Result<MldsaSignature, CryptoError> {
    require_approved()?;
    sign_message_raw(private_key, message)
}

/// Internal sign without approved-mode check (for self-tests).
pub(crate) fn sign_message_raw(
    private_key: &MldsaPrivateKey,
    message: &[u8],
) -> Result<MldsaSignature, CryptoError> {
    let sk = mldsa65::SecretKey::from_bytes(&private_key.0)
        .map_err(|e| CryptoError::InvalidKey(format!("ML-DSA-65 secret key: {e}")))?;
    let sig = mldsa65::detached_sign(message, &sk);
    Ok(MldsaSignature(sig.as_bytes().to_vec()))
}

/// Verify a signature with ML-DSA-65. Requires approved mode.
pub fn verify_signature(
    public_key: &MldsaPublicKey,
    message: &[u8],
    signature: &MldsaSignature,
) -> Result<(), CryptoError> {
    require_approved()?;
    verify_signature_raw(public_key, message, signature)
}

/// Internal verify without approved-mode check (for self-tests).
pub(crate) fn verify_signature_raw(
    public_key: &MldsaPublicKey,
    message: &[u8],
    signature: &MldsaSignature,
) -> Result<(), CryptoError> {
    let pk = mldsa65::PublicKey::from_bytes(&public_key.0)
        .map_err(|_| CryptoError::InvalidKey("invalid ML-DSA-65 public key".into()))?;
    let sig = mldsa65::DetachedSignature::from_bytes(&signature.0)
        .map_err(|_| CryptoError::InvalidSignature)?;
    mldsa65::verify_detached_signature(&sig, message, &pk)
        .map_err(|_| CryptoError::VerificationFailed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keygen_produces_valid_sizes() {
        let kp = generate_keypair_raw();
        assert_eq!(kp.public_key.as_bytes().len(), 1952);
        assert_eq!(kp.private_key.as_bytes().len(), 4032);
    }

    #[test]
    fn sign_verify_roundtrip() {
        let kp = generate_keypair_raw();
        let sig = sign_message_raw(&kp.private_key, b"test").unwrap();
        assert_eq!(sig.as_bytes().len(), 3309);
        verify_signature_raw(&kp.public_key, b"test", &sig).unwrap();
    }

    #[test]
    fn wrong_message_fails() {
        let kp = generate_keypair_raw();
        let sig = sign_message_raw(&kp.private_key, b"correct").unwrap();
        assert!(verify_signature_raw(&kp.public_key, b"wrong", &sig).is_err());
    }
}
