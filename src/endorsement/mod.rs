//! Endorsement policy engine
//!
//! Implements Hyperledger Fabric-style endorsement policies using Ed25519 DID signing.

pub mod org;
pub mod registry;
pub mod policy;
pub mod policy_store;
pub mod types;
pub mod validator;

pub use org::Organization;
pub use registry::{OrgRegistry, MemoryOrgRegistry};
pub use policy::EndorsementPolicy;
pub use policy_store::{PolicyStore, MemoryPolicyStore};
pub use types::Endorsement;
pub use validator::{EndorsementError, verify_endorsement, validate_endorsements};
