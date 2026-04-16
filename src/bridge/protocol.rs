//! Bridge protocol trait — abstracts over different bridge implementations
//! (light client, relay, HTLC) with a common chain registry.

use std::collections::HashMap;
use std::sync::Mutex;

use super::escrow::{EscrowError, EscrowVault};
use super::types::*;
use super::verifier;

/// Errors from the bridge protocol.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BridgeError {
    #[error("unknown chain: {0}")]
    UnknownChain(String),
    #[error("chain not active: {0}")]
    ChainNotActive(String),
    #[error("proof verification failed: {0}")]
    ProofFailed(String),
    #[error("escrow error: {0}")]
    Escrow(String),
    #[error("transfer exceeds maximum: {amount} > {max}")]
    ExceedsMax { amount: u64, max: u64 },
    #[error("insufficient confirmations: have {have}, need {need}")]
    InsufficientConfirmations { have: u64, need: u64 },
}

impl From<EscrowError> for BridgeError {
    fn from(e: EscrowError) -> Self {
        BridgeError::Escrow(e.to_string())
    }
}

/// Chain registry — manages known external chains and their configurations.
pub struct ChainRegistry {
    chains: Mutex<HashMap<String, ChainConfig>>,
}

impl ChainRegistry {
    pub fn new() -> Self {
        Self {
            chains: Mutex::new(HashMap::new()),
        }
    }

    /// Register a new external chain.
    pub fn register(&self, config: ChainConfig) {
        self.chains
            .lock()
            .unwrap()
            .insert(config.chain_id.0.clone(), config);
    }

    /// Get configuration for a chain.
    pub fn get(&self, chain_id: &ChainId) -> Option<ChainConfig> {
        self.chains.lock().unwrap().get(&chain_id.0).cloned()
    }

    /// List all registered chains.
    pub fn list(&self) -> Vec<ChainConfig> {
        self.chains.lock().unwrap().values().cloned().collect()
    }

    /// Check if a chain is registered and active.
    pub fn is_active(&self, chain_id: &ChainId) -> bool {
        self.chains
            .lock()
            .unwrap()
            .get(&chain_id.0)
            .is_some_and(|c| c.active)
    }
}

impl Default for ChainRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// The bridge engine — orchestrates cross-chain transfers using the
/// chain registry, escrow vault, and proof verification.
pub struct BridgeEngine {
    pub registry: ChainRegistry,
    pub escrow: EscrowVault,
    /// Processed message IDs (prevents replay).
    processed: Mutex<HashMap<MessageId, TransferStatus>>,
    /// Next sequence number per (source, dest) pair.
    sequences: Mutex<HashMap<(String, String), u64>>,
}

impl BridgeEngine {
    pub fn new() -> Self {
        Self {
            registry: ChainRegistry::new(),
            escrow: EscrowVault::new(),
            processed: Mutex::new(HashMap::new()),
            sequences: Mutex::new(HashMap::new()),
        }
    }

    /// Initiate an outbound transfer: lock tokens on rust-bc.
    pub fn initiate_transfer(
        &self,
        sender: &str,
        recipient: &str,
        amount: u64,
        denom: &str,
        dest_chain: &ChainId,
        block_height: u64,
    ) -> Result<BridgeMessage, BridgeError> {
        // Validate destination chain.
        let config = self
            .registry
            .get(dest_chain)
            .ok_or_else(|| BridgeError::UnknownChain(dest_chain.0.clone()))?;

        if !config.active {
            return Err(BridgeError::ChainNotActive(dest_chain.0.clone()));
        }

        if config.max_transfer > 0 && amount > config.max_transfer {
            return Err(BridgeError::ExceedsMax {
                amount,
                max: config.max_transfer,
            });
        }

        // Generate message ID and sequence.
        let sequence = self.next_sequence(&ChainId::native(), dest_chain);
        let msg_id = Self::compute_message_id(sender, recipient, amount, sequence);

        // Lock tokens in escrow.
        self.escrow
            .lock(msg_id, sender, amount, denom, dest_chain, block_height)?;

        // Build the outbound message.
        let message = BridgeMessage {
            id: msg_id,
            source_chain: ChainId::native(),
            dest_chain: dest_chain.clone(),
            sequence,
            payload: MessagePayload::TokenTransfer {
                sender: sender.to_string(),
                recipient: recipient.to_string(),
                amount,
                denom: denom.to_string(),
            },
            source_height: block_height,
            source_timestamp: 0,
            proof: None,
        };

        self.processed
            .lock()
            .unwrap()
            .insert(msg_id, TransferStatus::Pending);

        Ok(message)
    }

