//! Token escrow for cross-chain transfers.
//!
//! Manages the lock/mint/burn/release lifecycle:
//!
//! **Outbound** (rust-bc → external):
//! 1. `lock()` — freeze tokens in escrow on rust-bc
//! 2. External chain mints wrapped tokens
//! 3. On return: external chain burns wrapped tokens
//! 4. `release()` — unfreeze tokens from escrow on rust-bc
//!
//! **Inbound** (external → rust-bc):
//! 1. External chain locks tokens
//! 2. `mint()` — create wrapped tokens on rust-bc
//! 3. On return: `burn()` — destroy wrapped tokens on rust-bc
//! 4. External chain releases tokens

use std::collections::HashMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use super::types::{ChainId, MessageId, TransferStatus};

/// An escrow vault entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EscrowEntry {
    pub message_id: MessageId,
    pub sender: String,
    pub amount: u64,
    pub denom: String,
    pub dest_chain: ChainId,
    pub status: TransferStatus,
    pub locked_at: u64,
    pub released_at: Option<u64>,
}

/// Wrapped token balance for inbound transfers.
#[derive(Debug, Clone, Default)]
struct WrappedBalance {
    /// account → amount
    balances: HashMap<String, u64>,
    /// Total minted across all accounts.
    total_supply: u64,
}

/// Escrow engine managing locked native tokens and minted wrapped tokens.
pub struct EscrowVault {
    /// Native token escrow: message_id → EscrowEntry.
    locked: Mutex<HashMap<MessageId, EscrowEntry>>,
    /// Wrapped token balances per (chain_id, denom).
    wrapped: Mutex<HashMap<(String, String), WrappedBalance>>,
    /// Total native tokens locked in escrow.
    total_locked: Mutex<u64>,
}

/// Errors from escrow operations.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EscrowError {
    #[error("insufficient balance: need {required}, have {available}")]
    InsufficientBalance { required: u64, available: u64 },
    #[error("escrow entry not found: {0:?}")]
    NotFound(MessageId),
    #[error("escrow entry already exists: {0:?}")]
    AlreadyExists(MessageId),
    #[error("invalid state transition: {0}")]
    InvalidState(String),
    #[error("amount must be > 0")]
    ZeroAmount,
}

impl EscrowVault {
    pub fn new() -> Self {
        Self {
            locked: Mutex::new(HashMap::new()),
            wrapped: Mutex::new(HashMap::new()),
            total_locked: Mutex::new(0),
        }
    }

    // ── Outbound: lock/release ──────────────────────────────────────────

    /// Lock native tokens in escrow for an outbound transfer.
    ///
    /// Caller must verify the sender has sufficient balance before calling.
    pub fn lock(
        &self,
        message_id: MessageId,
        sender: &str,
        amount: u64,
        denom: &str,
        dest_chain: &ChainId,
        block_height: u64,
    ) -> Result<(), EscrowError> {
        if amount == 0 {
            return Err(EscrowError::ZeroAmount);
        }

        let mut locked = self.locked.lock().unwrap();
        if locked.contains_key(&message_id) {
            return Err(EscrowError::AlreadyExists(message_id));
        }

        let entry = EscrowEntry {
            message_id,
            sender: sender.to_string(),
            amount,
            denom: denom.to_string(),
            dest_chain: dest_chain.clone(),
            status: TransferStatus::Pending,
            locked_at: block_height,
            released_at: None,
        };

        locked.insert(message_id, entry);
        *self.total_locked.lock().unwrap() += amount;

        Ok(())
    }

    /// Release locked tokens back to the sender (return from external chain).
    pub fn release(
        &self,
        message_id: &MessageId,
        block_height: u64,
    ) -> Result<EscrowEntry, EscrowError> {
        let mut locked = self.locked.lock().unwrap();
        let entry = locked
            .get_mut(message_id)
            .ok_or(EscrowError::NotFound(*message_id))?;

        if entry.status != TransferStatus::Pending {
            return Err(EscrowError::InvalidState(format!(
                "expected Pending, got {:?}",
                entry.status
            )));
        }

        entry.status = TransferStatus::Completed;
        entry.released_at = Some(block_height);
        *self.total_locked.lock().unwrap() -= entry.amount;

        Ok(entry.clone())
    }

    /// Refund locked tokens (transfer failed or expired).
    pub fn refund(
        &self,
        message_id: &MessageId,
        block_height: u64,
    ) -> Result<EscrowEntry, EscrowError> {
        let mut locked = self.locked.lock().unwrap();
        let entry = locked
            .get_mut(message_id)
            .ok_or(EscrowError::NotFound(*message_id))?;

        if entry.status != TransferStatus::Pending {
            return Err(EscrowError::InvalidState(format!(
                "expected Pending, got {:?}",
                entry.status
            )));
        }

        entry.status = TransferStatus::Refunded;
        entry.released_at = Some(block_height);
        *self.total_locked.lock().unwrap() -= entry.amount;

        Ok(entry.clone())
    }

    // ── Inbound: mint/burn ──────────────────────────────────────────────

