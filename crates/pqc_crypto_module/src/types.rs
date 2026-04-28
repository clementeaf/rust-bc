//! Cryptographic types with zeroization and memory locking.

use zeroize::{Zeroize, ZeroizeOnDrop};

/// Best-effort mlock: pin memory pages to prevent swap-out of key material.
/// Fails silently if RLIMIT_MEMLOCK is insufficient — this is expected on
/// unprivileged processes and is not required at FIPS 140-3 Security Level 1.
fn mlock_buffer(buf: &[u8]) {
    if buf.is_empty() {
        return;
    }
    // SAFETY: buf points to a valid, initialized allocation. mlock is a
    // read-only advisory syscall that does not modify the buffer.
    unsafe {
        libc::mlock(buf.as_ptr().cast(), buf.len());
    }
}

/// ML-DSA-65 public key (1952 bytes).
#[derive(Debug, Clone)]
pub struct MldsaPublicKey(pub Vec<u8>);

impl MldsaPublicKey {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, crate::errors::CryptoError> {
        if bytes.len() != 1952 {
            return Err(crate::errors::CryptoError::InvalidKey(format!(
                "ML-DSA-65 public key must be 1952 bytes, got {}",
                bytes.len()
            )));
        }
        Ok(Self(bytes.to_vec()))
    }
}

/// ML-DSA-65 private key (4032 bytes). Zeroized on drop, mlock'd to prevent swap.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct MldsaPrivateKey(pub Vec<u8>);

impl MldsaPrivateKey {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Lock memory pages to prevent swap-out. Called after construction.
    pub fn mlock(&self) {
        mlock_buffer(&self.0);
    }
}

impl std::fmt::Debug for MldsaPrivateKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MldsaPrivateKey([REDACTED; {} bytes])", self.0.len())
    }
}

/// ML-DSA-65 signature (3309 bytes).
#[derive(Debug, Clone)]
pub struct MldsaSignature(pub Vec<u8>);

impl MldsaSignature {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, crate::errors::CryptoError> {
        if bytes.len() != 3309 {
            return Err(crate::errors::CryptoError::InvalidSignature);
        }
        Ok(Self(bytes.to_vec()))
    }
}

/// ML-DSA-65 keypair.
pub struct MldsaKeyPair {
    pub public_key: MldsaPublicKey,
    pub private_key: MldsaPrivateKey,
}

/// SHA3-256 hash output (32 bytes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hash256(pub [u8; 32]);

impl Hash256 {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

/// ML-KEM-768 public key.
#[derive(Debug, Clone)]
pub struct MlKemPublicKey(pub Vec<u8>);

impl MlKemPublicKey {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// ML-KEM-768 private key (2400 bytes). Zeroized on drop, mlock'd to prevent swap.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct MlKemPrivateKey(pub Vec<u8>);

impl MlKemPrivateKey {
    /// Lock memory pages to prevent swap-out. Called after construction.
    pub fn mlock(&self) {
        mlock_buffer(&self.0);
    }
}

impl std::fmt::Debug for MlKemPrivateKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MlKemPrivateKey([REDACTED; {} bytes])", self.0.len())
    }
}

/// ML-KEM-768 ciphertext.
#[derive(Debug, Clone)]
pub struct MlKemCiphertext(pub Vec<u8>);

impl MlKemCiphertext {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// ML-KEM-768 shared secret (32 bytes). Zeroized on drop, mlock'd to prevent swap.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct MlKemSharedSecret(pub Vec<u8>);

impl MlKemSharedSecret {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Lock memory pages to prevent swap-out. Called after construction.
    pub fn mlock(&self) {
        mlock_buffer(&self.0);
    }
}

impl std::fmt::Debug for MlKemSharedSecret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MlKemSharedSecret([REDACTED])")
    }
}
