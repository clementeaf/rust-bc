use std::sync::{Arc, Mutex, RwLock};

use crate::airdrop::AirdropManager;
use crate::billing::BillingManager;
use crate::block_storage::BlockStorage;
use crate::blockchain::Blockchain;
use crate::cache::BalanceCache;
use crate::checkpoint::CheckpointManager;
use crate::metrics::MetricsCollector;
use crate::models::{Mempool, WalletManager};
use crate::network::Node;
use crate::pruning::PruningManager;
use crate::smart_contracts::ContractManager;
use crate::staking::StakingManager;
use crate::storage::traits::BlockStore;
use crate::transaction_validation::TransactionValidator;

/// Shared application state for the HTTP API layer.
#[derive(Clone)]
pub struct AppState {
    pub blockchain: Arc<Mutex<Blockchain>>,
    pub wallet_manager: Arc<Mutex<WalletManager>>,
    pub block_storage: Option<Arc<BlockStorage>>,
    pub node: Option<Arc<Node>>,
    pub mempool: Arc<Mutex<Mempool>>,
    pub balance_cache: Arc<BalanceCache>,
    pub billing_manager: Arc<BillingManager>,
    pub contract_manager: Arc<RwLock<ContractManager>>,
    pub staking_manager: Arc<StakingManager>,
    pub airdrop_manager: Arc<AirdropManager>,
    pub pruning_manager: Option<Arc<PruningManager>>,
    pub checkpoint_manager: Option<Arc<Mutex<CheckpointManager>>>,
    pub transaction_validator: Arc<Mutex<TransactionValidator>>,
    pub metrics: Arc<MetricsCollector>,
    /// New storage layer (MemoryStore or future RocksDB).
    pub store: Option<Arc<dyn BlockStore>>,
}
