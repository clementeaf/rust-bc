//! Testnet node — minimal blockchain node for distributed validation.
//!
//! Holds mempool, chain, account state, and handles incoming messages.

use std::net::SocketAddr;
use std::sync::Mutex;

use log::info;

use crate::account::{AccountStore, MemoryAccountStore};
use crate::identity::signing::{SigningProvider, SoftwareSigningProvider};
use crate::tokenomics::economics::EconomicsState;
use crate::transaction::block_version::{validate_block_versioned, AnyBlock, ChainConfig};
use crate::transaction::compact_block::{
    reconstruct_compact_block, CompactBlock, CompactBlockHeader, MissingCompactRequest,
    MissingCompactResponse, SegWitBlock, SegWitMempool,
};
use crate::transaction::native::NativeTransaction;
use crate::transaction::pqc_validation::PqcValidationConfig;
use crate::transaction::segwit::{compute_tx_root, compute_witness_root, TxCore, TxWitness};
use crate::transaction::verification_cache::VerificationCache;

use super::client;
use super::messages::NetworkMessage;
use super::peer::Peer;

const CHAIN_ID: u64 = 9999; // testnet

/// Configuration for a testnet node.
#[derive(Debug, Clone)]
pub struct NodeConfig {
    pub addr: SocketAddr,
    pub peers: Vec<SocketAddr>,
    pub proposer_address: String,
}

/// Shared state for a testnet node, protected by interior mutability.
pub struct NodeHandle {
    pub config: NodeConfig,
    state: Mutex<NodeState>,
}

struct NodeState {
    mempool: SegWitMempool,
    chain: Vec<SegWitBlock>,
    accounts: MemoryAccountStore,
    #[allow(dead_code)]
    economics: EconomicsState,
    cache: VerificationCache,
    signer: SoftwareSigningProvider,
    chain_config: ChainConfig,
    pqc_config: PqcValidationConfig,
}

impl NodeHandle {
    /// Create a new testnet node with genesis allocations.
    pub fn new(config: NodeConfig, genesis: &[(&str, u64)]) -> Self {
        let signer = SoftwareSigningProvider::generate();
        Self {
            config,
            state: Mutex::new(NodeState {
                mempool: SegWitMempool::new(),
                chain: Vec::new(),
                accounts: MemoryAccountStore::with_genesis(genesis),
                economics: EconomicsState::default(),
                cache: VerificationCache::new(1024),
                signer,
                chain_config: ChainConfig {
                    segwit_pqc_activation_height: 0, // SegWit from genesis
                },
                pqc_config: PqcValidationConfig {
                    enforce_fees: false, // simplified for testnet
                    use_cache: true,
                    parallel_verify: false,
                },
            }),
        }
    }

    /// Handle an incoming network message.
    pub async fn handle_message(&self, msg: NetworkMessage, peer: &mut Peer) {
        match msg {
            NetworkMessage::NewTransaction(core, witness) => {
                self.on_new_transaction(core, witness);
            }
            NetworkMessage::NewBlock(block) => {
                self.on_new_block(block).await;
            }
            NetworkMessage::CompactBlock(compact) => {
                self.on_compact_block(compact, peer).await;
            }
            NetworkMessage::RequestMissing(req) => {
                self.on_request_missing(req, peer).await;
            }
            NetworkMessage::ResponseMissing(_resp) => {
                info!(
                    "[testnet:{}] received missing response (fallback to full blocks)",
                    self.config.addr
                );
            }
            NetworkMessage::SyncRequest => {
                self.on_sync_request(peer).await;
            }
            NetworkMessage::SyncResponse(blocks) => {
                self.on_sync_response(blocks);
            }
            NetworkMessage::MineBlock => {
                let result = self.mine_block().await;
                let (height, tx_count) = match result {
                    Some(b) => (b.header.height, b.tx_cores.len()),
                    None => (self.chain_height(), 0),
                };
                let _ = peer
                    .send(&NetworkMessage::MineBlockResponse { height, tx_count })
                    .await;
            }
            NetworkMessage::QueryBalance { address } => {
                let balance = self.get_balance(&address);
                let nonce = self.get_nonce(&address);
                let _ = peer
                    .send(&NetworkMessage::BalanceResponse {
                        address,
                        balance,
                        nonce,
                    })
                    .await;
            }
            NetworkMessage::MineBlockResponse { .. } | NetworkMessage::BalanceResponse { .. } => {
                // Responses handled by the CLI client, not the node
            }
        }
    }

    // ── Transaction handling ───────────────────────────────────────────

    fn on_new_transaction(&self, core: TxCore, witness: TxWitness) {
        let mut state = self.state.lock().unwrap();
        info!(
            "[testnet:{}] received tx from={} amount={}",
            self.config.addr, core.from, core.amount
        );
        state.mempool.insert(core, witness);
    }

