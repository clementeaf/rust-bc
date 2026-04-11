//! Channel configuration struct.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::endorsement::policy::EndorsementPolicy;
use crate::endorsement::policy_store::PolicyStore;
use crate::endorsement::registry::OrgRegistry;
use crate::endorsement::types::Endorsement;
use crate::endorsement::validator::{validate_endorsements, EndorsementError};

/// Errors that can occur when processing channel configuration.
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ChannelError {
    #[error("org already exists: {0}")]
    OrgAlreadyExists(String),
    #[error("org not found: {0}")]
    OrgNotFound(String),
    #[allow(dead_code)]
    #[error("unknown update type applied to channel")]
    UnknownUpdate,
    #[allow(dead_code)]
    #[error("modification policy not found for channel '{0}'")]
    PolicyNotFound(String),
    #[error("endorsement validation failed: {0}")]
    EndorsementFailed(String),
}

/// Full governance configuration of a channel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChannelConfig {
    /// Monotonically increasing version; incremented on every accepted config update.
    pub version: u64,
    /// MSP IDs of peer orgs that are members of this channel.
    pub member_orgs: Vec<String>,
    /// MSP IDs of orderer orgs serving this channel.
    pub orderer_orgs: Vec<String>,
    /// Default endorsement policy for the channel.
    pub endorsement_policy: EndorsementPolicy,
    /// ACL map: resource name → policy name.
    pub acls: HashMap<String, String>,
    /// Maximum number of transactions per block.
    pub batch_size: usize,
    /// Maximum time (ms) the orderer waits before cutting a block.
    pub batch_timeout_ms: u64,
    /// Anchor peers per org: org_id → list of "host:port" addresses.
    pub anchor_peers: HashMap<String, Vec<String>>,
}

impl Default for ChannelConfig {
    fn default() -> Self {
        Self {
            version: 0,
            member_orgs: Vec::new(),
            orderer_orgs: Vec::new(),
            endorsement_policy: EndorsementPolicy::AnyOf(Vec::new()),
            acls: HashMap::new(),
            batch_size: 100,
            batch_timeout_ms: 2000,
            anchor_peers: HashMap::new(),
        }
    }
}

/// A single configuration change applied to a [`ChannelConfig`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum ConfigUpdateType {
    AddOrg(String),
    RemoveOrg(String),
    SetPolicy(EndorsementPolicy),
    SetAcl {
        resource: String,
        policy_ref: String,
    },
    SetBatchSize(usize),
    SetBatchTimeout(u64),
    SetAnchorPeer {
        org_id: String,
        peer_address: String,
    },
}

/// Apply a slice of [`ConfigUpdateType`] changes to `config`, returning a new
/// [`ChannelConfig`] with `version` incremented. Returns an error on invalid operations.
///
/// `config` is never mutated — a full clone is made and returned.
pub fn apply_config_update(
    config: &ChannelConfig,
    updates: &[ConfigUpdateType],
) -> Result<ChannelConfig, ChannelError> {
    let mut next = config.clone();
    for update in updates {
        match update {
            ConfigUpdateType::AddOrg(org) => {
                if next.member_orgs.contains(org) {
                    return Err(ChannelError::OrgAlreadyExists(org.clone()));
                }
                next.member_orgs.push(org.clone());
            }
            ConfigUpdateType::RemoveOrg(org) => {
                let pos = next
                    .member_orgs
                    .iter()
                    .position(|o| o == org)
                    .ok_or_else(|| ChannelError::OrgNotFound(org.clone()))?;
                next.member_orgs.remove(pos);
            }
            ConfigUpdateType::SetPolicy(policy) => {
                next.endorsement_policy = policy.clone();
            }
            ConfigUpdateType::SetAcl {
                resource,
                policy_ref,
            } => {
                next.acls.insert(resource.clone(), policy_ref.clone());
            }
            ConfigUpdateType::SetBatchSize(size) => {
                next.batch_size = *size;
            }
            ConfigUpdateType::SetBatchTimeout(ms) => {
                next.batch_timeout_ms = *ms;
            }
            ConfigUpdateType::SetAnchorPeer {
                org_id,
                peer_address,
            } => {
                next.anchor_peers
                    .entry(org_id.clone())
                    .or_default()
                    .push(peer_address.clone());
            }
        }
    }
    next.version += 1;
    Ok(next)
}

