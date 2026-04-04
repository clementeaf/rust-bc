//! Fabric Gateway — orchestrates the endorse → order → commit lifecycle.

use std::sync::Arc;

use thiserror::Error;

use crate::discovery::service::DiscoveryError;
use crate::discovery::service::DiscoveryService;
use crate::endorsement::policy_store::PolicyStore;
use crate::endorsement::policy::EndorsementPolicy;
use crate::endorsement::registry::OrgRegistry;
use crate::events::types::BlockEvent;
use crate::events::EventBus;
use crate::ordering::service::OrderingService;
use crate::storage::traits::{BlockStore, Transaction};

/// Errors produced by the gateway.
#[derive(Debug, Error)]
pub enum GatewayError {
    #[error("endorsement policy not satisfied for chaincode '{0}'")]
    PolicyNotSatisfied(String),
    #[error("ordering service error: {0}")]
    Ordering(String),
    #[error("storage error: {0}")]
    Storage(String),
}

/// Result returned after a transaction is fully committed.
#[derive(Debug, Clone, PartialEq)]
pub struct TxResult {
    /// The transaction ID that was submitted.
    pub tx_id: String,
    /// The block height at which the transaction was committed.
    pub block_height: u64,
}

/// Orchestrates the endorse → order → commit lifecycle for a single node.
pub struct Gateway {
    pub org_registry: Arc<dyn OrgRegistry>,
    pub policy_store: Arc<dyn PolicyStore>,
    pub ordering_service: Arc<OrderingService>,
    pub store: Arc<dyn BlockStore>,
    /// Optional discovery service used to resolve endorsers at submit time.
    pub discovery_service: Option<Arc<DiscoveryService>>,
    /// Optional event bus — when set, emits `BlockCommitted` and
    /// `TransactionCommitted` events after each successful commit.
    pub event_bus: Option<Arc<EventBus>>,
}

impl Gateway {
    pub fn new(
        org_registry: Arc<dyn OrgRegistry>,
        policy_store: Arc<dyn PolicyStore>,
        ordering_service: Arc<OrderingService>,
        store: Arc<dyn BlockStore>,
    ) -> Self {
        Self {
            org_registry,
            policy_store,
            ordering_service,
            store,
            discovery_service: None,
            event_bus: None,
        }
    }

    /// Like [`new`] but with a discovery service for peer-aware endorsement.
    pub fn with_discovery(
        org_registry: Arc<dyn OrgRegistry>,
        policy_store: Arc<dyn PolicyStore>,
        ordering_service: Arc<OrderingService>,
        store: Arc<dyn BlockStore>,
        discovery_service: Arc<DiscoveryService>,
    ) -> Self {
        Self {
            org_registry,
            policy_store,
            ordering_service,
            store,
            discovery_service: Some(discovery_service),
            event_bus: None,
        }
    }

    /// Like [`new`] but with an event bus for block/transaction notifications.
    pub fn with_events(
        org_registry: Arc<dyn OrgRegistry>,
        policy_store: Arc<dyn PolicyStore>,
        ordering_service: Arc<OrderingService>,
        store: Arc<dyn BlockStore>,
        event_bus: Arc<EventBus>,
    ) -> Self {
        Self {
            org_registry,
            policy_store,
            ordering_service,
            store,
            discovery_service: None,
            event_bus: Some(event_bus),
        }
    }

