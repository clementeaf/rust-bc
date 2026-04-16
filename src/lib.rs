// Librería para tests de integración
pub mod blockchain;
pub mod chain_validation;
pub mod contract_validation;
pub mod governance_contracts;
pub mod models;
pub mod multisig_contracts;
pub mod network_security;
pub mod oracle_collateral;
pub mod oracle_system;
pub mod plugin_contracts;
pub mod smart_contract;
pub mod staking_rewards;
pub mod transaction_validation;

// Phase 2 Week 2-3: Storage & Consensus Layers
pub mod consensus;
pub mod identity;
pub mod storage;

// Runtime stack (shared with binary for AppState and scaffold handlers)
pub mod airdrop;
pub mod billing;
pub mod block_storage;
pub mod cache;
pub mod checkpoint;
pub mod metrics;
pub mod network;
pub mod pruning;
pub mod smart_contracts;
pub mod staking;
pub mod state_reconstructor;
pub mod state_snapshot;

// Phase 3 Week 5: REST API Gateway
pub mod acl;
pub mod api;
pub mod app_state;
pub mod audit;
pub mod block_creation;
pub mod chaincode;
pub mod channel;
pub mod discovery;
pub mod endorsement;
pub mod events;
pub mod bridge;
pub mod gateway;
pub mod governance;
pub mod msp;
pub mod ordering;
pub mod pki;
pub mod private_data;
pub mod tls;
pub mod tokenomics;
pub mod transaction;

pub use app_state::AppState;
