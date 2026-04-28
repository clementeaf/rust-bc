//! Legacy non-approved algorithms.
//!
//! These algorithms are NOT part of the approved cryptographic boundary.
//! They exist solely for backwards compatibility with pre-PQC blocks and
//! legacy wallet operations. Production strict-PQC mode must not rely on them.
//!
//! **Do not use for new functionality.** Migrate to approved APIs.

/// Legacy Ed25519 operations (non-approved).
///
/// Re-exports from `ed25519_dalek` for legacy signature verification only.
pub mod ed25519 {
    pub use ed25519_dalek::{
        Signature, SignatureError, Signer, SigningKey, Verifier, VerifyingKey,
    };
}

/// Legacy SHA-256 hashing (non-approved).
///
/// Re-exports `sha2::Sha256` for legacy block hashing and Merkle roots.
pub mod sha256 {
    pub use sha2::{Digest, Sha256};
}

/// Legacy HMAC-SHA256 (non-approved).
pub mod hmac {
    pub use hmac::{Hmac, Mac};
    pub use sha2::Sha256;
    pub type HmacSha256 = Hmac<Sha256>;
}

/// Legacy random number generation (non-approved).
///
/// Re-exports for key generation in legacy Ed25519 paths.
pub mod rng {
    pub use rand::rngs::OsRng;
    pub use rand::seq::SliceRandom;
    pub use rand::Rng;
    pub use rand::RngCore;
    pub use rand_core;
}

/// Legacy ML-DSA direct access (non-approved outside module boundary).
///
/// Re-exports for legacy code that directly calls pqcrypto_mldsa.
/// New code should use `api::sign_message` / `api::verify_signature` instead.
pub mod mldsa_raw {
    pub use pqcrypto_mldsa::mldsa65;
    pub use pqcrypto_traits::sign::{DetachedSignature, PublicKey, SecretKey};
}
