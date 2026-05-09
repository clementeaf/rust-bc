//! ISO compliance module — validators and schemas for financial standards.
//!
//! Provides validation for:
//! - ISO 20022: financial messaging (pacs.008, pain.001, camt.053)
//! - ISO 3166-1: country codes
//! - ISO 4217: currency codes

pub mod erc3643;
pub mod iso20022;
pub mod iso3166;
pub mod iso4217;
pub mod iso8601;
