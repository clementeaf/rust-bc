//! Cryptographic module error types.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("module not initialized — call initialize_approved_mode() first")]
    ModuleNotInitialized,
    #[error("module in error state — self-test failure")]
    ModuleInErrorState,
    #[error("self-test failed: {0}")]
    SelfTestFailed(String),
    #[error("invalid key: {0}")]
    InvalidKey(String),
    #[error("invalid signature")]
    InvalidSignature,
    #[error("signature verification failed")]
    VerificationFailed,
    #[error("RNG failure: {0}")]
    RngFailure(String),
    #[error("non-approved algorithm requested")]
    NonApprovedAlgorithm,
    #[error("serialization error: {0}")]
    SerializationError(String),
}