    /// Submit a transaction to this node's mempool and broadcast.
    pub async fn submit_transaction(&self, core: TxCore, witness: TxWitness) {
        {
            let mut state = self.state.lock().unwrap();
            state.mempool.insert(core.clone(), witness.clone());
        }
        info!(
            "[testnet:{}] submitted tx from={} amount={}",
            self.config.addr, core.from, core.amount
        );
        let msg = NetworkMessage::NewTransaction(core, witness);
        client::broadcast(&self.config.peers, &msg).await;
    }

    // ── Block handling ─────────────────────────────────────────────────

    async fn on_new_block(&self, block: SegWitBlock) {
        let height = block.header.height;
        info!(
            "[testnet:{}] received block height={}",
            self.config.addr, height
        );
        if self.validate_and_apply_block(&block) {
            info!(
                "[testnet:{}] validated block height={}",
                self.config.addr, height
            );
        } else {
            info!(
                "[testnet:{}] rejected block height={}",
                self.config.addr, height
            );
        }
    }

    async fn on_compact_block(&self, compact: CompactBlock, peer: &mut Peer) {
        let height = compact.header.height;
        info!(
            "[testnet:{}] received compact block height={}",
            self.config.addr, height
        );

        // Try full reconstruction first
        let result = {
            let state = self.state.lock().unwrap();
            reconstruct_compact_block(&compact, &state.mempool)
        };

        match result {
            Ok(block) => {
                if self.validate_and_apply_block(&block) {
                    info!(
                        "[testnet:{}] reconstructed+validated compact block height={}",
                        self.config.addr, height
                    );
                }
            }
            Err(missing_req) => {
                info!(
                    "[testnet:{}] requesting {} missing cores, {} missing witnesses",
                    self.config.addr,
                    missing_req.missing_tx_core_ids.len(),
                    missing_req.missing_witness_ids.len()
                );
                let _ = peer
                    .send(&NetworkMessage::RequestMissing(missing_req))
                    .await;
            }
        }
    }

    async fn on_request_missing(&self, req: MissingCompactRequest, peer: &mut Peer) {
        let resp = {
            let state = self.state.lock().unwrap();
            let mut missing_cores = Vec::new();
            let mut missing_witnesses = Vec::new();
            for id in &req.missing_tx_core_ids {
                if let Some(core) = state.mempool.get_core(id) {
                    missing_cores.push(core.clone());
                }
            }
            for id in &req.missing_witness_ids {
                if let Some(witness) = state.mempool.get_witness(id) {
                    missing_witnesses.push(witness.clone());
                }
            }
            NetworkMessage::ResponseMissing(MissingCompactResponse {
                block_hash: req.block_hash,
                tx_cores: missing_cores,
                witnesses: missing_witnesses,
            })
        };
        let _ = peer.send(&resp).await;
    }

    // ── Sync ───────────────────────────────────────────────────────────

    async fn on_sync_request(&self, peer: &mut Peer) {
        let blocks = {
            let state = self.state.lock().unwrap();
            state.chain.clone()
        };
        info!(
            "[testnet:{}] sync: sending {} blocks",
            self.config.addr,
            blocks.len()
        );
        let _ = peer.send(&NetworkMessage::SyncResponse(blocks)).await;
    }

    fn on_sync_response(&self, blocks: Vec<SegWitBlock>) {
        info!(
            "[testnet:{}] sync: received {} blocks",
            self.config.addr,
            blocks.len()
        );
        for block in blocks {
            self.validate_and_apply_block(&block);
        }
        info!(
            "[testnet:{}] sync completed, chain height={}",
            self.config.addr,
            self.chain_height()
        );
    }

    // ── Block production ───────────────────────────────────────────────