    /// Process an inbound transfer: verify proof and mint wrapped tokens.
    pub fn process_inbound(
        &self,
        message: &BridgeMessage,
        current_height: u64,
    ) -> Result<(), BridgeError> {
        // Check source chain.
        let config = self
            .registry
            .get(&message.source_chain)
            .ok_or_else(|| BridgeError::UnknownChain(message.source_chain.0.clone()))?;

        if !config.active {
            return Err(BridgeError::ChainNotActive(
                message.source_chain.0.clone(),
            ));
        }

        // Check confirmations.
        let confirmations = current_height.saturating_sub(message.source_height);
        if confirmations < config.min_confirmations {
            return Err(BridgeError::InsufficientConfirmations {
                have: confirmations,
                need: config.min_confirmations,
            });
        }

        // Verify proof.
        let proof = message
            .proof
            .as_ref()
            .ok_or_else(|| BridgeError::ProofFailed("missing proof".into()))?;

        let msg_bytes = serde_json::to_vec(&message.payload)
            .map_err(|e| BridgeError::ProofFailed(e.to_string()))?;

        if !verifier::verify_merkle_proof(&msg_bytes, proof) {
            return Err(BridgeError::ProofFailed(
                "merkle proof verification failed".into(),
            ));
        }

        // Replay protection.
        {
            let mut processed = self.processed.lock().unwrap();
            if processed.contains_key(&message.id) {
                return Err(BridgeError::ProofFailed("message already processed".into()));
            }
            processed.insert(message.id, TransferStatus::Completed);
        }

        // Mint wrapped tokens.
        if let MessagePayload::TokenTransfer {
            ref recipient,
            amount,
            ref denom,
            ..
        } = message.payload
        {
            self.escrow
                .mint(recipient, amount, &message.source_chain, denom)?;
        }

        Ok(())
    }

    fn next_sequence(&self, source: &ChainId, dest: &ChainId) -> u64 {
        let key = (source.0.clone(), dest.0.clone());
        let mut seqs = self.sequences.lock().unwrap();
        let seq = seqs.entry(key).or_insert(0);
        *seq += 1;
        *seq
    }

    fn compute_message_id(sender: &str, recipient: &str, amount: u64, sequence: u64) -> MessageId {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(sender.as_bytes());
        hasher.update(recipient.as_bytes());
        hasher.update(amount.to_le_bytes());
        hasher.update(sequence.to_le_bytes());
        hasher.finalize().into()
    }
}

