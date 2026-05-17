//! Asset Registry — certified digital identity for physical assets.
//!
//! Generic module for registering physical assets (vehicles, equipment,
//! machinery) with a DID, tracking their lifecycle via signed events,
//! and providing certified data export for third parties.
//!
//! Designed for reuse across industries. Client-specific adapters
//! (e.g., MyScania telemetry) feed data into this module via the
//! ingestion endpoints.

pub mod compliance;
pub mod tokenization;
pub mod types;