    /// Mint wrapped tokens on rust-bc for an inbound transfer.
    pub fn mint(
        &self,
        recipient: &str,
        amount: u64,
        source_chain: &ChainId,
        denom: &str,
    ) -> Result<u64, EscrowError> {
        if amount == 0 {
            return Err(EscrowError::ZeroAmount);
        }

        let key = (source_chain.0.clone(), denom.to_string());
        let mut wrapped = self.wrapped.lock().unwrap();
        let balance = wrapped.entry(key).or_default();

        *balance.balances.entry(recipient.to_string()).or_insert(0) += amount;
        balance.total_supply += amount;

        Ok(balance.balances[recipient])
    }

    /// Burn wrapped tokens on rust-bc (returning to external chain).
    pub fn burn(
        &self,
        account: &str,
        amount: u64,
        source_chain: &ChainId,
        denom: &str,
    ) -> Result<u64, EscrowError> {
        if amount == 0 {
            return Err(EscrowError::ZeroAmount);
        }

        let key = (source_chain.0.clone(), denom.to_string());
        let mut wrapped = self.wrapped.lock().unwrap();
        let balance = wrapped
            .get_mut(&key)
            .ok_or(EscrowError::InsufficientBalance {
                required: amount,
                available: 0,
            })?;

        let acct = balance
            .balances
            .get_mut(account)
            .ok_or(EscrowError::InsufficientBalance {
                required: amount,
                available: 0,
            })?;

        if *acct < amount {
            return Err(EscrowError::InsufficientBalance {
                required: amount,
                available: *acct,
            });
        }

        *acct -= amount;
        balance.total_supply -= amount;

        Ok(*acct)
    }

    // ── Queries ─────────────────────────────────────────────────────────

    /// Total native tokens locked in escrow.
    pub fn total_locked(&self) -> u64 {
        *self.total_locked.lock().unwrap()
    }

    /// Get an escrow entry by message ID.
    pub fn get_escrow(&self, message_id: &MessageId) -> Option<EscrowEntry> {
        self.locked.lock().unwrap().get(message_id).cloned()
    }

    /// Get wrapped token balance for an account.
    pub fn wrapped_balance(&self, account: &str, source_chain: &ChainId, denom: &str) -> u64 {
        let key = (source_chain.0.clone(), denom.to_string());
        self.wrapped
            .lock()
            .unwrap()
            .get(&key)
            .and_then(|b| b.balances.get(account))
            .copied()
            .unwrap_or(0)
    }

    /// Total supply of a wrapped token.
    pub fn wrapped_total_supply(&self, source_chain: &ChainId, denom: &str) -> u64 {
        let key = (source_chain.0.clone(), denom.to_string());
        self.wrapped
            .lock()
            .unwrap()
            .get(&key)
            .map(|b| b.total_supply)
            .unwrap_or(0)
    }
}

impl Default for EscrowVault {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg_id(id: u8) -> MessageId {
        let mut h = [0u8; 32];
        h[0] = id;
        h
    }

    fn eth() -> ChainId {
        ChainId("ethereum".into())
    }

    // --- lock ---

    #[test]
    fn lock_tokens() {
        let vault = EscrowVault::new();
        vault
            .lock(msg_id(1), "alice", 1000, "NOTA", &eth(), 10)
            .unwrap();

        assert_eq!(vault.total_locked(), 1000);
        let entry = vault.get_escrow(&msg_id(1)).unwrap();
        assert_eq!(entry.sender, "alice");
        assert_eq!(entry.amount, 1000);
        assert_eq!(entry.status, TransferStatus::Pending);
    }

    #[test]
    fn lock_zero_amount_fails() {
        let vault = EscrowVault::new();
        let err = vault
            .lock(msg_id(1), "alice", 0, "NOTA", &eth(), 10)
            .unwrap_err();
        assert!(matches!(err, EscrowError::ZeroAmount));
    }

    #[test]
    fn lock_duplicate_message_id_fails() {
        let vault = EscrowVault::new();
        vault
            .lock(msg_id(1), "alice", 100, "NOTA", &eth(), 10)
            .unwrap();
        let err = vault
            .lock(msg_id(1), "bob", 200, "NOTA", &eth(), 11)
            .unwrap_err();
        assert!(matches!(err, EscrowError::AlreadyExists(_)));
    }

    // --- release ---

    #[test]
    fn release_returns_entry() {
        let vault = EscrowVault::new();
        vault
            .lock(msg_id(1), "alice", 500, "NOTA", &eth(), 10)
            .unwrap();

        let entry = vault.release(&msg_id(1), 20).unwrap();
        assert_eq!(entry.status, TransferStatus::Completed);
        assert_eq!(entry.released_at, Some(20));
        assert_eq!(vault.total_locked(), 0);
    }

    #[test]
    fn release_nonexistent_fails() {
        let vault = EscrowVault::new();
        let err = vault.release(&msg_id(99), 20).unwrap_err();
        assert!(matches!(err, EscrowError::NotFound(_)));
    }

    #[test]
    fn release_already_completed_fails() {
        let vault = EscrowVault::new();
        vault
            .lock(msg_id(1), "alice", 500, "NOTA", &eth(), 10)
            .unwrap();
        vault.release(&msg_id(1), 20).unwrap();

        let err = vault.release(&msg_id(1), 30).unwrap_err();
        assert!(matches!(err, EscrowError::InvalidState(_)));
    }

