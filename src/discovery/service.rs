use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use crate::endorsement::policy::EndorsementPolicy;
use crate::endorsement::policy_store::PolicyStore;
use crate::endorsement::registry::OrgRegistry;
use crate::metrics::MetricsCollector;

use super::PeerDescriptor;

/// Errors returned by the discovery service.
#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    #[error("peer not found: {0}")]
    PeerNotFound(String),
    #[error("no policy found for {0}")]
    PolicyNotFound(String),
    #[error("cannot satisfy endorsement policy: insufficient peers")]
    InsufficientPeers,
}

/// Registry of active peers with their capabilities.
pub struct DiscoveryService {
    peers: Mutex<HashMap<String, PeerDescriptor>>,
    pub org_registry: Arc<dyn OrgRegistry>,
    pub policy_store: Arc<dyn PolicyStore>,
    metrics: Option<Arc<MetricsCollector>>,
}

impl DiscoveryService {
    pub fn new(org_registry: Arc<dyn OrgRegistry>, policy_store: Arc<dyn PolicyStore>) -> Self {
        Self {
            peers: Mutex::new(HashMap::new()),
            org_registry,
            policy_store,
            metrics: None,
        }
    }

    /// Attach a metrics collector so peer registrations update `discovery_peers_registered`.
    pub fn with_metrics(mut self, metrics: Arc<MetricsCollector>) -> Self {
        self.metrics = Some(metrics);
        self
    }

    /// Add or update a peer entry.
    pub fn register_peer(&self, desc: PeerDescriptor) {
        let count = {
            let mut map = self.peers.lock().unwrap_or_else(|e| e.into_inner());
            map.insert(desc.peer_address.clone(), desc);
            map.len()
        };
        if let Some(m) = &self.metrics {
            m.set_discovery_peers(count);
        }
    }

    /// Remove a peer entry; returns an error if the peer was not registered.
    pub fn unregister_peer(&self, address: &str) -> Result<(), DiscoveryError> {
        let count = {
            let mut map = self.peers.lock().unwrap_or_else(|e| e.into_inner());
            if map.remove(address).is_none() {
                return Err(DiscoveryError::PeerNotFound(address.to_string()));
            }
            map.len()
        };
        if let Some(m) = &self.metrics {
            m.set_discovery_peers(count);
        }
        Ok(())
    }

    /// Update the `last_heartbeat` timestamp for a registered peer.
    pub fn heartbeat(&self, address: &str, timestamp: u64) -> Result<(), DiscoveryError> {
        let mut map = self.peers.lock().unwrap_or_else(|e| e.into_inner());
        let peer = map
            .get_mut(address)
            .ok_or_else(|| DiscoveryError::PeerNotFound(address.to_string()))?;
        peer.last_heartbeat = timestamp;
        Ok(())
    }

    /// Return all peers that participate in `channel_id`.
    pub fn channel_peers(&self, channel_id: &str) -> Vec<PeerDescriptor> {
        self.peers
            .lock()
            .unwrap()
            .values()
            .filter(|p| p.channels.iter().any(|ch| ch == channel_id))
            .cloned()
            .collect()
    }

    /// Return a snapshot of all registered peers.
    pub fn all_peers(&self) -> Vec<PeerDescriptor> {
        self.peers
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .values()
            .cloned()
            .collect()
    }

    /// Return the minimum set of peers needed to satisfy the endorsement policy
    /// for `chaincode_id` on `channel_id`.
    ///
    /// Only peers that have the chaincode installed **and** participate in the
    /// channel are considered as candidates.
    pub fn endorsement_plan(
        &self,
        chaincode_id: &str,
        channel_id: &str,
    ) -> Result<Vec<PeerDescriptor>, DiscoveryError> {
        let resource_id = format!("{channel_id}/{chaincode_id}");
        let policy = self
            .policy_store
            .get_policy(&resource_id)
            .map_err(|_| DiscoveryError::PolicyNotFound(resource_id))?;

        // Peers that have the chaincode installed and are on the channel.
        let candidates: Vec<PeerDescriptor> = self
            .peers
            .lock()
            .unwrap()
            .values()
            .filter(|p| {
                p.chaincodes.iter().any(|c| c == chaincode_id)
                    && p.channels.iter().any(|ch| ch == channel_id)
            })
            .cloned()
            .collect();

        // Group one representative peer per org.
        let mut org_to_peer: HashMap<String, PeerDescriptor> = HashMap::new();
        for p in &candidates {
            org_to_peer
                .entry(p.org_id.clone())
                .or_insert_with(|| p.clone());
        }

        // Determine the orgs we need to cover to satisfy the policy.
        let required_orgs = required_orgs_for_policy(&policy, &org_to_peer)?;

        Ok(required_orgs
            .into_iter()
            .filter_map(|org| org_to_peer.get(&org).cloned())
            .collect())
    }
}

