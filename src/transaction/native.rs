//! Native cryptocurrency transactions — protocol-level transfers without chaincode.
//!
//! `NativeTransaction` is the first-class transaction type for the NOTA token.
//! It carries sender, recipient, amount, nonce (replay protection), fee, and
//! a cryptographic signature over the canonical payload.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::account::{AccountError, AccountStore};
use crate::tokenomics::economics::{split_fees, BURN_ADDRESS, MIN_TX_FEE};

// ── Transaction Types ──────────────────────────────────────────────────────

/// Discriminant for transaction types at the protocol level.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TransactionKind {
    /// Simple value transfer between accounts.
    Transfer {
        from: String,
        to: String,
        amount: u64,
    },
    /// Coinbase reward (minted by the protocol, no sender).
    Coinbase { to: String, amount: u64 },
}

/// A native protocol transaction with nonce-based replay protection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeTransaction {
    /// Unique transaction ID (hex hash of the canonical payload).
    pub id: String,
    /// The transaction payload.
    pub kind: TransactionKind,
    /// Sender's nonce (must match account nonce for transfers; 0 for coinbase).
    pub nonce: u64,
    /// Fee offered by the sender (in smallest NOTA unit).
    pub fee: u64,
    /// Unix timestamp (seconds).
    pub timestamp: u64,
    /// Signature over `signing_payload()` bytes.
    #[serde(default)]
    pub signature: Vec<u8>,
    /// Algorithm used for the signature.
    #[serde(default)]
    pub signature_algorithm: String,
}

impl NativeTransaction {
    /// Create a new transfer transaction.
    pub fn new_transfer(
        from: impl Into<String>,
        to: impl Into<String>,
        amount: u64,
        nonce: u64,
        fee: u64,
    ) -> Self {
        let from = from.into();
        let to = to.into();
        let kind = TransactionKind::Transfer {
            from: from.clone(),
            to: to.clone(),
            amount,
        };
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let id = Self::compute_id(&kind, nonce, fee, timestamp);

        Self {
            id,
            kind,
            nonce,
            fee,
            timestamp,
            signature: Vec::new(),
            signature_algorithm: String::new(),
        }
    }

    /// Create a coinbase (block reward) transaction.
    pub fn new_coinbase(to: impl Into<String>, amount: u64) -> Self {
        let to = to.into();
        let kind = TransactionKind::Coinbase {
            to: to.clone(),
            amount,
        };
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let id = Self::compute_id(&kind, 0, 0, timestamp);

        Self {
            id,
            kind,
            nonce: 0,
            fee: 0,
            timestamp,
            signature: Vec::new(),
            signature_algorithm: String::new(),
        }
    }

    /// Canonical bytes for signing (kind + nonce + fee + timestamp).
    pub fn signing_payload(&self) -> Vec<u8> {
        let canonical = serde_json::json!({
            "kind": self.kind,
            "nonce": self.nonce,
            "fee": self.fee,
            "timestamp": self.timestamp,
        });
        canonical.to_string().into_bytes()
    }

    /// Sender address (None for coinbase).
    pub fn sender(&self) -> Option<&str> {
        match &self.kind {
            TransactionKind::Transfer { from, .. } => Some(from),
            TransactionKind::Coinbase { .. } => None,
        }
    }

    /// Recipient address.
    pub fn recipient(&self) -> &str {
        match &self.kind {
            TransactionKind::Transfer { to, .. } | TransactionKind::Coinbase { to, .. } => to,
        }
    }

    /// Transfer amount.
    pub fn amount(&self) -> u64 {
        match &self.kind {
            TransactionKind::Transfer { amount, .. } | TransactionKind::Coinbase { amount, .. } => {
                *amount
            }
        }
    }

    fn compute_id(kind: &TransactionKind, nonce: u64, fee: u64, timestamp: u64) -> String {
        use pqc_crypto_module::legacy::legacy_sha256;
        let payload = serde_json::json!({
            "kind": kind,
            "nonce": nonce,
            "fee": fee,
            "timestamp": timestamp,
        });
        let bytes = payload.to_string().into_bytes();
        match legacy_sha256(&bytes) {
            Ok(hash) => hex::encode(hash),
            Err(_) => hex::encode(&bytes[..32.min(bytes.len())]),
        }
    }
}