    // --- refund ---

    #[test]
    fn refund_returns_tokens() {
        let vault = EscrowVault::new();
        vault
            .lock(msg_id(1), "alice", 300, "NOTA", &eth(), 10)
            .unwrap();

        let entry = vault.refund(&msg_id(1), 25).unwrap();
        assert_eq!(entry.status, TransferStatus::Refunded);
        assert_eq!(vault.total_locked(), 0);
    }

    // --- mint ---

    #[test]
    fn mint_wrapped_tokens() {
        let vault = EscrowVault::new();
        let balance = vault.mint("bob", 1000, &eth(), "wETH").unwrap();
        assert_eq!(balance, 1000);
        assert_eq!(vault.wrapped_balance("bob", &eth(), "wETH"), 1000);
        assert_eq!(vault.wrapped_total_supply(&eth(), "wETH"), 1000);
    }

    #[test]
    fn mint_accumulates() {
        let vault = EscrowVault::new();
        vault.mint("bob", 500, &eth(), "wETH").unwrap();
        vault.mint("bob", 300, &eth(), "wETH").unwrap();
        assert_eq!(vault.wrapped_balance("bob", &eth(), "wETH"), 800);
        assert_eq!(vault.wrapped_total_supply(&eth(), "wETH"), 800);
    }

    #[test]
    fn mint_zero_fails() {
        let vault = EscrowVault::new();
        assert!(matches!(
            vault.mint("bob", 0, &eth(), "wETH"),
            Err(EscrowError::ZeroAmount)
        ));
    }

    // --- burn ---

    #[test]
    fn burn_wrapped_tokens() {
        let vault = EscrowVault::new();
        vault.mint("bob", 1000, &eth(), "wETH").unwrap();

        let remaining = vault.burn("bob", 400, &eth(), "wETH").unwrap();
        assert_eq!(remaining, 600);
        assert_eq!(vault.wrapped_total_supply(&eth(), "wETH"), 600);
    }

    #[test]
    fn burn_insufficient_balance_fails() {
        let vault = EscrowVault::new();
        vault.mint("bob", 100, &eth(), "wETH").unwrap();

        let err = vault.burn("bob", 200, &eth(), "wETH").unwrap_err();
        assert!(matches!(err, EscrowError::InsufficientBalance { .. }));
    }

    #[test]
    fn burn_unknown_account_fails() {
        let vault = EscrowVault::new();
        vault.mint("bob", 100, &eth(), "wETH").unwrap();

        let err = vault.burn("alice", 50, &eth(), "wETH").unwrap_err();
        assert!(matches!(err, EscrowError::InsufficientBalance { .. }));
    }

    // --- full lifecycle ---

    #[test]
    fn outbound_lock_and_release_lifecycle() {
        let vault = EscrowVault::new();

        // Lock on rust-bc (outbound to Ethereum).
        vault
            .lock(msg_id(1), "alice", 1000, "NOTA", &eth(), 100)
            .unwrap();
        assert_eq!(vault.total_locked(), 1000);

        // External chain mints wNOTA... (off-chain)
        // User sends wNOTA back... (off-chain)
        // External chain burns wNOTA and emits proof... (off-chain)

        // Release on rust-bc (proof verified).
        let entry = vault.release(&msg_id(1), 200).unwrap();
        assert_eq!(entry.amount, 1000);
        assert_eq!(vault.total_locked(), 0);
    }

    #[test]
    fn inbound_mint_and_burn_lifecycle() {
        let vault = EscrowVault::new();

        // External chain locked ETH... (off-chain, proof submitted)

        // Mint wETH on rust-bc.
        vault.mint("bob", 2000, &eth(), "wETH").unwrap();
        assert_eq!(vault.wrapped_balance("bob", &eth(), "wETH"), 2000);

        // Bob uses wETH on rust-bc... (normal txs)

        // Bob wants to return to Ethereum — burn wETH.
        vault.burn("bob", 2000, &eth(), "wETH").unwrap();
        assert_eq!(vault.wrapped_balance("bob", &eth(), "wETH"), 0);
        assert_eq!(vault.wrapped_total_supply(&eth(), "wETH"), 0);

        // External chain releases ETH... (off-chain)
    }

    // --- multiple chains ---

    #[test]
    fn wrapped_tokens_per_chain_are_independent() {
        let vault = EscrowVault::new();
        let cosmos = ChainId("cosmos".into());

        vault.mint("bob", 1000, &eth(), "wETH").unwrap();
        vault.mint("bob", 500, &cosmos, "wATOM").unwrap();

        assert_eq!(vault.wrapped_balance("bob", &eth(), "wETH"), 1000);
        assert_eq!(vault.wrapped_balance("bob", &cosmos, "wATOM"), 500);
        assert_eq!(vault.wrapped_total_supply(&eth(), "wETH"), 1000);
        assert_eq!(vault.wrapped_total_supply(&cosmos, "wATOM"), 500);
    }
}