    /// Submit a transaction through the full endorse → order → commit pipeline.
    ///
    /// Steps (single-node implementation):
    /// 1. Resolve endorsers via the discovery service when available (channel-aware),
    ///    or fall back to a self-endorsement check using the local org registry.
    /// 2. Enqueue the transaction in the ordering service.
    /// 3. Cut a block immediately and persist it to the store.
    /// 4. Return [`TxResult`] with the transaction ID and the committed block height.
    ///
    /// When `channel_id` is non-empty and a `DiscoveryService` is configured, step 1
    /// calls `endorsement_plan` to find the required endorsers.  If the policy is not
    /// found in discovery the method falls back to the local org-registry check.
    pub fn submit(
        &self,
        chaincode_id: &str,
        channel_id: &str,
        tx: Transaction,
    ) -> Result<TxResult, GatewayError> {
        // ── Step 1: endorsement check ─────────────────────────────────────────
        if let (Some(svc), false) = (&self.discovery_service, channel_id.is_empty()) {
            // Discovery path: query endorsement_plan to find the required peers.
            match svc.endorsement_plan(chaincode_id, channel_id) {
                Ok(_endorsers) => {
                    // endorsers list is non-empty → policy satisfied via discovery.
                }
                Err(DiscoveryError::PolicyNotFound(_)) => {
                    // No policy registered in discovery; fall through to org-registry check.
                    self.self_endorse(chaincode_id)?;
                }
                Err(DiscoveryError::InsufficientPeers) => {
                    return Err(GatewayError::PolicyNotSatisfied(chaincode_id.to_string()));
                }
                Err(DiscoveryError::PeerNotFound(addr)) => {
                    return Err(GatewayError::PolicyNotSatisfied(
                        format!("peer not found: {addr}"),
                    ));
                }
            }
        } else {
            // No discovery or no channel — fall back to local org-registry check.
            self.self_endorse(chaincode_id)?;
        }

        // ── Step 2: enqueue in ordering service ───────────────────────────────
        let tx_id = tx.id.clone();
        self.ordering_service
            .submit_tx(tx)
            .map_err(|e| GatewayError::Ordering(e.to_string()))?;

        // ── Step 3: cut block and commit to store ─────────────────────────────
        let next_height = self.store.get_latest_height().unwrap_or(0) + 1;

        let block = self
            .ordering_service
            .cut_block(next_height, "gateway")
            .map_err(|e| GatewayError::Ordering(e.to_string()))?
            .ok_or_else(|| GatewayError::Ordering("cut_block returned no block".to_string()))?;

        let block_height = block.height;

        self.store
            .write_block(&block)
            .map_err(|e| GatewayError::Storage(e.to_string()))?;

        // ── Step 4: emit events ───────────────────────────────────────────────
        if let Some(ref bus) = self.event_bus {
            bus.publish(BlockEvent::BlockCommitted {
                channel_id: channel_id.to_string(),
                height: block_height,
                tx_count: block.transactions.len(),
            });
            for tx_id_in_block in &block.transactions {
                bus.publish(BlockEvent::TransactionCommitted {
                    channel_id: channel_id.to_string(),
                    tx_id: tx_id_in_block.clone(),
                    block_height,
                    valid: true,
                });
            }
        }

        // ── Step 5: return result ─────────────────────────────────────────────
        Ok(TxResult { tx_id, block_height })
    }