// ── Execution Errors ───────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum NativeTxError {
    #[error("fee too low: minimum is {min}, got {got}")]
    FeeTooLow { min: u64, got: u64 },
    #[error("self-transfer not allowed")]
    SelfTransfer,
    #[error("zero amount transfer")]
    ZeroAmount,
    #[error("account error: {0}")]
    Account(#[from] AccountError),
}

// ── Execution ──────────────────────────────────────────────────────────────

/// Execute a native transfer against the account store.
///
/// 1. Validate fee >= MIN_TX_FEE
/// 2. Debit sender: amount + fee (atomic with nonce check)
/// 3. Credit recipient: amount
/// 4. Split fee: 80% burn, 20% to proposer
///
/// Returns (burned, proposer_share) fee split.
pub fn execute_transfer(
    store: &dyn AccountStore,
    tx: &NativeTransaction,
    proposer_address: &str,
) -> Result<(u64, u64), NativeTxError> {
    let (from, to, amount) = match &tx.kind {
        TransactionKind::Transfer { from, to, amount } => (from.as_str(), to.as_str(), *amount),
        TransactionKind::Coinbase { to, amount } => {
            // Coinbase: just credit recipient, no fee
            store.credit(to, *amount)?;
            return Ok((0, 0));
        }
    };

    if from == to {
        return Err(NativeTxError::SelfTransfer);
    }
    if amount == 0 {
        return Err(NativeTxError::ZeroAmount);
    }
    if tx.fee < MIN_TX_FEE {
        return Err(NativeTxError::FeeTooLow {
            min: MIN_TX_FEE,
            got: tx.fee,
        });
    }

    // Debit sender: amount + fee, with nonce check
    let total_debit = amount.checked_add(tx.fee).ok_or(AccountError::Overflow)?;

    // Manually do transfer + fee debit in one logical operation
    let mut sender = store.get_account(from)?;
    if sender.nonce != tx.nonce {
        return Err(AccountError::NonceMismatch {
            expected: sender.nonce,
            got: tx.nonce,
        }
        .into());
    }
    if sender.balance < total_debit {
        return Err(AccountError::InsufficientBalance {
            have: sender.balance,
            need: total_debit,
        }
        .into());
    }

    sender.balance -= total_debit;
    sender.nonce += 1;
    store.set_account(from, &sender)?;

    // Credit recipient
    store.credit(to, amount)?;

    // Fee distribution
    let fee_split = split_fees(tx.fee);
    store.credit(BURN_ADDRESS, fee_split.burn)?;
    if fee_split.proposer > 0 {
        store.credit(proposer_address, fee_split.proposer)?;
    }

    Ok((fee_split.burn, fee_split.proposer))
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::MemoryAccountStore;

    #[test]
    fn new_transfer_has_id_and_fields() {
        let tx = NativeTransaction::new_transfer("alice", "bob", 100, 0, 5);
        assert!(!tx.id.is_empty());
        assert_eq!(tx.amount(), 100);
        assert_eq!(tx.sender(), Some("alice"));
        assert_eq!(tx.recipient(), "bob");
        assert_eq!(tx.nonce, 0);
        assert_eq!(tx.fee, 5);
    }

    #[test]
    fn new_coinbase_has_no_sender() {
        let tx = NativeTransaction::new_coinbase("miner", 50);
        assert!(tx.sender().is_none());
        assert_eq!(tx.recipient(), "miner");
        assert_eq!(tx.amount(), 50);
        assert_eq!(tx.fee, 0);
    }

    #[test]
    fn signing_payload_is_deterministic() {
        let tx = NativeTransaction {
            id: "test".into(),
            kind: TransactionKind::Transfer {
                from: "a".into(),
                to: "b".into(),
                amount: 10,
            },
            nonce: 1,
            fee: 2,
            timestamp: 1000,
            signature: vec![],
            signature_algorithm: String::new(),
        };
        let p1 = tx.signing_payload();
        let p2 = tx.signing_payload();
        assert_eq!(p1, p2);
    }

    #[test]
    fn execute_transfer_success() {
        let store = MemoryAccountStore::with_genesis(&[("alice", 1000)]);
        let tx = NativeTransaction::new_transfer("alice", "bob", 100, 0, 10);
        let (burned, proposer) = execute_transfer(&store, &tx, "validator").unwrap();

        let alice = store.get_account("alice").unwrap();
        assert_eq!(alice.balance, 890); // 1000 - 100 - 10
        assert_eq!(alice.nonce, 1);

        let bob = store.get_account("bob").unwrap();
        assert_eq!(bob.balance, 100);

        // Fee split: 80% burn (8), 20% proposer (2)
        assert_eq!(burned, 8);
        assert_eq!(proposer, 2);

        let validator = store.get_account("validator").unwrap();
        assert_eq!(validator.balance, 2);
    }

    #[test]
    fn execute_transfer_insufficient_balance() {
        let store = MemoryAccountStore::with_genesis(&[("alice", 50)]);
        let tx = NativeTransaction::new_transfer("alice", "bob", 100, 0, 5);
        let err = execute_transfer(&store, &tx, "v").unwrap_err();
        assert!(matches!(
            err,
            NativeTxError::Account(AccountError::InsufficientBalance { .. })
        ));
    }

    #[test]
    fn execute_transfer_fee_too_low() {
        let store = MemoryAccountStore::with_genesis(&[("alice", 1000)]);
        let tx = NativeTransaction::new_transfer("alice", "bob", 100, 0, 0);
        let err = execute_transfer(&store, &tx, "v").unwrap_err();
        assert!(matches!(err, NativeTxError::FeeTooLow { .. }));
    }

    #[test]
    fn execute_transfer_self_transfer() {
        let store = MemoryAccountStore::with_genesis(&[("alice", 1000)]);
        let tx = NativeTransaction::new_transfer("alice", "alice", 100, 0, 5);
        let err = execute_transfer(&store, &tx, "v").unwrap_err();
        assert!(matches!(err, NativeTxError::SelfTransfer));
    }

    #[test]
    fn execute_transfer_zero_amount() {
        let store = MemoryAccountStore::with_genesis(&[("alice", 1000)]);
        let tx = NativeTransaction::new_transfer("alice", "bob", 0, 0, 5);
        let err = execute_transfer(&store, &tx, "v").unwrap_err();
        assert!(matches!(err, NativeTxError::ZeroAmount));
    }

    #[test]
    fn execute_transfer_nonce_mismatch() {
        let store = MemoryAccountStore::with_genesis(&[("alice", 1000)]);
        let tx = NativeTransaction::new_transfer("alice", "bob", 100, 99, 5);
        let err = execute_transfer(&store, &tx, "v").unwrap_err();
        assert!(matches!(
            err,
            NativeTxError::Account(AccountError::NonceMismatch { .. })
        ));
    }

    #[test]
    fn execute_coinbase_credits_recipient() {
        let store = MemoryAccountStore::new();
        let tx = NativeTransaction::new_coinbase("miner", 50);
        let (burned, proposer) = execute_transfer(&store, &tx, "validator").unwrap();

        assert_eq!(burned, 0);
        assert_eq!(proposer, 0);

        let miner = store.get_account("miner").unwrap();
        assert_eq!(miner.balance, 50);
    }

    #[test]
    fn sequential_transfers_increment_nonce() {
        let store = MemoryAccountStore::with_genesis(&[("alice", 1000)]);

        for i in 0..5 {
            let tx = NativeTransaction::new_transfer("alice", "bob", 10, i, 1);
            execute_transfer(&store, &tx, "v").unwrap();
        }

        let alice = store.get_account("alice").unwrap();
        assert_eq!(alice.nonce, 5);
        assert_eq!(alice.balance, 1000 - 5 * 11); // 5 * (10 + 1 fee)
    }

    #[test]
    fn serde_roundtrip() {
        let tx = NativeTransaction::new_transfer("alice", "bob", 42, 7, 3);
        let json = serde_json::to_string(&tx).unwrap();
        let deserialized: NativeTransaction = serde_json::from_str(&json).unwrap();
        assert_eq!(tx, deserialized);
    }
}
