//! Storage trait definitions
//!
//! Defines the BlockStore trait and related interfaces for storage operations.

use super::errors::StorageResult;
use serde::{Deserialize, Serialize};

/// Block structure for storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub height: u64,
    pub timestamp: u64,
    pub parent_hash: [u8; 32],
    pub merkle_root: [u8; 32],
    pub transactions: Vec<String>,
    pub proposer: String,
    pub signature: [u8; 64],
}

/// Transaction structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: String,
    pub block_height: u64,
    pub timestamp: u64,
    pub input_did: String,
    pub output_recipient: String,
    pub amount: u64,
    pub state: String,
}

/// Identity record structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityRecord {
    pub did: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub status: String,
}

/// Credential structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credential {
    pub id: String,
    pub issuer_did: String,
    pub subject_did: String,
    pub cred_type: String,
    pub issued_at: u64,
    pub expires_at: u64,
    pub revoked_at: Option<u64>,
}

/// BlockStore trait - main storage interface
pub trait BlockStore: Send + Sync {
    /// Write a block to storage
    fn write_block(&self, block: &Block) -> StorageResult<()>;

    /// Read a block by height
    fn read_block(&self, height: u64) -> StorageResult<Block>;

    /// Write a transaction
    fn write_transaction(&self, tx: &Transaction) -> StorageResult<()>;

    /// Read a transaction by ID
    fn read_transaction(&self, tx_id: &str) -> StorageResult<Transaction>;

    /// Write an identity record
    fn write_identity(&self, identity: &IdentityRecord) -> StorageResult<()>;

    /// Read an identity record by DID
    fn read_identity(&self, did: &str) -> StorageResult<IdentityRecord>;

    /// Write a credential
    fn write_credential(&self, credential: &Credential) -> StorageResult<()>;

    /// Read a credential by ID
    fn read_credential(&self, cred_id: &str) -> StorageResult<Credential>;

    /// Batch write operations (atomic)
    fn write_batch(&self, blocks: &[Block], txs: &[Transaction]) -> StorageResult<()>;

    /// Get latest block height
    fn get_latest_height(&self) -> StorageResult<u64>;

    /// Check if block exists
    fn block_exists(&self, height: u64) -> StorageResult<bool>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_serialization() {
        let block = Block {
            height: 1,
            timestamp: 1000,
            parent_hash: [0u8; 32],
            merkle_root: [1u8; 32],
            transactions: vec!["tx1".to_string()],
            proposer: "proposer1".to_string(),
            signature: [2u8; 64],
        };

        let serialized = serde_json::to_string(&block).unwrap();
        let deserialized: Block = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.height, 1);
        assert_eq!(deserialized.proposer, "proposer1");
    }

    #[test]
    fn test_transaction_serialization() {
        let tx = Transaction {
            id: "tx123".to_string(),
            block_height: 1,
            timestamp: 1000,
            input_did: "did:bc:input".to_string(),
            output_recipient: "did:bc:output".to_string(),
            amount: 100,
            state: "confirmed".to_string(),
        };

        let serialized = serde_json::to_string(&tx).unwrap();
        let deserialized: Transaction = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.id, "tx123");
        assert_eq!(deserialized.amount, 100);
    }

    #[test]
    fn test_identity_record_serialization() {
        let identity = IdentityRecord {
            did: "did:bc:abc123".to_string(),
            created_at: 1000,
            updated_at: 2000,
            status: "active".to_string(),
        };

        let serialized = serde_json::to_string(&identity).unwrap();
        let deserialized: IdentityRecord = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.did, "did:bc:abc123");
        assert_eq!(deserialized.status, "active");
    }

    #[test]
    fn test_credential_serialization() {
        let cred = Credential {
            id: "cred-uuid-1".to_string(),
            issuer_did: "did:bc:issuer".to_string(),
            subject_did: "did:bc:subject".to_string(),
            cred_type: "eid".to_string(),
            issued_at: 1000,
            expires_at: 2000,
            revoked_at: None,
        };

        let serialized = serde_json::to_string(&cred).unwrap();
        let deserialized: Credential = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.id, "cred-uuid-1");
        assert_eq!(deserialized.cred_type, "eid");
        assert!(deserialized.revoked_at.is_none());
    }
}
