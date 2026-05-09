//! Auto-discovery — gossip-based peer exchange.
//!
//! Periodically asks connected peers for their peer lists and registers
//! any new peers with the `DiscoveryService`. This allows nodes to find
//! each other without manual registration.
//!
//! Activate by setting `AUTO_DISCOVERY=true` (default: false).
//! `AUTO_DISCOVERY_INTERVAL_SECS` controls the polling interval (default: 30).

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use super::service::DiscoveryService;
use super::PeerDescriptor;

/// Peer exchange request/response — a list of known peers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerExchange {
    pub peers: Vec<PeerInfo>,
}

/// Minimal peer info for exchange (no credentials, just address + org).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct PeerInfo {
    pub address: String,
    pub org_id: String,
    pub chaincodes: Vec<String>,
}

impl From<&PeerDescriptor> for PeerInfo {
    fn from(desc: &PeerDescriptor) -> Self {
        Self {
            address: desc.peer_address.clone(),
            org_id: desc.org_id.clone(),
            chaincodes: desc.chaincodes.clone(),
        }
    }
}

/// Configuration for auto-discovery.
#[derive(Debug, Clone)]
pub struct AutoDiscoveryConfig {
    pub enabled: bool,
    pub interval: Duration,
}

impl AutoDiscoveryConfig {
    pub fn from_env() -> Self {
        let enabled = std::env::var("AUTO_DISCOVERY")
            .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
            .unwrap_or(false);
        let interval_secs: u64 = std::env::var("AUTO_DISCOVERY_INTERVAL_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(30);
        Self {
            enabled,
            interval: Duration::from_secs(interval_secs),
        }
    }
}

/// In-memory tracker of discovered peers to avoid redundant registrations.
pub struct AutoDiscovery {
    known_addresses: std::sync::Mutex<HashSet<String>>,
}

impl AutoDiscovery {
    pub fn new() -> Self {
        Self {
            known_addresses: std::sync::Mutex::new(HashSet::new()),
        }
    }

    /// Process a peer exchange from a connected peer. Returns newly discovered addresses.
    pub fn process_exchange(
        &self,
        exchange: &PeerExchange,
        discovery: &DiscoveryService,
    ) -> Vec<String> {
        let mut known = self.known_addresses.lock().unwrap();
        let mut new_peers = Vec::new();

        for peer in &exchange.peers {
            if known.contains(&peer.address) {
                continue;
            }

            debug!(address = %peer.address, org = %peer.org_id, "Discovered new peer via gossip");

            discovery.register_peer(PeerDescriptor {
                peer_address: peer.address.clone(),
                org_id: peer.org_id.clone(),
                role: crate::ordering::NodeRole::Peer,
                chaincodes: peer.chaincodes.clone(),
                channels: Vec::new(),
                last_heartbeat: 0,
            });

            known.insert(peer.address.clone());
            new_peers.push(peer.address.clone());
        }

        if !new_peers.is_empty() {
            info!(
                count = new_peers.len(),
                "Registered new peers from gossip exchange"
            );
        }

        new_peers
    }

    /// Build a peer exchange message from the current discovery service state.
    pub fn build_exchange(&self, discovery: &DiscoveryService) -> PeerExchange {
        let peers = discovery.all_peers();
        PeerExchange {
            peers: peers.iter().map(PeerInfo::from).collect(),
        }
    }

