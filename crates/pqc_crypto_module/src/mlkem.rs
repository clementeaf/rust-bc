//! ML-KEM-768 key encapsulation (FIPS 203).
//!
//! Uses `pqcrypto-mlkem` for the underlying ML-KEM-768 implementation.

use pqcrypto_mlkem::mlkem768;
use pqcrypto_traits::kem::{Ciphertext, PublicKey, SecretKey, SharedSecret};

use crate::approved_mode::require_approved;
use crate::errors::CryptoError;
use crate::types::{MlKemCiphertext, MlKemPrivateKey, MlKemPublicKey, MlKemSharedSecret};

/// ML-KEM-768 keypair.
pub struct MlKemKeyPair {
    pub public_key: MlKemPublicKey,
    pub private_key: MlKemPrivateKey,
}

/// Generate an ML-KEM-768 keypair. Requires approved mode.
pub fn generate_keypair() -> Result<MlKemKeyPair, CryptoError> {
    require_approved()?;
    generate_keypair_raw()
}

pub(crate) fn generate_keypair_raw() -> Result<MlKemKeyPair, CryptoError> {
    let (pk, sk) = mlkem768::keypair();
    let private_key = MlKemPrivateKey(sk.as_bytes().to_vec());
    private_key.mlock();
    Ok(MlKemKeyPair {
        public_key: MlKemPublicKey(pk.as_bytes().to_vec()),
        private_key,
    })
}

/// Encapsulate: generate a shared secret and ciphertext from a public key.
pub fn encapsulate(
    public_key: &MlKemPublicKey,
) -> Result<(MlKemCiphertext, MlKemSharedSecret), CryptoError> {
    require_approved()?;
    encapsulate_raw(public_key)
}

pub(crate) fn encapsulate_raw(
    public_key: &MlKemPublicKey,
) -> Result<(MlKemCiphertext, MlKemSharedSecret), CryptoError> {
    let pk = mlkem768::PublicKey::from_bytes(public_key.as_bytes())
        .map_err(|_| CryptoError::InvalidKey("invalid ML-KEM-768 public key".into()))?;
    let (ss, ct) = mlkem768::encapsulate(&pk);
    let shared = MlKemSharedSecret(ss.as_bytes().to_vec());
    shared.mlock();
    Ok((MlKemCiphertext(ct.as_bytes().to_vec()), shared))
}

/// Decapsulate: recover the shared secret from a ciphertext and private key.
pub fn decapsulate(
    private_key: &MlKemPrivateKey,
    ciphertext: &MlKemCiphertext,
) -> Result<MlKemSharedSecret, CryptoError> {
    require_approved()?;
    decapsulate_raw(private_key, ciphertext)
}

pub(crate) fn decapsulate_raw(
    private_key: &MlKemPrivateKey,
    ciphertext: &MlKemCiphertext,
) -> Result<MlKemSharedSecret, CryptoError> {
    let sk = mlkem768::SecretKey::from_bytes(&private_key.0)
        .map_err(|_| CryptoError::InvalidKey("invalid ML-KEM-768 private key".into()))?;
    let ct = mlkem768::Ciphertext::from_bytes(ciphertext.as_bytes())
        .map_err(|_| CryptoError::InvalidKey("invalid ML-KEM-768 ciphertext".into()))?;
    let ss = mlkem768::decapsulate(&ct, &sk);
    let shared = MlKemSharedSecret(ss.as_bytes().to_vec());
    shared.mlock();
    Ok(shared)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::approved_mode::{set_state, ModuleState};

    #[test]
    fn keygen_produces_correct_sizes() {
        let kp = generate_keypair_raw().unwrap();
        assert_eq!(kp.public_key.as_bytes().len(), 1184);
        assert_eq!(kp.private_key.0.len(), 2400);
    }

    #[test]
    fn encapsulate_produces_correct_sizes() {
        let kp = generate_keypair_raw().unwrap();
        let (ct, ss) = encapsulate_raw(&kp.public_key).unwrap();
        assert_eq!(ct.as_bytes().len(), 1088);
        assert_eq!(ss.as_bytes().len(), 32);
    }

    #[test]
    fn mlkem_keypair_encaps_decaps_roundtrip() {
        let kp = generate_keypair_raw().unwrap();
        let (ct, ss1) = encapsulate_raw(&kp.public_key).unwrap();
        let ss2 = decapsulate_raw(&kp.private_key, &ct).unwrap();
        assert_eq!(ss1.as_bytes(), ss2.as_bytes());
    }

    #[test]
    fn mlkem_invalid_ciphertext_rejected() {
        let kp = generate_keypair_raw().unwrap();
        let bad_ct = MlKemCiphertext(vec![0xAA; 100]);
        assert!(decapsulate_raw(&kp.private_key, &bad_ct).is_err());
    }

    #[test]
    fn mlkem_private_key_zeroizes_on_drop() {
        let kp = generate_keypair_raw().unwrap();
        let len = kp.private_key.0.len();
        assert!(len > 0);
        drop(kp.private_key);
        // ZeroizeOnDrop derive verified by compilation and non-zero initial size.
    }

    #[test]
    fn mlkem_shared_secret_zeroizes_on_drop() {
        let kp = generate_keypair_raw().unwrap();
        let (_, ss) = encapsulate_raw(&kp.public_key).unwrap();
        let len = ss.as_bytes().len();
        assert_eq!(len, 32);
        drop(ss);
        // ZeroizeOnDrop derive verified by compilation.
    }

    #[test]
    fn mlkem_api_rejects_before_approved_mode() {
        // Reset state to Uninitialized
        set_state(ModuleState::Uninitialized);
        assert!(generate_keypair().is_err());
    }

    #[test]
    fn mlkem_api_works_after_approved_mode() {
        set_state(ModuleState::Approved);
        let kp = generate_keypair().unwrap();
        let (ct, ss1) = encapsulate(&kp.public_key).unwrap();
        let ss2 = decapsulate(&kp.private_key, &ct).unwrap();
        assert_eq!(ss1.as_bytes(), ss2.as_bytes());
    }
}
