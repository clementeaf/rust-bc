//! Public API — single entry point for all approved cryptographic operations.
//!
//! All DLT code should call functions from this module only.
//! No direct use of `pqcrypto`, `sha3`, `rand`, or `ed25519` crates.

use crate::approved_mode::{require_approved, set_state, ModuleState};
use crate::errors::CryptoError;
use crate::types::*;

/// Initialize the module into approved mode by running self-tests.
///
/// Must be called once at startup before any crypto operation.
/// If self-tests fail, the module enters `Error` state and all
/// subsequent operations will be rejected.
pub fn initialize_approved_mode() -> Result<(), CryptoError> {
    set_state(ModuleState::SelfTesting);

    match crate::self_tests::run_all() {
        Ok(()) => {
            set_state(ModuleState::Approved);
            Ok(())
        }
        Err(e) => {
            set_state(ModuleState::Error);
            Err(e)
        }
    }
}

/// Generate an ML-DSA-65 keypair.
pub fn generate_mldsa_keypair() -> Result<MldsaKeyPair, CryptoError> {
    crate::mldsa::generate_keypair()
}

/// Sign a message with ML-DSA-65.
pub fn sign_message(
    private_key: &MldsaPrivateKey,
    message: &[u8],
) -> Result<MldsaSignature, CryptoError> {
    crate::mldsa::sign_message(private_key, message)
}

/// Verify a signature with ML-DSA-65.
pub fn verify_signature(
    public_key: &MldsaPublicKey,
    message: &[u8],
    signature: &MldsaSignature,
) -> Result<(), CryptoError> {
    crate::mldsa::verify_signature(public_key, message, signature)
}

/// Compute SHA3-256 hash.
pub fn sha3_256(data: &[u8]) -> Result<Hash256, CryptoError> {
    crate::hashing::sha3_256(data)
}

/// Generate an ML-KEM-768 keypair.
pub fn generate_mlkem_keypair() -> Result<crate::mlkem::MlKemKeyPair, CryptoError> {
    crate::mlkem::generate_keypair()
}

/// ML-KEM-768 encapsulation.
pub fn mlkem_encapsulate(
    public_key: &MlKemPublicKey,
) -> Result<(MlKemCiphertext, MlKemSharedSecret), CryptoError> {
    crate::mlkem::encapsulate(public_key)
}

/// ML-KEM-768 decapsulation.
pub fn mlkem_decapsulate(
    private_key: &MlKemPrivateKey,
    ciphertext: &MlKemCiphertext,
) -> Result<MlKemSharedSecret, CryptoError> {
    crate::mlkem::decapsulate(private_key, ciphertext)
}

/// Generate cryptographically secure random bytes.
pub fn random_bytes(n: usize) -> Result<Vec<u8>, CryptoError> {
    require_approved()?;
    crate::rng::random_bytes(n)
}
