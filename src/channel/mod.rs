//! Channel model — a logical ledger scope with its own member orgs and endorsement policy.

pub mod registry;
pub mod config;
pub mod genesis;
pub use config::{ChannelConfig, ConfigUpdateType};

use crate::endorsement::policy::EndorsementPolicy;

/// A channel represents an isolated ledger partition.
///
/// Each channel has its own `BlockStore` (keyed by `channel_id` in `AppState`),
/// its own endorsement policy, and a restricted set of member/orderer orgs.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Channel {
    pub channel_id: String,
    pub member_org_ids: Vec<String>,
    pub orderer_org_ids: Vec<String>,
    /// Unix epoch seconds
    pub created_at: u64,
    pub endorsement_policy: EndorsementPolicy,
}

impl Channel {
    /// Add an org to the member list if not already present.
    pub fn add_member(&mut self, org_id: impl Into<String>) {
        let id = org_id.into();
        if !self.member_org_ids.contains(&id) {
            self.member_org_ids.push(id);
        }
    }

    /// Returns `true` if `org_id` is a channel member.
    pub fn is_member(&self, org_id: &str) -> bool {
        self.member_org_ids.iter().any(|o| o == org_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_channel() -> Channel {
        Channel {
            channel_id: "ch1".to_string(),
            member_org_ids: vec!["org1".to_string()],
            orderer_org_ids: vec!["orderer1".to_string()],
            created_at: 1_000_000,
            endorsement_policy: EndorsementPolicy::AnyOf(vec!["org1".to_string()]),
        }
    }

    #[test]
    fn creates_channel_with_basic_fields() {
        let ch = sample_channel();
        assert_eq!(ch.channel_id, "ch1");
        assert_eq!(ch.created_at, 1_000_000);
    }

    #[test]
    fn add_org_adds_when_not_present() {
        let mut ch = sample_channel();
        ch.add_member("org2");
        assert!(ch.is_member("org2"));
        assert_eq!(ch.member_org_ids.len(), 2);
    }

    #[test]
    fn add_org_is_idempotent() {
        let mut ch = sample_channel();
        ch.add_member("org1");
        assert_eq!(ch.member_org_ids.len(), 1);
    }

    #[test]
    fn is_member_returns_false_for_unknown_org() {
        let ch = sample_channel();
        assert!(!ch.is_member("unknown"));
    }
}