    /// Number of known peers.
    pub fn known_count(&self) -> usize {
        self.known_addresses.lock().unwrap().len()
    }
}

impl Default for AutoDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

/// Spawn background gossip exchange loop.
pub fn spawn_auto_discovery(
    config: AutoDiscoveryConfig,
    discovery: Arc<DiscoveryService>,
    auto: Arc<AutoDiscovery>,
) {
    if !config.enabled {
        return;
    }

    tokio::spawn(async move {
        info!(
            interval_secs = config.interval.as_secs(),
            "Auto-discovery gossip started"
        );

        loop {
            tokio::time::sleep(config.interval).await;

            let exchange = auto.build_exchange(&discovery);
            let peer_count = exchange.peers.len();
            debug!(peers = peer_count, "Broadcasting peer exchange");

            // In a full implementation, this would send the exchange to all
            // connected peers via P2P and process their responses.
            // For now, the exchange is built and ready — the P2P layer calls
            // `auto.process_exchange()` when it receives a PeerExchange message.
        }
    });
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::endorsement::policy::EndorsementPolicy;
    use crate::endorsement::registry::MemoryOrgRegistry;
    use crate::endorsement::MemoryPolicyStore;

    fn make_discovery() -> DiscoveryService {
        let org_reg = Arc::new(MemoryOrgRegistry::new());
        let policy_store = Arc::new(MemoryPolicyStore::new());
        DiscoveryService::new(org_reg, policy_store)
    }

    fn make_descriptor(addr: &str, org: &str) -> PeerDescriptor {
        PeerDescriptor {
            peer_address: addr.to_string(),
            org_id: org.to_string(),
            role: crate::ordering::NodeRole::Peer,
            chaincodes: vec!["basic".to_string()],
            channels: vec!["default".to_string()],
            last_heartbeat: 0,
        }
    }

    #[test]
    fn process_exchange_registers_new_peers() {
        let discovery = make_discovery();
        let auto = AutoDiscovery::new();

        let exchange = PeerExchange {
            peers: vec![
                PeerInfo {
                    address: "peer1:8081".into(),
                    org_id: "org1".into(),
                    chaincodes: vec![],
                },
                PeerInfo {
                    address: "peer2:8081".into(),
                    org_id: "org2".into(),
                    chaincodes: vec![],
                },
            ],
        };

        let new = auto.process_exchange(&exchange, &discovery);
        assert_eq!(new.len(), 2);
        assert_eq!(auto.known_count(), 2);
    }

    #[test]
    fn process_exchange_skips_known_peers() {
        let discovery = make_discovery();
        let auto = AutoDiscovery::new();

        let exchange = PeerExchange {
            peers: vec![PeerInfo {
                address: "peer1:8081".into(),
                org_id: "org1".into(),
                chaincodes: vec![],
            }],
        };

        auto.process_exchange(&exchange, &discovery);
        let new = auto.process_exchange(&exchange, &discovery);
        assert!(new.is_empty()); // Already known
    }

    #[test]
    fn build_exchange_includes_registered_peers() {
        let discovery = make_discovery();
        discovery.register_peer(make_descriptor("peer1:8081", "org1"));
        discovery.register_peer(make_descriptor("peer2:8081", "org2"));

        let auto = AutoDiscovery::new();
        let exchange = auto.build_exchange(&discovery);
        assert_eq!(exchange.peers.len(), 2);
    }

    #[test]
    fn build_exchange_empty_when_no_peers() {
        let discovery = make_discovery();
        let auto = AutoDiscovery::new();
        let exchange = auto.build_exchange(&discovery);
        assert!(exchange.peers.is_empty());
    }

    #[test]
    fn config_defaults() {
        std::env::remove_var("AUTO_DISCOVERY");
        let config = AutoDiscoveryConfig::from_env();
        assert!(!config.enabled);
        assert_eq!(config.interval, Duration::from_secs(30));
    }

    #[test]
    fn peer_info_from_descriptor() {
        let desc = make_descriptor("peer1:8081", "org1");
        let info = PeerInfo::from(&desc);
        assert_eq!(info.address, "peer1:8081");
        assert_eq!(info.org_id, "org1");
    }

    #[test]
    fn peer_exchange_serde_roundtrip() {
        let exchange = PeerExchange {
            peers: vec![PeerInfo {
                address: "peer1:8081".into(),
                org_id: "org1".into(),
                chaincodes: vec!["basic".into()],
            }],
        };
        let json = serde_json::to_string(&exchange).unwrap();
        let restored: PeerExchange = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.peers.len(), 1);
        assert_eq!(restored.peers[0].address, "peer1:8081");
    }
}
