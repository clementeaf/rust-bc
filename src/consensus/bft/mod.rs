//! BFT consensus primitives — votes, quorum certificates, and quorum validation.
//!
//! Implements a HotStuff-inspired BFT layer on top of the existing DAG consensus.
//! Raft remains available as an alternative backend for permissioned deployments.

pub mod types;
pub mod quorum;
