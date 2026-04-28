//! Shared deterministic seeds for all chaos/property/fuzz tests.
//!
//! Using fixed seeds ensures reproducibility across CI runs.

pub const CI_SEEDS: &[u64] = &[1, 42, 1337, 9001, 123456789];
