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
    /// Chain ID for domain separation (prevents cross-network replay).
    /// 0 = legacy/unset, 9999 = testnet, 9998 = devnet, 1 = mainnet.
    #[serde(default)]
    pub chain_id: u64,
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
    /// Create a new transfer transaction (chain_id defaults to 0 = unset).
    pub fn new_transfer(
        from: impl Into<String>,
        to: impl Into<String>,
        amount: u64,
        nonce: u64,
        fee: u64,
    ) -> Self {
        Self::new_transfer_with_chain(from, to, amount, nonce, fee, 0)
    }

    /// Create a new transfer with explicit chain ID.
    pub fn new_transfer_with_chain(
        from: impl Into<String>,
        to: impl Into<String>,
        amount: u64,
        nonce: u64,
        fee: u64,
        chain_id: u64,
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

        let id = Self::compute_id(&kind, nonce, fee, timestamp, chain_id);

        Self {
            id,
            chain_id,
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

        let id = Self::compute_id(&kind, 0, 0, timestamp, 0);

        Self {
            id,
            chain_id: 0,
            kind,
            nonce: 0,
            fee: 0,
            timestamp,
            signature: Vec::new(),
            signature_algorithm: String::new(),
        }
    }

    /// Canonical bytes for signing (chain_id + kind + nonce + fee + timestamp).
    ///
    /// `chain_id` is always included for domain separation, even if 0.
    pub fn signing_payload(&self) -> Vec<u8> {
        let canonical = serde_json::json!({
            "chain_id": self.chain_id,
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

    fn compute_id(
        kind: &TransactionKind,
        nonce: u64,
        fee: u64,
        timestamp: u64,
        chain_id: u64,
    ) -> String {
        use pqc_crypto_module::legacy::legacy_sha256;
        let payload = serde_json::json!({
            "chain_id": chain_id,
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
    #[error("chain_id mismatch: expected {expected}, got {got}")]
    ChainIdMismatch { expected: u64, got: u64 },
    #[error("missing or empty signature")]
    MissingSignature,
    #[error("invalid signature")]
    InvalidSignature,
    #[error("account error: {0}")]
    Account(#[from] AccountError),
}

// ── Signature Verification ─────────────────────────────────────────────────

/// Verify the signature on a native transaction using the sender's public key.
///
/// Supports Ed25519 (64-byte sig, 32-byte pk) and ML-DSA-65 (3309-byte sig, 1952-byte pk).
pub fn verify_tx_signature(tx: &NativeTransaction, pubkey: &[u8]) -> Result<bool, NativeTxError> {
    if tx.signature.is_empty() {
        return Err(NativeTxError::MissingSignature);
    }

    let payload = tx.signing_payload();

    // Detect algorithm by signature size
    match tx.signature.len() {
        64 => {
            // Ed25519
            use pqc_crypto_module::legacy::ed25519::{Signature, Verifier, VerifyingKey};
            let vk = VerifyingKey::from_bytes(
                pubkey
                    .try_into()
                    .map_err(|_| NativeTxError::InvalidSignature)?,
            )
            .map_err(|_| NativeTxError::InvalidSignature)?;
            let sig_bytes: [u8; 64] = tx
                .signature
                .as_slice()
                .try_into()
                .map_err(|_| NativeTxError::InvalidSignature)?;
            let sig = Signature::from_bytes(&sig_bytes);
            Ok(vk.verify(&payload, &sig).is_ok())
        }
        3309 => {
            // ML-DSA-65
            use pqc_crypto_module::legacy::mldsa_raw::mldsa65;
            use pqcrypto_traits::sign::{DetachedSignature as _, PublicKey as _};
            let pk = mldsa65::PublicKey::from_bytes(pubkey)
                .map_err(|_| NativeTxError::InvalidSignature)?;
            let sig = mldsa65::DetachedSignature::from_bytes(&tx.signature)
                .map_err(|_| NativeTxError::InvalidSignature)?;
            Ok(mldsa65::verify_detached_signature(&sig, &payload, &pk).is_ok())
        }
        _ => Err(NativeTxError::InvalidSignature),
    }
}

// ── Execution ──────────────────────────────────────────────────────────────

/// Execute a native transfer with chain ID validation.
///
/// If `expected_chain_id` is non-zero and tx.chain_id is non-zero,
/// they must match. Chain ID 0 is accepted as "any" for backwards compat.
pub fn execute_transfer_checked(
    store: &dyn AccountStore,
    tx: &NativeTransaction,
    proposer_address: &str,
    expected_chain_id: u64,
) -> Result<(u64, u64), NativeTxError> {
    if expected_chain_id != 0 && tx.chain_id != 0 && tx.chain_id != expected_chain_id {
        return Err(NativeTxError::ChainIdMismatch {
            expected: expected_chain_id,
            got: tx.chain_id,
        });
    }
    execute_transfer(store, tx, proposer_address)
}

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
            chain_id: 0,
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

    #[test]
    fn verify_ed25519_signature() {
        use crate::identity::signing::{SigningProvider, SoftwareSigningProvider};
        let provider = SoftwareSigningProvider::generate();
        let pk = provider.public_key();

        let mut tx = NativeTransaction::new_transfer("alice", "bob", 100, 0, 5);
        let payload = tx.signing_payload();
        tx.signature = provider.sign(&payload).unwrap();
        tx.signature_algorithm = "ed25519".to_string();

        assert!(verify_tx_signature(&tx, &pk).unwrap());
    }

    #[test]
    fn verify_rejects_empty_signature() {
        let tx = NativeTransaction::new_transfer("alice", "bob", 100, 0, 5);
        let err = verify_tx_signature(&tx, &[0u8; 32]).unwrap_err();
        assert!(matches!(err, NativeTxError::MissingSignature));
    }

    #[test]
    fn verify_rejects_wrong_signature() {
        let mut tx = NativeTransaction::new_transfer("alice", "bob", 100, 0, 5);
        tx.signature = vec![0u8; 64]; // wrong sig bytes

        use crate::identity::signing::{SigningProvider, SoftwareSigningProvider};
        let provider = SoftwareSigningProvider::generate();
        let pk = provider.public_key();

        assert!(!verify_tx_signature(&tx, &pk).unwrap());
    }

    #[test]
    fn verify_rejects_bad_sig_length() {
        let mut tx = NativeTransaction::new_transfer("alice", "bob", 100, 0, 5);
        tx.signature = vec![0u8; 50]; // not 64 or 3309
        let err = verify_tx_signature(&tx, &[0u8; 32]).unwrap_err();
        assert!(matches!(err, NativeTxError::InvalidSignature));
    }

    // ── Chain ID tests ─────────────────────────────────────────────────────

    #[test]
    fn chain_id_in_signing_payload() {
        let tx_a = NativeTransaction::new_transfer_with_chain("a", "b", 10, 0, 1, 9999);
        let tx_b = NativeTransaction::new_transfer_with_chain("a", "b", 10, 0, 1, 9998);
        // Different chain_id → different payload → different ID
        assert_ne!(tx_a.signing_payload(), tx_b.signing_payload());
        assert_ne!(tx_a.id, tx_b.id);
    }

    #[test]
    fn chain_id_mismatch_rejected() {
        let store = MemoryAccountStore::with_genesis(&[("alice", 1000)]);
        let tx = NativeTransaction::new_transfer_with_chain("alice", "bob", 100, 0, 5, 9999);
        let err = execute_transfer_checked(&store, &tx, "v", 1).unwrap_err();
        assert!(matches!(
            err,
            NativeTxError::ChainIdMismatch {
                expected: 1,
                got: 9999
            }
        ));
    }

    #[test]
    fn chain_id_zero_accepted_as_any() {
        let store = MemoryAccountStore::with_genesis(&[("alice", 1000)]);
        // tx with chain_id=0 accepted by any network
        let tx = NativeTransaction::new_transfer("alice", "bob", 100, 0, 5);
        assert_eq!(tx.chain_id, 0);
        execute_transfer_checked(&store, &tx, "v", 9999).unwrap();
    }

    #[test]
    fn chain_id_match_accepted() {
        let store = MemoryAccountStore::with_genesis(&[("alice", 1000)]);
        let tx = NativeTransaction::new_transfer_with_chain("alice", "bob", 100, 0, 5, 9999);
        execute_transfer_checked(&store, &tx, "v", 9999).unwrap();
    }

    #[test]
    fn cross_network_replay_prevented_by_chain_id() {
        let store = MemoryAccountStore::with_genesis(&[("alice", 1000)]);
        let tx = NativeTransaction::new_transfer_with_chain("alice", "bob", 100, 0, 5, 9999);
        // Works on testnet (9999)
        assert!(execute_transfer_checked(&store, &tx, "v", 9999).is_ok());

        // Same tx rejected on mainnet (1)
        let store2 = MemoryAccountStore::with_genesis(&[("alice", 1000)]);
        let err = execute_transfer_checked(&store2, &tx, "v", 1).unwrap_err();
        assert!(matches!(err, NativeTxError::ChainIdMismatch { .. }));
    }
}
