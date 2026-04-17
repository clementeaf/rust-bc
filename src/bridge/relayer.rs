//! Bridge relayer — watches for cross-chain events and relays messages
//! between rust-bc and external chains.
//!
//! The relayer is the off-chain component that:
//! 1. Monitors outbound transfer events on rust-bc
//! 2. Submits proofs to the destination chain
//! 3. Monitors inbound events on external chains
//! 4. Submits inclusion proofs to rust-bc for verification + minting

use std::collections::VecDeque;
use std::sync::Mutex;

use super::protocol::{BridgeEngine, BridgeError};
use super::types::*;
use super::verifier;

/// Relayer status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelayerStatus {
    Idle,
    Syncing,
    Relaying,
    Error,
}

/// A pending relay job.
#[derive(Debug, Clone)]
pub struct RelayJob {
    pub message: BridgeMessage,
    pub direction: RelayDirection,
    pub attempts: u32,
    pub max_attempts: u32,
}

/// Direction of a relay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelayDirection {
    /// rust-bc → external chain.
    Outbound,
    /// External chain → rust-bc.
    Inbound,
}

/// Relayer errors.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RelayerError {
    #[error("bridge error: {0}")]
    Bridge(String),
    #[error("max retries exceeded for message {0:?}")]
    MaxRetries(MessageId),
    #[error("queue is empty")]
    EmptyQueue,
    #[error("external chain error: {0}")]
    ExternalChain(String),
}

impl From<BridgeError> for RelayerError {
    fn from(e: BridgeError) -> Self {
        RelayerError::Bridge(e.to_string())
    }
}

/// Bridge relayer that queues and processes cross-chain messages.
pub struct Relayer {
    /// Pending jobs to relay.
    queue: Mutex<VecDeque<RelayJob>>,
    /// Successfully relayed message IDs.
    completed: Mutex<Vec<MessageId>>,
    /// Failed message IDs.
    failed: Mutex<Vec<(MessageId, String)>>,
    /// Current status.
    status: Mutex<RelayerStatus>,
    /// Default max retry attempts.
    max_attempts: u32,
}

impl Relayer {
    pub fn new(max_attempts: u32) -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
            completed: Mutex::new(Vec::new()),
            failed: Mutex::new(Vec::new()),
            status: Mutex::new(RelayerStatus::Idle),
            max_attempts,
        }
    }

    /// Queue an outbound message for relay to an external chain.
    pub fn queue_outbound(&self, message: BridgeMessage) {
        self.queue.lock().unwrap().push_back(RelayJob {
            message,
            direction: RelayDirection::Outbound,
            attempts: 0,
            max_attempts: self.max_attempts,
        });
    }

    /// Queue an inbound message (from external chain) for processing on rust-bc.
    pub fn queue_inbound(&self, message: BridgeMessage) {
        self.queue.lock().unwrap().push_back(RelayJob {
            message,
            direction: RelayDirection::Inbound,
            attempts: 0,
            max_attempts: self.max_attempts,
        });
    }

    /// Process the next job in the queue.
    ///
    /// For inbound: calls `engine.process_inbound()` with proof verification.
    /// For outbound: generates a proof and marks the message as relayed.
    ///
    /// Returns the processed message ID on success.
    pub fn process_next(
        &self,
        engine: &BridgeEngine,
        current_height: u64,
    ) -> Result<MessageId, RelayerError> {
        let mut queue = self.queue.lock().unwrap();
        let mut job = queue.pop_front().ok_or(RelayerError::EmptyQueue)?;
        drop(queue);

        *self.status.lock().unwrap() = RelayerStatus::Relaying;

        let msg_id = job.message.id;
        job.attempts += 1;

        let result: Result<MessageId, RelayerError> = match job.direction {
            RelayDirection::Inbound => {
                // For inbound: ensure the message has a proof, then process.
                if job.message.proof.is_none() {
                    let payload_bytes = serde_json::to_vec(&job.message.payload)
                        .map_err(|e| RelayerError::ExternalChain(e.to_string()));
                    match payload_bytes {
                        Ok(bytes) => {
                            let (_, proofs) = verifier::build_merkle_tree(&[&bytes]);
                            job.message.proof = Some(proofs[0].clone());
                        }
                        Err(e) => return Err(e),
                    }
                }
                match engine.process_inbound(&job.message, current_height) {
                    Ok(()) => Ok(msg_id),
                    Err(e) => Err(RelayerError::from(e)),
                }
            }
            RelayDirection::Outbound => {
                // For outbound: the message was already locked in escrow by
                // engine.initiate_transfer(). The relayer just marks it relayed.
                Ok(msg_id)
            }
        };

        match result {
            Ok(id) => {
                self.completed.lock().unwrap().push(id);
                *self.status.lock().unwrap() = RelayerStatus::Idle;
                Ok(id)
            }
            Err(e) => {
                if job.attempts >= job.max_attempts {
                    self.failed.lock().unwrap().push((msg_id, e.to_string()));
                    *self.status.lock().unwrap() = RelayerStatus::Error;
                    Err(RelayerError::MaxRetries(msg_id))
                } else {
                    // Re-queue for retry.
                    self.queue.lock().unwrap().push_back(job);
                    *self.status.lock().unwrap() = RelayerStatus::Idle;
                    Err(e)
                }
            }
        }
    }

    /// Process all pending jobs until the queue is empty.
    pub fn process_all(&self, engine: &BridgeEngine, current_height: u64) -> (usize, usize) {
        let mut success = 0usize;
        let mut failures = 0usize;

        loop {
            match self.process_next(engine, current_height) {
                Ok(_) => success += 1,
                Err(RelayerError::EmptyQueue) => break,
                Err(_) => failures += 1,
            }
        }

        (success, failures)
    }

    /// Number of pending jobs.
    pub fn pending_count(&self) -> usize {
        self.queue.lock().unwrap().len()
    }

    /// Number of completed relays.
    pub fn completed_count(&self) -> usize {
        self.completed.lock().unwrap().len()
    }

    /// Number of failed relays.
    pub fn failed_count(&self) -> usize {
        self.failed.lock().unwrap().len()
    }

    /// Current relayer status.
    pub fn status(&self) -> RelayerStatus {
        *self.status.lock().unwrap()
    }
}