    /// Self-endorsement check using the local org registry.
    ///
    /// Returns `Ok(())` when the registered orgs satisfy the policy for
    /// `chaincode_id`, or when no policy is configured.
    fn self_endorse(&self, chaincode_id: &str) -> Result<(), GatewayError> {
        let policy = self.policy_store.get_policy(chaincode_id).ok();

        if let Some(ref p) = policy {
            let registered_orgs = self.org_registry.list_orgs().unwrap_or_default();
            let org_ids: Vec<&str> =
                registered_orgs.iter().map(|o| o.org_id.as_str()).collect();

            let satisfied = match p {
                EndorsementPolicy::And(_, _) | EndorsementPolicy::Or(_, _) => p.evaluate(&org_ids),
                EndorsementPolicy::AnyOf(orgs) => {
                    orgs.iter().any(|o| org_ids.contains(&o.as_str()))
                }
                EndorsementPolicy::AllOf(orgs) => {
                    orgs.iter().all(|o| org_ids.contains(&o.as_str()))
                }
                EndorsementPolicy::NOutOf { n, orgs } => {
                    orgs.iter().filter(|o| org_ids.contains(&o.as_str())).count() >= *n
                }
            };

            if !satisfied {
                return Err(GatewayError::PolicyNotSatisfied(chaincode_id.to_string()));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::endorsement::org::Organization;
    use crate::endorsement::policy::EndorsementPolicy;
    use crate::endorsement::policy_store::MemoryPolicyStore;
    use crate::endorsement::registry::MemoryOrgRegistry;
    use crate::storage::memory::MemoryStore;

    fn make_tx(id: &str) -> Transaction {
        Transaction {
            id: id.to_string(),
            block_height: 0,
            timestamp: 0,
            input_did: "did:bc:alice".to_string(),
            output_recipient: "did:bc:bob".to_string(),
            amount: 10,
            state: "pending".to_string(),
        }
    }

    fn make_org(id: &str) -> Organization {
        Organization::new(
            id,
            &format!("{id}MSP"),
            vec![format!("did:bc:{id}:admin")],
            vec![],
            vec![],
        )
        .unwrap()
    }

    fn make_gateway() -> Gateway {
        Gateway::new(
            Arc::new(MemoryOrgRegistry::new()),
            Arc::new(MemoryPolicyStore::new()),
            Arc::new(OrderingService::with_config(10, 500)),
            Arc::new(MemoryStore::new()),
        )
    }

    // ── 9.1.1 tests (kept) ────────────────────────────────────────────────────

    #[test]
    fn create_gateway_with_memory_impls() {
        let gw = make_gateway();
        let _ = gw.org_registry.list_orgs().unwrap();
        assert_eq!(gw.ordering_service.max_batch_size, 10);
    }

    #[test]
    fn gateway_store_is_empty_on_init() {
        let gw = make_gateway();
        assert!(gw.store.read_block(0).is_err());
    }

    #[test]
    fn gateway_policy_store_starts_empty() {
        let gw = make_gateway();
        assert!(gw.policy_store.get_policy("cc/missing").is_err());
    }

    // ── 9.1.2 tests ───────────────────────────────────────────────────────────

    #[test]
    fn submit_tx_no_policy_commits_block() {
        let gw = make_gateway();
        let result = gw.submit("cc-nopolicy", "", make_tx("tx-1")).unwrap();

        assert_eq!(result.tx_id, "tx-1");
        assert_eq!(result.block_height, 1);

        // Block must be persisted in the store.
        let block = gw.store.read_block(1).unwrap();
        assert!(block.transactions.contains(&"tx-1".to_string()));
    }

    #[test]
    fn submit_tx_with_any_of_policy_satisfied() {
        let registry = Arc::new(MemoryOrgRegistry::new());
        registry.register_org(&make_org("org1")).unwrap();

        let policy_store = Arc::new(MemoryPolicyStore::new());
        policy_store
            .set_policy("cc1", &EndorsementPolicy::AnyOf(vec!["org1".into()]))
            .unwrap();

        let gw = Gateway::new(
            registry,
            policy_store,
            Arc::new(OrderingService::with_config(10, 500)),
            Arc::new(MemoryStore::new()),
        );

        let result = gw.submit("cc1", "", make_tx("tx-2")).unwrap();
        assert_eq!(result.block_height, 1);
        let block = gw.store.read_block(1).unwrap();
        assert!(block.transactions.contains(&"tx-2".to_string()));
    }

    #[test]
    fn submit_tx_with_policy_not_satisfied_returns_error() {
        let gw_no_orgs = make_gateway(); // registry is empty
        gw_no_orgs
            .policy_store
            .set_policy("cc-strict", &EndorsementPolicy::AllOf(vec!["org1".into(), "org2".into()]))
            .unwrap();

        let err = gw_no_orgs.submit("cc-strict", "", make_tx("tx-3")).unwrap_err();
        assert!(matches!(err, GatewayError::PolicyNotSatisfied(_)));
    }

    #[test]
    fn multiple_submits_produce_sequential_block_heights() {
        let gw = make_gateway();

        let r1 = gw.submit("cc", "", make_tx("tx-a")).unwrap();
        let r2 = gw.submit("cc", "", make_tx("tx-b")).unwrap();

        assert_eq!(r1.block_height, 1);
        assert_eq!(r2.block_height, 2);
        assert_eq!(gw.store.get_latest_height().unwrap(), 2);
    }

    // ── 10.4.1 — Discovery-based endorsement tests ────────────────────────────

    use crate::discovery::service::DiscoveryService;
    use crate::discovery::PeerDescriptor;
    use crate::ordering::NodeRole;

    fn make_discovery_svc(
        policy_key: &str,
        policy: EndorsementPolicy,
        peers: Vec<(&str, &str)>, // (address, org)
    ) -> Arc<DiscoveryService> {
        let ps = Arc::new(MemoryPolicyStore::new());
        ps.set_policy(policy_key, &policy).unwrap();
        let svc = Arc::new(DiscoveryService::new(Arc::new(MemoryOrgRegistry::new()), ps));
        for (addr, org) in peers {
            svc.register_peer(PeerDescriptor {
                peer_address: addr.to_string(),
                org_id: org.to_string(),
                role: NodeRole::Peer,
                chaincodes: vec!["basic".to_string()],
                channels: vec!["mychannel".to_string()],
                last_heartbeat: 0,
            });
        }
        svc
    }

    #[test]
    fn submit_uses_discovery_to_endorse_and_commits_block() {
        let disc = make_discovery_svc(
            "mychannel/basic",
            EndorsementPolicy::NOutOf {
                n: 2,
                orgs: vec!["Org1MSP".into(), "Org2MSP".into(), "Org3MSP".into()],
            },
            vec![("peer1:7051", "Org1MSP"), ("peer2:7051", "Org2MSP")],
        );

        let gw = Gateway::with_discovery(
            Arc::new(MemoryOrgRegistry::new()),
            Arc::new(MemoryPolicyStore::new()),
            Arc::new(OrderingService::with_config(10, 500)),
            Arc::new(MemoryStore::new()),
            disc,
        );

        let result = gw.submit("basic", "mychannel", make_tx("tx-disc-1")).unwrap();
        assert_eq!(result.tx_id, "tx-disc-1");
        assert_eq!(result.block_height, 1);
        let block = gw.store.read_block(1).unwrap();
        assert!(block.transactions.contains(&"tx-disc-1".to_string()));
    }

    #[test]
    fn submit_discovery_insufficient_peers_returns_policy_not_satisfied() {
        // Policy requires 2 orgs but only 1 peer registered
        let disc = make_discovery_svc(
            "mychannel/basic",
            EndorsementPolicy::NOutOf {
                n: 2,
                orgs: vec!["Org1MSP".into(), "Org2MSP".into()],
            },
            vec![("peer1:7051", "Org1MSP")],
        );

        let gw = Gateway::with_discovery(
            Arc::new(MemoryOrgRegistry::new()),
            Arc::new(MemoryPolicyStore::new()),
            Arc::new(OrderingService::with_config(10, 500)),
            Arc::new(MemoryStore::new()),
            disc,
        );

        let err = gw.submit("basic", "mychannel", make_tx("tx-fail")).unwrap_err();
        assert!(matches!(err, GatewayError::PolicyNotSatisfied(_)));
    }

    #[test]
    fn submit_discovery_policy_not_found_falls_back_to_org_registry() {
        // Discovery has no policy for this chaincode → falls back to org registry
        // org registry is empty → policy_store also has no policy → should succeed (no policy = allow)
        let disc = Arc::new(DiscoveryService::new(
            Arc::new(MemoryOrgRegistry::new()),
            Arc::new(MemoryPolicyStore::new()),
        ));

        let gw = Gateway::with_discovery(
            Arc::new(MemoryOrgRegistry::new()),
            Arc::new(MemoryPolicyStore::new()),
            Arc::new(OrderingService::with_config(10, 500)),
            Arc::new(MemoryStore::new()),
            disc,
        );

        // No policy configured anywhere → should commit successfully
        let result = gw.submit("unknown-cc", "mychannel", make_tx("tx-fallback")).unwrap();
        assert_eq!(result.block_height, 1);
    }

    // ── 11.2.1 — Event emission tests ─────────────────────────────────────────

    use crate::events::EventBus;
    use crate::events::types::BlockEvent;

    fn make_gateway_with_events() -> (Gateway, Arc<EventBus>) {
        let bus = Arc::new(EventBus::new());
        let gw = Gateway::with_events(
            Arc::new(MemoryOrgRegistry::new()),
            Arc::new(MemoryPolicyStore::new()),
            Arc::new(OrderingService::with_config(10, 500)),
            Arc::new(MemoryStore::new()),
            bus.clone(),
        );
        (gw, bus)
    }

    #[test]
    fn block_with_3_txs_emits_one_block_committed_and_three_tx_committed() {
        let (gw, bus) = make_gateway_with_events();
        let mut rx = bus.subscribe();

        // Pre-enqueue 2 TXs directly so the next submit cuts a block with 3.
        gw.ordering_service.submit_tx(make_tx("tx-1")).unwrap();
        gw.ordering_service.submit_tx(make_tx("tx-2")).unwrap();
        gw.submit("cc", "", make_tx("tx-3")).unwrap();

        // Collect all 4 events (1 BlockCommitted + 3 TransactionCommitted).
        let mut events = Vec::new();
        while let Ok(e) = rx.try_recv() {
            events.push(e);
        }

        assert_eq!(events.len(), 4, "expected 4 events: 1 block + 3 tx");

        let block_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, BlockEvent::BlockCommitted { .. }))
            .collect();
        assert_eq!(block_events.len(), 1);
        assert_eq!(
            block_events[0],
            &BlockEvent::BlockCommitted { channel_id: "".to_string(), height: 1, tx_count: 3 }
        );

        let tx_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, BlockEvent::TransactionCommitted { .. }))
            .collect();
        assert_eq!(tx_events.len(), 3);

