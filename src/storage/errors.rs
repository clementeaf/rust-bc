//! Storage layer error types and result type
//!
//! Defines custom error types for RocksDB operations with proper error handling
//! and recovery strategies.

use thiserror::Error;

/// Storage-specific errors
#[derive(Error, Debug)]
pub enum StorageError {
    /// RocksDB operation failed
    #[error("RocksDB error: {0}")]
    RocksDbError(String),

    /// Serialization error (CBOR/bincode)
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Deserialization error
    #[error("Deserialization error: {0}")]
    DeserializationError(String),

    /// Key not found in storage
    #[error("Key not found: {0}")]
    KeyNotFound(String),

    /// Invalid key format
    #[error("Invalid key format: {0}")]
    InvalidKeyFormat(String),

    /// Block height mismatch
    #[error("Block height mismatch: expected {expected}, got {actual}")]
    BlockHeightMismatch { expected: u64, actual: u64 },

    /// Transaction validation failed
    #[error("Transaction validation failed: {0}")]
    TransactionValidationFailed(String),

    /// Identity record not found
    #[error("Identity record not found: {0}")]
    IdentityNotFound(String),

    /// Credential not found
    #[error("Credential not found: {0}")]
    CredentialNotFound(String),

    /// Corrupted data detected
    #[error("Corrupted data: {0}")]
    DataCorrupted(String),

    /// Batch operation failed
    #[error("Batch operation failed: {0}")]
    BatchOperationFailed(String),

    /// Column family not found
    #[error("Column family not found: {0}")]
    ColumnFamilyNotFound(String),

    /// Schema version mismatch
    #[error("Schema version mismatch: expected {expected}, got {actual}")]
    SchemaMismatch { expected: u32, actual: u32 },

    /// Checkpoint error
    #[error("Checkpoint error: {0}")]
    CheckpointError(String),

    /// I/O error
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// Generic error
    #[error("Storage error: {0}")]
    Other(String),
}

/// Result type for storage operations
pub type StorageResult<T> = Result<T, StorageError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = StorageError::KeyNotFound("block:123".to_string());
        assert_eq!(err.to_string(), "Key not found: block:123");
    }

    #[test]
    fn test_block_height_mismatch_error() {
        let err = StorageError::BlockHeightMismatch {
            expected: 100,
            actual: 99,
        };
        assert_eq!(
            err.to_string(),
            "Block height mismatch: expected 100, got 99"
        );
    }

    #[test]
    fn test_storage_result_ok() {
        let result: StorageResult<i32> = Ok(42);
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_storage_result_err() {
        let result: StorageResult<i32> = Err(StorageError::KeyNotFound("test".to_string()));
        assert!(result.is_err());
    }
}