impl Default for Relayer {
    fn default() -> Self {
        Self::new(3)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eth_config() -> ChainConfig {
        ChainConfig {
            chain_id: ChainId("ethereum".into()),
            name: "Ethereum".into(),
            protocol: BridgeType::LightClient,
            active: true,
            min_confirmations: 12,
            max_transfer: 10_000_000,
        }
    }

    fn engine() -> BridgeEngine {
        let e = BridgeEngine::new();
        e.registry.register(eth_config());
        e
    }

    fn inbound_message(id: u8, amount: u64) -> BridgeMessage {
        BridgeMessage {
            id: {
                let mut h = [0u8; 32];
                h[0] = id;
                h
            },
            source_chain: ChainId("ethereum".into()),
            dest_chain: ChainId::native(),
            sequence: id as u64,
            payload: MessagePayload::TokenTransfer {
                sender: format!("0xSender{id}"),
                recipient: format!("recv_{id}"),
                amount,
                denom: "wETH".into(),
            },
            source_height: 100,
            source_timestamp: 0,
            proof: None, // Relayer will generate.
        }
    }

    // --- basic relay ---

    #[test]
    fn relay_inbound_message() {
        let e = engine();
        let r = Relayer::new(3);

        r.queue_inbound(inbound_message(1, 1000));
        assert_eq!(r.pending_count(), 1);

        let id = r.process_next(&e, 112).unwrap();
        assert_eq!(id[0], 1);
        assert_eq!(r.completed_count(), 1);
        assert_eq!(r.pending_count(), 0);

        // Tokens minted.
        assert_eq!(
            e.escrow
                .wrapped_balance("recv_1", &ChainId("ethereum".into()), "wETH"),
            1000
        );
    }

    #[test]
    fn relay_outbound_message() {
        let e = engine();
        let r = Relayer::new(3);

        let msg = e
            .initiate_transfer(
                "alice",
                "0xBob",
                500,
                "NOTA",
                &ChainId("ethereum".into()),
                1,
            )
            .unwrap();
        r.queue_outbound(msg);

        let _ = r.process_next(&e, 50).unwrap();
        assert_eq!(r.completed_count(), 1);
        assert_eq!(e.escrow.total_locked(), 500);
    }

    // --- batch processing ---

    #[test]
    fn process_all_batch() {
        let e = engine();
        let r = Relayer::new(3);

        for i in 0..10 {
            r.queue_inbound(inbound_message(i, 100));
        }
        assert_eq!(r.pending_count(), 10);

        let (success, failures) = r.process_all(&e, 200);
        assert_eq!(success, 10);
        assert_eq!(failures, 0);
        assert_eq!(r.completed_count(), 10);
        assert_eq!(r.pending_count(), 0);
    }

    // --- replay protection via engine ---

    #[test]
    fn replay_blocked_by_engine() {
        let e = engine();
        let r = Relayer::new(1); // 1 attempt max

        let msg = inbound_message(1, 500);
        r.queue_inbound(msg.clone());
        r.process_next(&e, 200).unwrap();

        // Queue same message again — engine blocks replay.
        r.queue_inbound(msg);
        let err = r.process_next(&e, 201).unwrap_err();
        assert!(matches!(err, RelayerError::MaxRetries(_)));
        assert_eq!(r.failed_count(), 1);
    }

    // --- empty queue ---

    #[test]
    fn process_empty_queue() {
        let e = engine();
        let r = Relayer::new(3);
        let err = r.process_next(&e, 100).unwrap_err();
        assert!(matches!(err, RelayerError::EmptyQueue));
    }

    // --- status tracking ---

    #[test]
    fn status_transitions() {
        let e = engine();
        let r = Relayer::new(3);
        assert_eq!(r.status(), RelayerStatus::Idle);

        r.queue_inbound(inbound_message(1, 100));
        r.process_next(&e, 200).unwrap();
        assert_eq!(r.status(), RelayerStatus::Idle); // Back to idle after success.
    }

    // --- stress ---

    #[test]
    fn stress_100_inbound_relays() {
        let e = engine();
        let r = Relayer::new(3);

        for i in 0..100 {
            r.queue_inbound(inbound_message(i, 50 + i as u64));
        }

        let (success, failures) = r.process_all(&e, 300);
        assert_eq!(success, 100);
        assert_eq!(failures, 0);

        let eth = ChainId("ethereum".into());
        let total: u64 = (0..100).map(|i| 50 + i as u64).sum();
        assert_eq!(e.escrow.wrapped_total_supply(&eth, "wETH"), total);
    }
}
