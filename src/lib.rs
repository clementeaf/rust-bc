#![feature(unsigned_is_multiple_of)]

// Librería para tests de integración
pub mod blockchain;
pub mod models;
pub mod chain_validation;
pub mod network_security;
pub mod transaction_validation;
pub mod governance_contracts;
pub mod staking_rewards;
pub mod multisig_contracts;
pub mod plugin_contracts;
pub mod contract_validation;
pub mod oracle_system;
pub mod oracle_collateral;
pub mod smart_contract;

// Phase 2 Week 2-3: Storage & Consensus Layers
pub mod storage;
pub mod consensus;
pub mod identity;

// Runtime stack (shared with binary for AppState and scaffold handlers)
pub mod block_storage;
pub mod cache;
pub mod checkpoint;
pub mod smart_contracts;
pub mod staking;
pub mod airdrop;
pub mod state_reconstructor;
pub mod state_snapshot;
pub mod pruning;
pub mod network;
pub mod billing;
pub mod metrics;

// Phase 3 Week 5: REST API Gateway
pub mod app_state;
pub mod api;
pub mod block_creation;
pub mod tls;
pub mod pki;
pub mod endorsement;
pub mod ordering;
pub mod transaction;
pub mod channel;
pub mod msp;
pub mod private_data;
pub mod chaincode;
pub mod gateway;
pub mod discovery;
pub mod events;

pub use app_state::AppState;
