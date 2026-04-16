//! BFT consensus primitives — votes, quorum certificates, and quorum validation.
//!
//! Implements a HotStuff-inspired BFT layer on top of the existing DAG consensus.
//! Raft remains available as an alternative backend for permissioned deployments.

pub mod quorum;
pub mod round;
pub mod round_manager;
pub mod types;
pub mod vote_collector;
