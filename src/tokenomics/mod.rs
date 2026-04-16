//! Protocol-native tokenomics for the NOTA token.
//!
//! Centralizes supply management, fee economics, storage deposits, and
//! issuance schedules that were previously scattered across blockchain.rs,
//! staking.rs, and billing.rs.

pub mod economics;
pub mod storage_deposit;
