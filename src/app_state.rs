use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
pub type StoreMap = Arc<RwLock<HashMap<String, Arc<dyn BlockStore>>>>;

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
use crate::endorsement::policy_store::PolicyStore;
use crate::endorsement::registry::OrgRegistry;
use crate::msp::CrlStore;
use crate::chaincode::{ChaincodeDefinitionStore, ChaincodePackageStore};
use crate::gateway::Gateway;
use crate::private_data::{CollectionRegistry, PrivateDataStore};
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
    /// Per-channel storage layer. Key `"default"` holds the main store.
    /// Wrapped in `RwLock` so channels can be added at runtime via `POST /channels`.
    pub store: StoreMap,
    /// Organization registry for endorsement policies.
    pub org_registry: Option<Arc<dyn OrgRegistry>>,
    /// Endorsement policy store.
    pub policy_store: Option<Arc<dyn PolicyStore>>,
    /// Certificate Revocation List store.
    pub crl_store: Option<Arc<dyn CrlStore>>,
    /// Private data side-store (one per node, shared across channels).
    pub private_data_store: Option<Arc<dyn PrivateDataStore>>,
    /// Registry of private data collection definitions.
    pub collection_registry: Option<Arc<dyn CollectionRegistry>>,
    /// Chaincode Wasm package store.
    pub chaincode_package_store: Option<Arc<dyn ChaincodePackageStore>>,
    /// Chaincode lifecycle definition store.
    pub chaincode_definition_store: Option<Arc<dyn ChaincodeDefinitionStore>>,
    /// Fabric Gateway — orchestrates endorse → order → commit.
    pub gateway: Option<Arc<Gateway>>,
}
