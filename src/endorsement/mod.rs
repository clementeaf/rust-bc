//! Endorsement policy engine
//!
//! Implements Hyperledger Fabric-style endorsement policies using Ed25519 DID signing.

pub mod key_policy;
pub mod org;
pub mod policy;
pub mod policy_store;
pub mod registry;
pub mod types;
pub mod validator;

pub use key_policy::{KeyEndorsementStore, MemoryKeyEndorsementStore};
pub use org::Organization;
pub use policy::EndorsementPolicy;
pub use policy_store::{MemoryPolicyStore, PolicyStore};
pub use registry::{MemoryOrgRegistry, OrgRegistry};
pub use types::Endorsement;
pub use validator::{
    validate_endorsements, validate_endorsements_for_writes, verify_endorsement, EndorsementError,
};