/// Validate that `tx` carries enough endorsed signatures to satisfy the channel's
/// modification policy.
///
/// The modification policy is looked up from `policy_store` under the key
/// `"channel/{channel_id}/mod_policy"`. If absent, the channel's default
/// `endorsement_policy` is used as a fallback.
pub fn validate_config_tx(
    tx: &ConfigTransaction,
    current_config: &ChannelConfig,
    policy_store: &dyn PolicyStore,
    org_registry: &dyn OrgRegistry,
) -> Result<(), ChannelError> {
    let mod_policy_key = format!("channel/{}/mod_policy", tx.channel_id);
    let policy = policy_store
        .get_policy(&mod_policy_key)
        .unwrap_or_else(|_| current_config.endorsement_policy.clone());

    validate_endorsements(&tx.signatures, &policy, org_registry, None)
        .map_err(|e: EndorsementError| ChannelError::EndorsementFailed(e.to_string()))
}

/// A signed proposal to apply one or more [`ConfigUpdateType`] changes to a channel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfigTransaction {
    /// Unique transaction identifier.
    pub tx_id: String,
    /// ID of the channel this transaction targets.
    pub channel_id: String,
    /// Ordered list of configuration changes to apply.
    pub updates: Vec<ConfigUpdateType>,
    /// Endorsements from authorised signers.
    pub signatures: Vec<Endorsement>,
    /// Unix timestamp (seconds) when this transaction was created.
    pub created_at: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ChannelConfig {
        ChannelConfig {
            version: 1,
            member_orgs: vec!["org1".to_string(), "org2".to_string()],
            orderer_orgs: vec!["orderer".to_string()],
            endorsement_policy: EndorsementPolicy::AnyOf(vec!["org1".to_string()]),
            acls: HashMap::from([("peer/ChaincodeInvoke".to_string(), "OrgPolicy".to_string())]),
            batch_size: 50,
            batch_timeout_ms: 1000,
            anchor_peers: HashMap::from([(
                "org1".to_string(),
                vec!["peer0.org1:7051".to_string()],
            )]),
        }
    }

    #[test]
    fn creates_channel_config() {
        let cfg = sample();
        assert_eq!(cfg.version, 1);
        assert_eq!(cfg.member_orgs, vec!["org1", "org2"]);
        assert_eq!(cfg.batch_size, 50);
    }

    #[test]
    fn serde_roundtrip() {
        let cfg = sample();
        let json = serde_json::to_string(&cfg).expect("serialize");
        let restored: ChannelConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(cfg, restored);
    }

    #[test]
    fn default_values() {
        let cfg = ChannelConfig::default();
        assert_eq!(cfg.version, 0);
        assert_eq!(cfg.batch_size, 100);
        assert_eq!(cfg.batch_timeout_ms, 2000);
        assert!(cfg.member_orgs.is_empty());
        assert!(cfg.acls.is_empty());
        assert!(cfg.anchor_peers.is_empty());
    }

    fn roundtrip(update: ConfigUpdateType) -> ConfigUpdateType {
        let json = serde_json::to_string(&update).expect("serialize");
        serde_json::from_str(&json).expect("deserialize")
    }

    #[test]
    fn config_update_add_org_roundtrip() {
        let u = ConfigUpdateType::AddOrg("org3".to_string());
        assert_eq!(roundtrip(u.clone()), u);
    }

    #[test]
    fn config_update_remove_org_roundtrip() {
        let u = ConfigUpdateType::RemoveOrg("org2".to_string());
        assert_eq!(roundtrip(u.clone()), u);
    }

    #[test]
    fn config_update_set_policy_roundtrip() {
        let u = ConfigUpdateType::SetPolicy(EndorsementPolicy::AnyOf(vec!["org1".to_string()]));
        assert_eq!(roundtrip(u.clone()), u);
    }

    #[test]
    fn config_update_set_acl_roundtrip() {
        let u = ConfigUpdateType::SetAcl {
            resource: "peer/BlockEvents".to_string(),
            policy_ref: "AdminPolicy".to_string(),
        };
        assert_eq!(roundtrip(u.clone()), u);
    }

    #[test]
    fn config_update_set_batch_size_roundtrip() {
        let u = ConfigUpdateType::SetBatchSize(200);
        assert_eq!(roundtrip(u.clone()), u);
    }

    #[test]
    fn config_update_set_batch_timeout_roundtrip() {
        let u = ConfigUpdateType::SetBatchTimeout(5000);
        assert_eq!(roundtrip(u.clone()), u);
    }

    #[test]
    fn config_update_set_anchor_peer_roundtrip() {
        let u = ConfigUpdateType::SetAnchorPeer {
            org_id: "org1".to_string(),
            peer_address: "peer0.org1:7051".to_string(),
        };
        assert_eq!(roundtrip(u.clone()), u);
    }

    fn sample_endorsement() -> Endorsement {
        Endorsement {
            signer_did: "did:example:admin".to_string(),
            org_id: "org1".to_string(),
            signature: vec![1u8; 64],
            payload_hash: [2u8; 32],
            timestamp: 1_000_000,
        }
    }

    fn sample_config_tx() -> ConfigTransaction {
        ConfigTransaction {
            tx_id: "tx-001".to_string(),
            channel_id: "channel-alpha".to_string(),
            updates: vec![
                ConfigUpdateType::AddOrg("org3".to_string()),
                ConfigUpdateType::SetBatchSize(200),
            ],
            signatures: vec![sample_endorsement()],
            created_at: 1_700_000_000,
        }
    }

    #[test]
    fn config_transaction_fields() {
        let tx = sample_config_tx();
        assert_eq!(tx.tx_id, "tx-001");
        assert_eq!(tx.channel_id, "channel-alpha");
        assert_eq!(tx.updates.len(), 2);
        assert_eq!(tx.signatures.len(), 1);
        assert_eq!(tx.created_at, 1_700_000_000);
    }

    #[test]
    fn config_transaction_serde_roundtrip() {
        let tx = sample_config_tx();
        let json = serde_json::to_string(&tx).expect("serialize");
        let restored: ConfigTransaction = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(tx, restored);
    }

    #[test]
    fn config_transaction_empty_updates_allowed() {
        let tx = ConfigTransaction {
            tx_id: "tx-002".to_string(),
            channel_id: "channel-beta".to_string(),
            updates: vec![],
            signatures: vec![],
            created_at: 0,
        };
        assert!(tx.updates.is_empty());
        assert!(tx.signatures.is_empty());
    }

    #[test]
    fn apply_add_org_increments_version() {
        let cfg = sample();
        let next = apply_config_update(&cfg, &[ConfigUpdateType::AddOrg("org3".to_string())])
            .expect("apply");
        assert_eq!(next.version, cfg.version + 1);
        assert!(next.member_orgs.contains(&"org3".to_string()));
        // original unchanged
        assert_eq!(cfg.member_orgs.len(), 2);
    }

    #[test]
    fn apply_remove_org() {
        let cfg = sample();
        let next = apply_config_update(&cfg, &[ConfigUpdateType::RemoveOrg("org2".to_string())])
            .expect("apply");
        assert!(!next.member_orgs.contains(&"org2".to_string()));
        assert_eq!(next.member_orgs.len(), 1);
    }

    #[test]
    fn apply_set_policy() {
        let cfg = sample();
        let new_policy = EndorsementPolicy::AnyOf(vec!["org1".to_string(), "org2".to_string()]);
        let next = apply_config_update(&cfg, &[ConfigUpdateType::SetPolicy(new_policy.clone())])
            .expect("apply");
        assert_eq!(next.endorsement_policy, new_policy);
    }

    #[test]
    fn apply_set_anchor_peer() {
        let cfg = ChannelConfig::default();
        let next = apply_config_update(
            &cfg,
            &[ConfigUpdateType::SetAnchorPeer {
                org_id: "org1".to_string(),
                peer_address: "peer0.org1:7051".to_string(),
            }],
        )
        .expect("apply");
        assert_eq!(
            next.anchor_peers.get("org1").unwrap(),
            &vec!["peer0.org1:7051".to_string()]
        );
    }

    #[test]
    fn apply_batch_size_update() {
        let cfg = sample();
        let next =
            apply_config_update(&cfg, &[ConfigUpdateType::SetBatchSize(500)]).expect("apply");
        assert_eq!(next.batch_size, 500);
    }

    #[test]
    fn apply_add_duplicate_org_returns_error() {
        let cfg = sample();
        let err =
            apply_config_update(&cfg, &[ConfigUpdateType::AddOrg("org1".to_string())]).unwrap_err();
        assert_eq!(err, ChannelError::OrgAlreadyExists("org1".to_string()));
    }

    #[test]
    fn apply_remove_nonexistent_org_returns_error() {
        let cfg = sample();
        let err = apply_config_update(&cfg, &[ConfigUpdateType::RemoveOrg("org9".to_string())])
            .unwrap_err();
        assert_eq!(err, ChannelError::OrgNotFound("org9".to_string()));
    }

    // ── validate_config_tx tests ─────────────────────────────────────────────

    fn make_keypair() -> (ed25519_dalek::SigningKey, [u8; 32]) {
        use rand::rngs::OsRng;
        let sk = ed25519_dalek::SigningKey::generate(&mut OsRng);
        let pk = sk.verifying_key().to_bytes();
        (sk, pk)
    }

    fn make_signed_endorsement(
        sk: &ed25519_dalek::SigningKey,
        payload_hash: [u8; 32],
        org_id: &str,
    ) -> Endorsement {
        use ed25519_dalek::Signer as _;
        Endorsement {
            signer_did: format!("did:bc:{org_id}:admin"),
            org_id: org_id.to_string(),
            signature: sk.sign(&payload_hash).to_bytes().to_vec(),
            payload_hash,
            timestamp: 0,
        }
    }

    fn registry_with(orgs: &[(&str, [u8; 32])]) -> crate::endorsement::registry::MemoryOrgRegistry {
        use crate::endorsement::org::Organization;
        use crate::endorsement::registry::{MemoryOrgRegistry, OrgRegistry as _};
        let reg = MemoryOrgRegistry::new();
        for (org_id, pk) in orgs {
            let org = Organization::new(
                *org_id,
                format!("{org_id}MSP"),
                vec![format!("did:bc:{org_id}:admin")],
                vec![],
                vec![*pk],
            )
            .unwrap();
            reg.register_org(&org).unwrap();
        }
        reg
    }

    fn base_config_with_policy(policy: EndorsementPolicy) -> ChannelConfig {
        ChannelConfig {
            version: 0,
            member_orgs: vec!["org1".to_string()],
            orderer_orgs: vec!["orderer".to_string()],
            endorsement_policy: policy,
            ..ChannelConfig::default()
        }
    }

    #[test]
    fn validate_config_tx_pass_with_sufficient_signatures() {
        use crate::endorsement::MemoryPolicyStore;
        let (sk, pk) = make_keypair();
        let payload = [9u8; 32];
        let reg = registry_with(&[("org1", pk)]);
        let policy_store = MemoryPolicyStore::new();
        let cfg = base_config_with_policy(EndorsementPolicy::AnyOf(vec!["org1".to_string()]));

        let tx = ConfigTransaction {
            tx_id: "tx-pass".to_string(),
            channel_id: "ch1".to_string(),
            updates: vec![ConfigUpdateType::SetBatchSize(200)],
            signatures: vec![make_signed_endorsement(&sk, payload, "org1")],
            created_at: 0,
        };

        assert!(validate_config_tx(&tx, &cfg, &policy_store, &reg).is_ok());
    }

    #[test]
    fn validate_config_tx_fail_with_insufficient_signatures() {
        use crate::endorsement::MemoryPolicyStore;
        let (_, pk) = make_keypair();
        let reg = registry_with(&[("org1", pk)]);
        let policy_store = MemoryPolicyStore::new();
        // Policy requires org1 but tx has no signatures.
        let cfg = base_config_with_policy(EndorsementPolicy::AnyOf(vec!["org1".to_string()]));

        let tx = ConfigTransaction {
            tx_id: "tx-fail".to_string(),
            channel_id: "ch1".to_string(),
            updates: vec![ConfigUpdateType::SetBatchSize(200)],
            signatures: vec![],
            created_at: 0,
        };

        assert!(validate_config_tx(&tx, &cfg, &policy_store, &reg).is_err());
    }
}
