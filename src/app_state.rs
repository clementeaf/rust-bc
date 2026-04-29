use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
pub type StoreMap = Arc<RwLock<HashMap<String, Arc<dyn BlockStore>>>>;

use crate::account::AccountStore;
use crate::acl::AclProvider;
use crate::airdrop::AirdropManager;
use crate::billing::BillingManager;
use crate::block_storage::BlockStorage;
use crate::blockchain::Blockchain;
use crate::cache::BalanceCache;
use crate::chaincode::{ChaincodeDefinitionStore, ChaincodePackageStore};
use crate::channel::config::ChannelConfig;
use crate::checkpoint::CheckpointManager;
use crate::discovery::service::DiscoveryService;
use crate::endorsement::policy_store::PolicyStore;
use crate::endorsement::registry::OrgRegistry;
use crate::events::EventBus;
use crate::gateway::Gateway;
use crate::governance::params::ParamRegistry;
use crate::governance::proposals::ProposalStore;
use crate::governance::voting::VoteStore;
use crate::metrics::MetricsCollector;
use crate::models::{Mempool, WalletManager};
use crate::msp::CrlStore;
use crate::network::Node;
use crate::ordering::OrderingBackend;
use crate::pin::store::PinStore;
use crate::private_data::{CollectionRegistry, PrivateDataStore};
use crate::pruning::PruningManager;
use crate::smart_contracts::ContractManager;
use crate::staking::StakingManager;
use crate::storage::traits::BlockStore;
use crate::tokenomics::economics::EconomicsState;
use crate::transaction::mempool::Mempool as NativeMempool;
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
    /// Service Discovery — peer registry and endorsement plan computation.
    pub discovery_service: Option<Arc<DiscoveryService>>,
    /// Event bus — fan-out channel for block and transaction events.
    pub event_bus: Arc<EventBus>,
    /// Per-channel config history. Key = channel_id; value = ordered list of configs (index 0 = genesis).
    pub channel_configs: Arc<RwLock<HashMap<String, Vec<ChannelConfig>>>>,
    /// ACL provider — maps resource names to endorsement policy references.
    pub acl_provider: Option<Arc<dyn AclProvider>>,
    /// Ordering backend — solo (default) or raft.
    pub ordering_backend: Option<Arc<dyn OrderingBackend>>,
    /// World state for snapshots and state queries.
    pub world_state: Option<Arc<dyn crate::storage::world_state::WorldState>>,
    /// Audit trail — immutable log of all API requests.
    pub audit_store: Option<Arc<dyn crate::audit::AuditStore>>,
    /// Governance — proposal store.
    pub proposal_store: Option<Arc<ProposalStore>>,
    /// Governance — vote store.
    pub vote_store: Option<Arc<VoteStore>>,
    /// Governance — protocol parameter registry.
    pub param_registry: Option<Arc<ParamRegistry>>,
    /// PIN store — DID-to-hashed-PIN mappings.
    pub pin_store: Option<Arc<dyn PinStore>>,
    // ── Cryptocurrency layer ───────────────────────────────────────────────
    /// Native account state (balances, nonces).
    pub account_store: Option<Arc<dyn AccountStore>>,
    /// Fee-ordered mempool for native transactions.
    pub native_mempool: Option<Arc<NativeMempool>>,
    /// Protocol economics state (supply, base fee, epoch).
    pub economics_state: Arc<Mutex<EconomicsState>>,
}