    /// Mine a block from the current mempool and broadcast it.
    pub async fn mine_block(&self) -> Option<SegWitBlock> {
        let block = {
            let mut state = self.state.lock().unwrap();

            // Drain mempool into tx pairs
            let (cores, witnesses) = state.mempool.drain_all();
            if cores.is_empty() {
                info!(
                    "[testnet:{}] mine: empty mempool, skipping",
                    self.config.addr
                );
                return None;
            }

            let height = state.chain.len() as u64 + 1;
            let parent_hash = state
                .chain
                .last()
                .map(|b| b.header.hash)
                .unwrap_or([0u8; 32]);

            // Proposer identity
            let proposer_pk = state.signer.public_key();
            let proposer_addr = crate::account::address::address_from_pubkey(&proposer_pk);

            // Build SegWit block
            let tx_root = compute_tx_root(&cores);
            let witness_root = compute_witness_root(&witnesses);

            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            let header = CompactBlockHeader {
                height,
                hash: [0u8; 32],
                parent_hash,
                timestamp,
                proposer: proposer_addr,
            };

            let block = SegWitBlock {
                header,
                tx_cores: cores,
                witnesses,
                tx_root,
                witness_root,
            };

            // Execute transfers against account store
            for core in &block.tx_cores {
                let native = NativeTransaction::new_transfer_with_chain(
                    &core.from,
                    &core.to,
                    core.amount,
                    core.nonce,
                    core.fee,
                    core.chain_id,
                );
                match crate::transaction::native::execute_transfer_checked(
                    &state.accounts,
                    &native,
                    &block.header.proposer,
                    CHAIN_ID,
                ) {
                    Ok(_) => info!(
                        "[testnet:{}] executed tx from={} to={} amount={}",
                        self.config.addr, core.from, core.to, core.amount
                    ),
                    Err(e) => info!("[testnet:{}] tx execution failed: {e}", self.config.addr),
                }
            }

            state.chain.push(block.clone());
            info!(
                "[testnet:{}] mined block height={} txs={}",
                self.config.addr,
                height,
                block.tx_cores.len()
            );

            block
        };

        // Broadcast full block
        let msg = NetworkMessage::NewBlock(block.clone());
        client::broadcast(&self.config.peers, &msg).await;

        Some(block)
    }

    // ── Validation ─────────────────────────────────────────────────────

    fn validate_and_apply_block(&self, block: &SegWitBlock) -> bool {
        let mut state = self.state.lock().unwrap();

        let expected_height = state.chain.len() as u64 + 1;
        if block.header.height != expected_height {
            info!(
                "[testnet:{}] block height mismatch: got {}, expected {}",
                self.config.addr, block.header.height, expected_height
            );
            return false;
        }

        // Check parent hash
        let expected_parent = state
            .chain
            .last()
            .map(|b| b.header.hash)
            .unwrap_or([0u8; 32]);
        if block.header.parent_hash != expected_parent {
            info!(
                "[testnet:{}] parent hash mismatch at height {}",
                self.config.addr, block.header.height
            );
            return false;
        }

        // Clone config values to avoid borrow conflicts with cache
        let pqc_config = state.pqc_config.clone();
        let chain_config = state.chain_config.clone();

        // Validate with versioned pipeline
        let any_block = AnyBlock::SegWit(block.clone());
        if let Err(e) =
            validate_block_versioned(&any_block, &mut state.cache, &pqc_config, &chain_config)
        {
            info!(
                "[testnet:{}] block validation failed: {e}",
                self.config.addr
            );
            return false;
        }

        // Execute transfers against account store
        for core in &block.tx_cores {
            let native = NativeTransaction::new_transfer_with_chain(
                &core.from,
                &core.to,
                core.amount,
                core.nonce,
                core.fee,
                core.chain_id,
            );
            if let Err(e) = crate::transaction::native::execute_transfer_checked(
                &state.accounts,
                &native,
                &block.header.proposer,
                CHAIN_ID,
            ) {
                info!(
                    "[testnet:{}] tx execution failed during block apply: {e}",
                    self.config.addr
                );
                return false;
            }
        }

        state.chain.push(block.clone());
        true
    }

    // ── Queries ────────────────────────────────────────────────────────

    /// Current chain height.
    pub fn chain_height(&self) -> u64 {
        let state = self.state.lock().unwrap();
        state.chain.len() as u64
    }

    /// Get account balance.
    pub fn get_balance(&self, address: &str) -> u64 {
        let state = self.state.lock().unwrap();
        state
            .accounts
            .get_account(address)
            .map(|a| a.balance)
            .unwrap_or(0)
    }

    /// Get account nonce.
    pub fn get_nonce(&self, address: &str) -> u64 {
        let state = self.state.lock().unwrap();
        state
            .accounts
            .get_account(address)
            .map(|a| a.nonce)
            .unwrap_or(0)
    }

    /// Get the chain (clone).
    pub fn get_chain(&self) -> Vec<SegWitBlock> {
        let state = self.state.lock().unwrap();
        state.chain.clone()
    }

    /// Get the mempool size.
    pub fn mempool_size(&self) -> usize {
        let state = self.state.lock().unwrap();
        state.mempool.cores_len()
    }

    /// Get the node's signer public key.
    pub fn public_key(&self) -> Vec<u8> {
        let state = self.state.lock().unwrap();
        state.signer.public_key()
    }

    /// Request sync from a peer.
    pub async fn sync_from(&self, peer_addr: SocketAddr) {
        match client::send_to_peer(peer_addr, &NetworkMessage::SyncRequest).await {
            Ok(mut peer) => {
                // Wait for response
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                if let Ok(Some(msg)) = peer.recv().await {
                    self.handle_message(msg, &mut peer).await;
                }
            }
            Err(e) => info!("[testnet:{}] sync request failed: {e}", self.config.addr),
        }
    }
}
