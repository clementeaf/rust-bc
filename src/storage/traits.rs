//! Storage trait definitions
//!
//! Defines the BlockStore trait and related interfaces for storage operations.

use super::errors::StorageResult;

/// Block structure for storage
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
pub struct IdentityRecord {
    pub did: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub status: String,
}

/// Credential structure
#[derive(Debug, Clone)]
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

    // Serialization tests deferred: need custom CBOR encoding for byte arrays
    // Will implement in integration with RocksDB layer
}