/// Returns the minimal list of org IDs (from the available peers) that
/// satisfies `policy`.  Errors with `InsufficientPeers` if not possible.
fn required_orgs_for_policy(
    policy: &EndorsementPolicy,
    available: &HashMap<String, PeerDescriptor>,
) -> Result<Vec<String>, DiscoveryError> {
    match policy {
        EndorsementPolicy::AnyOf(orgs) => {
            // One peer from any org in the list suffices.
            orgs.iter()
                .find(|o| available.contains_key(*o))
                .map(|o| vec![o.clone()])
                .ok_or(DiscoveryError::InsufficientPeers)
        }
        EndorsementPolicy::AllOf(orgs) => {
            // Need one peer per org.
            let covered: Vec<String> = orgs
                .iter()
                .filter(|o| available.contains_key(*o))
                .cloned()
                .collect();
            if covered.len() == orgs.len() {
                Ok(covered)
            } else {
                Err(DiscoveryError::InsufficientPeers)
            }
        }
        EndorsementPolicy::NOutOf { n, orgs } => {
            // Pick the first n orgs from the policy list that have candidates.
            let covered: Vec<String> = orgs
                .iter()
                .filter(|o| available.contains_key(*o))
                .take(*n)
                .cloned()
                .collect();
            if covered.len() >= *n {
                Ok(covered)
            } else {
                Err(DiscoveryError::InsufficientPeers)
            }
        }
        EndorsementPolicy::And(a, b) => {
            // Union of both sub-plans (deduplicated).
            let mut orgs_a = required_orgs_for_policy(a, available)?;
            let orgs_b = required_orgs_for_policy(b, available)?;
            let seen: HashSet<String> = orgs_a.iter().cloned().collect();
            for o in orgs_b {
                if !seen.contains(&o) {
                    orgs_a.push(o);
                }
            }
            Ok(orgs_a)
        }
        EndorsementPolicy::Or(a, b) => {
            // Prefer the smaller of the two sub-plans.
            match (
                required_orgs_for_policy(a, available),
                required_orgs_for_policy(b, available),
            ) {
                (Ok(la), Ok(lb)) => Ok(if la.len() <= lb.len() { la } else { lb }),
                (Ok(la), Err(_)) => Ok(la),
                (Err(_), Ok(lb)) => Ok(lb),
                (Err(e), Err(_)) => Err(e),
            }
        }
        EndorsementPolicy::OuBased { ou_ids, min_count } => {
            // Treat OU IDs as org IDs for discovery purposes.
            let covered: Vec<String> = ou_ids
                .iter()
                .filter(|o| available.contains_key(*o))
                .take(*min_count)
                .cloned()
                .collect();
            if covered.len() >= *min_count {
                Ok(covered)
            } else {
                Err(DiscoveryError::InsufficientPeers)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::endorsement::policy_store::MemoryPolicyStore;
    use crate::endorsement::registry::MemoryOrgRegistry;
    use crate::ordering::NodeRole;

    fn make_service() -> DiscoveryService {
        DiscoveryService::new(
            Arc::new(MemoryOrgRegistry::new()),
            Arc::new(MemoryPolicyStore::new()),
        )
    }

    fn peer(address: &str, org: &str) -> PeerDescriptor {
        PeerDescriptor {
            peer_address: address.to_string(),
            org_id: org.to_string(),
            role: NodeRole::Peer,
            chaincodes: vec!["basic".to_string()],
            channels: vec!["mychannel".to_string()],
            last_heartbeat: 1_000,
        }
    }

    #[test]
    fn register_three_peers() {
        let svc = make_service();
        svc.register_peer(peer("peer1:7051", "Org1MSP"));
        svc.register_peer(peer("peer2:7051", "Org2MSP"));
        svc.register_peer(peer("peer3:7051", "Org3MSP"));
        assert_eq!(svc.all_peers().len(), 3);
    }

    #[test]
    fn heartbeat_updates_timestamp() {
        let svc = make_service();
        svc.register_peer(peer("peer1:7051", "Org1MSP"));
        svc.heartbeat("peer1:7051", 9_999).unwrap();
        let peers = svc.all_peers();
        assert_eq!(peers[0].last_heartbeat, 9_999);
    }

    #[test]
    fn heartbeat_unknown_peer_returns_error() {
        let svc = make_service();
        let result = svc.heartbeat("ghost:7051", 1_000);
        assert!(matches!(result, Err(DiscoveryError::PeerNotFound(_))));
    }

    #[test]
    fn unregister_removes_peer() {
        let svc = make_service();
        svc.register_peer(peer("peer1:7051", "Org1MSP"));
        svc.register_peer(peer("peer2:7051", "Org2MSP"));
        svc.unregister_peer("peer1:7051").unwrap();
        let peers = svc.all_peers();
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].peer_address, "peer2:7051");
    }

    #[test]
    fn unregister_unknown_peer_returns_error() {
        let svc = make_service();
        let result = svc.unregister_peer("ghost:7051");
        assert!(matches!(result, Err(DiscoveryError::PeerNotFound(_))));
    }

    // ---- endorsement_plan tests ----

    use crate::endorsement::policy::EndorsementPolicy;

    fn make_service_with_policy(policy: EndorsementPolicy) -> DiscoveryService {
        let ps = Arc::new(MemoryPolicyStore::new());
        ps.set_policy("mychannel/basic", &policy).unwrap();
        DiscoveryService::new(Arc::new(MemoryOrgRegistry::new()), ps)
    }

    fn peer_cc(
        address: &str,
        org: &str,
        chaincodes: Vec<&str>,
        channels: Vec<&str>,
    ) -> PeerDescriptor {
        PeerDescriptor {
            peer_address: address.to_string(),
            org_id: org.to_string(),
            role: NodeRole::Peer,
            chaincodes: chaincodes.into_iter().map(String::from).collect(),
            channels: channels.into_iter().map(String::from).collect(),
            last_heartbeat: 1_000,
        }
    }

    #[test]
    fn n_out_of_2_from_3_orgs_returns_2_peers_from_distinct_orgs() {
        let policy = EndorsementPolicy::NOutOf {
            n: 2,
            orgs: vec!["Org1MSP".into(), "Org2MSP".into(), "Org3MSP".into()],
        };
        let svc = make_service_with_policy(policy);
        // 5 peers across 3 orgs, all with the chaincode on the channel
        svc.register_peer(peer_cc(
            "peer1:7051",
            "Org1MSP",
            vec!["basic"],
            vec!["mychannel"],
        ));
        svc.register_peer(peer_cc(
            "peer2:7051",
            "Org1MSP",
            vec!["basic"],
            vec!["mychannel"],
        ));
        svc.register_peer(peer_cc(
            "peer3:7051",
            "Org2MSP",
            vec!["basic"],
            vec!["mychannel"],
        ));
        svc.register_peer(peer_cc(
            "peer4:7051",
            "Org3MSP",
            vec!["basic"],
            vec!["mychannel"],
        ));
        svc.register_peer(peer_cc(
            "peer5:7051",
            "Org3MSP",
            vec!["basic"],
            vec!["mychannel"],
        ));

        let plan = svc.endorsement_plan("basic", "mychannel").unwrap();
        assert_eq!(plan.len(), 2, "should return exactly 2 peers");
        let orgs: HashSet<String> = plan.iter().map(|p| p.org_id.clone()).collect();
        assert_eq!(orgs.len(), 2, "the 2 peers must be from 2 distinct orgs");
    }

    #[test]
    fn endorsement_plan_filters_out_peers_without_chaincode() {
        let policy = EndorsementPolicy::AnyOf(vec!["Org1MSP".into()]);
        let svc = make_service_with_policy(policy);
        // peer without the chaincode
        svc.register_peer(peer_cc(
            "peer1:7051",
            "Org1MSP",
            vec!["other"],
            vec!["mychannel"],
        ));

        let result = svc.endorsement_plan("basic", "mychannel");
        assert!(matches!(result, Err(DiscoveryError::InsufficientPeers)));
    }

    #[test]
    fn endorsement_plan_filters_out_peers_on_wrong_channel() {
        let policy = EndorsementPolicy::AnyOf(vec!["Org1MSP".into()]);
        let svc = make_service_with_policy(policy);
        svc.register_peer(peer_cc(
            "peer1:7051",
            "Org1MSP",
            vec!["basic"],
            vec!["otherchannel"],
        ));

        let result = svc.endorsement_plan("basic", "mychannel");
        assert!(matches!(result, Err(DiscoveryError::InsufficientPeers)));
    }

    // ---- channel_peers tests ----

    #[test]
    fn channel_peers_returns_only_peers_on_channel() {
        let svc = make_service();
        svc.register_peer(peer_cc(
            "peer1:7051",
            "Org1MSP",
            vec!["basic"],
            vec!["mychannel"],
        ));
        svc.register_peer(peer_cc(
            "peer2:7051",
            "Org2MSP",
            vec!["basic"],
            vec!["mychannel"],
        ));
        svc.register_peer(peer_cc(
            "peer3:7051",
            "Org3MSP",
            vec!["basic"],
            vec!["mychannel"],
        ));
        svc.register_peer(peer_cc(
            "peer4:7051",
            "Org1MSP",
            vec!["basic"],
            vec!["otherchannel"],
        ));
        svc.register_peer(peer_cc(
            "peer5:7051",
            "Org2MSP",
            vec!["basic"],
            vec!["otherchannel"],
        ));

        let result = svc.channel_peers("mychannel");
        assert_eq!(result.len(), 3);
        assert!(result
            .iter()
            .all(|p| p.channels.contains(&"mychannel".to_string())));
    }

    #[test]
    fn channel_peers_empty_when_no_match() {
        let svc = make_service();
        svc.register_peer(peer_cc(
            "peer1:7051",
            "Org1MSP",
            vec!["basic"],
            vec!["otherchannel"],
        ));
        assert!(svc.channel_peers("mychannel").is_empty());
    }

    // ---- endorsement_plan tests ----

    #[test]
    fn endorsement_plan_missing_policy_returns_error() {
        let svc = make_service();
        svc.register_peer(peer("peer1:7051", "Org1MSP"));
        let result = svc.endorsement_plan("basic", "mychannel");
        assert!(matches!(result, Err(DiscoveryError::PolicyNotFound(_))));
    }
}