impl Default for BridgeEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eth_config() -> ChainConfig {
        ChainConfig {
            chain_id: ChainId("ethereum".into()),
            name: "Ethereum Mainnet".into(),
            protocol: BridgeType::LightClient,
            active: true,
            min_confirmations: 12,
            max_transfer: 1_000_000,
        }
    }

    fn engine_with_eth() -> BridgeEngine {
        let e = BridgeEngine::new();
        e.registry.register(eth_config());
        e
    }

    // --- initiate_transfer ---

    #[test]
    fn initiate_outbound_transfer() {
        let e = engine_with_eth();
        let eth = ChainId("ethereum".into());

        let msg = e
            .initiate_transfer("alice", "0xBob", 1000, "NOTA", &eth, 100)
            .unwrap();

        assert_eq!(msg.source_chain, ChainId::native());
        assert_eq!(msg.dest_chain, eth);
        assert_eq!(msg.sequence, 1);
        assert!(matches!(
            msg.payload,
            MessagePayload::TokenTransfer { amount: 1000, .. }
        ));
        assert_eq!(e.escrow.total_locked(), 1000);
    }

    #[test]
    fn initiate_transfer_unknown_chain_fails() {
        let e = BridgeEngine::new();
        let err = e
            .initiate_transfer("alice", "bob", 100, "NOTA", &ChainId("unknown".into()), 1)
            .unwrap_err();
        assert!(matches!(err, BridgeError::UnknownChain(_)));
    }

    #[test]
    fn initiate_transfer_inactive_chain_fails() {
        let e = BridgeEngine::new();
        e.registry.register(ChainConfig {
            active: false,
            ..eth_config()
        });
        let err = e
            .initiate_transfer("alice", "bob", 100, "NOTA", &ChainId("ethereum".into()), 1)
            .unwrap_err();
        assert!(matches!(err, BridgeError::ChainNotActive(_)));
    }

    #[test]
    fn initiate_transfer_exceeds_max_fails() {
        let e = engine_with_eth();
        let err = e
            .initiate_transfer(
                "alice",
                "bob",
                2_000_000,
                "NOTA",
                &ChainId("ethereum".into()),
                1,
            )
            .unwrap_err();
        assert!(matches!(err, BridgeError::ExceedsMax { .. }));
    }

    #[test]
    fn sequence_increments_per_pair() {
        let e = engine_with_eth();
        let eth = ChainId("ethereum".into());

        let m1 = e
            .initiate_transfer("alice", "bob", 100, "NOTA", &eth, 1)
            .unwrap();
        let m2 = e
            .initiate_transfer("alice", "bob", 200, "NOTA", &eth, 2)
            .unwrap();

        assert_eq!(m1.sequence, 1);
        assert_eq!(m2.sequence, 2);
    }

    // --- process_inbound ---

    #[test]
    fn process_inbound_with_valid_proof() {
        let e = engine_with_eth();
        let eth = ChainId("ethereum".into());

        let payload = MessagePayload::TokenTransfer {
            sender: "0xAlice".into(),
            recipient: "bob".into(),
            amount: 500,
            denom: "wETH".into(),
        };
        let msg_bytes = serde_json::to_vec(&payload).unwrap();

        // Build a merkle tree with this message.
        let (_, proofs) = verifier::build_merkle_tree(&[&msg_bytes]);

        let message = BridgeMessage {
            id: [1u8; 32],
            source_chain: eth.clone(),
            dest_chain: ChainId::native(),
            sequence: 1,
            payload,
            source_height: 100,
            source_timestamp: 0,
            proof: Some(proofs[0].clone()),
        };

        // Current height = 100 + 12 confirmations.
        e.process_inbound(&message, 112).unwrap();

        assert_eq!(e.escrow.wrapped_balance("bob", &eth, "wETH"), 500);
    }

    #[test]
    fn process_inbound_insufficient_confirmations_fails() {
        let e = engine_with_eth();
        let eth = ChainId("ethereum".into());

        let payload = MessagePayload::TokenTransfer {
            sender: "0xAlice".into(),
            recipient: "bob".into(),
            amount: 500,
            denom: "wETH".into(),
        };
        let msg_bytes = serde_json::to_vec(&payload).unwrap();
        let (_, proofs) = verifier::build_merkle_tree(&[&msg_bytes]);

        let message = BridgeMessage {
            id: [2u8; 32],
            source_chain: eth,
            dest_chain: ChainId::native(),
            sequence: 1,
            payload,
            source_height: 100,
            source_timestamp: 0,
            proof: Some(proofs[0].clone()),
        };

        // Only 5 confirmations (need 12).
        let err = e.process_inbound(&message, 105).unwrap_err();
        assert!(matches!(
            err,
            BridgeError::InsufficientConfirmations { have: 5, need: 12 }
        ));
    }

    #[test]
    fn process_inbound_invalid_proof_fails() {
        let e = engine_with_eth();
        let eth = ChainId("ethereum".into());

        let payload = MessagePayload::TokenTransfer {
            sender: "0xAlice".into(),
            recipient: "bob".into(),
            amount: 500,
            denom: "wETH".into(),
        };

        // Proof for different data.
        let (_, proofs) = verifier::build_merkle_tree(&[b"wrong data"]);

        let message = BridgeMessage {
            id: [3u8; 32],
            source_chain: eth,
            dest_chain: ChainId::native(),
            sequence: 1,
            payload,
            source_height: 100,
            source_timestamp: 0,
            proof: Some(proofs[0].clone()),
        };

        let err = e.process_inbound(&message, 200).unwrap_err();
        assert!(matches!(err, BridgeError::ProofFailed(_)));
    }

    #[test]
    fn process_inbound_replay_rejected() {
        let e = engine_with_eth();
        let eth = ChainId("ethereum".into());

        let payload = MessagePayload::TokenTransfer {
            sender: "0xAlice".into(),
            recipient: "bob".into(),
            amount: 500,
            denom: "wETH".into(),
        };
        let msg_bytes = serde_json::to_vec(&payload).unwrap();
        let (_, proofs) = verifier::build_merkle_tree(&[&msg_bytes]);

        let message = BridgeMessage {
            id: [4u8; 32],
            source_chain: eth,
            dest_chain: ChainId::native(),
            sequence: 1,
            payload,
            source_height: 100,
            source_timestamp: 0,
            proof: Some(proofs[0].clone()),
        };

        e.process_inbound(&message, 200).unwrap();
        // Replay the same message.
        let err = e.process_inbound(&message, 201).unwrap_err();
        assert!(matches!(err, BridgeError::ProofFailed(_)));
    }

    // --- chain registry ---

    #[test]
    fn registry_list_chains() {
        let e = engine_with_eth();
        let chains = e.registry.list();
        assert_eq!(chains.len(), 1);
        assert_eq!(chains[0].chain_id.0, "ethereum");
    }

    #[test]
    fn registry_is_active() {
        let e = engine_with_eth();
        assert!(e.registry.is_active(&ChainId("ethereum".into())));
        assert!(!e.registry.is_active(&ChainId("unknown".into())));
    }
}