        let mut tx_ids: Vec<String> = tx_events
            .iter()
            .map(|e| match e {
                BlockEvent::TransactionCommitted { tx_id, .. } => tx_id.clone(),
                _ => unreachable!(),
            })
            .collect();
        tx_ids.sort();
        assert_eq!(tx_ids, vec!["tx-1", "tx-2", "tx-3"]);
    }

    #[test]
    fn single_tx_submit_emits_block_and_tx_events() {
        let (gw, bus) = make_gateway_with_events();
        let mut rx = bus.subscribe();

        gw.submit("cc", "", make_tx("tx-solo")).unwrap();

        let e1 = rx.try_recv().unwrap();
        let e2 = rx.try_recv().unwrap();

        assert_eq!(e1, BlockEvent::BlockCommitted { channel_id: "".to_string(), height: 1, tx_count: 1 });
        assert_eq!(
            e2,
            BlockEvent::TransactionCommitted {
                channel_id: "".to_string(),
                tx_id: "tx-solo".to_string(),
                block_height: 1,
                valid: true,
            }
        );
        assert!(rx.try_recv().is_err(), "no more events expected");
    }

    #[test]
    fn no_event_bus_submit_still_commits() {
        // Gateway without event_bus must still commit successfully.
        let gw = make_gateway();
        let result = gw.submit("cc", "", make_tx("tx-no-bus")).unwrap();
        assert_eq!(result.block_height, 1);
    }
}
