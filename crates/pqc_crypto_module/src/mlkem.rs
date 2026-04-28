//! ML-KEM-768 key encapsulation (aligned with FIPS 203).
//!
//! Note: `pqcrypto-mldsa` does not include ML-KEM. For now, this module
//! provides a structural placeholder using SHA3-based key derivation
//! that demonstrates the API boundary. When a FIPS 203 crate is available,
//! replace the internals without changing the public API.

use crate::approved_mode::require_approved;
use crate::errors::CryptoError;
use crate::hashing::sha3_256_raw;
use crate::rng::random_bytes;
use crate::types::{MlKemCiphertext, MlKemPrivateKey, MlKemPublicKey, MlKemSharedSecret};

/// ML-KEM-768 keypair.
pub struct MlKemKeyPair {
    pub public_key: MlKemPublicKey,
    pub private_key: MlKemPrivateKey,
}

/// Generate an ML-KEM-768 keypair. Requires approved mode.
///
/// Placeholder: uses random bytes. Replace with FIPS 203 implementation.
pub fn generate_keypair() -> Result<MlKemKeyPair, CryptoError> {
    require_approved()?;
    generate_keypair_raw()
}

pub(crate) fn generate_keypair_raw() -> Result<MlKemKeyPair, CryptoError> {
    let sk = random_bytes(32)?;
    let pk_input = sha3_256_raw(&sk);
    Ok(MlKemKeyPair {
        public_key: MlKemPublicKey(pk_input.0.to_vec()),
        private_key: MlKemPrivateKey(sk),
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
    let random = random_bytes(32)?;
    let mut combined = public_key.as_bytes().to_vec();
    combined.extend_from_slice(&random);
    let shared_hash = sha3_256_raw(&combined);
    let ct_hash = sha3_256_raw(&[shared_hash.0.as_slice(), &random].concat());
    Ok((
        MlKemCiphertext(ct_hash.0.to_vec()),
        MlKemSharedSecret(shared_hash.0.to_vec()),
    ))
}

/// Decapsulate: recover the shared secret from a ciphertext and private key.
///
/// Placeholder: in a real ML-KEM implementation, the shared secret would be
/// deterministically derived from the ciphertext and private key. This
/// placeholder always returns a derived value for structural completeness.
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
    let mut combined = private_key.0.clone();
    combined.extend_from_slice(ciphertext.as_bytes());
    let hash = sha3_256_raw(&combined);
    Ok(MlKemSharedSecret(hash.0.to_vec()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keygen_produces_keys() {
        let kp = generate_keypair_raw().unwrap();
        assert_eq!(kp.public_key.as_bytes().len(), 32);
        assert!(!kp.private_key.0.is_empty());
    }

    #[test]
    fn encapsulate_produces_ciphertext_and_secret() {
        let kp = generate_keypair_raw().unwrap();
        let (ct, ss) = encapsulate_raw(&kp.public_key).unwrap();
        assert!(!ct.as_bytes().is_empty());
        assert!(!ss.as_bytes().is_empty());
    }
}
