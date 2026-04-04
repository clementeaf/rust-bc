//! Fabric Gateway — orchestrates the endorse → order → commit lifecycle.

use std::sync::Arc;

use thiserror::Error;

use crate::endorsement::policy_store::PolicyStore;
use crate::endorsement::policy::EndorsementPolicy;
use crate::endorsement::registry::OrgRegistry;
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
        }
    }

    /// Submit a transaction through the full endorse → order → commit pipeline.
    ///
    /// Steps (single-node implementation):
    /// 1. Look up the endorsement policy for `chaincode_id`; collect required org IDs.
    /// 2. Verify the local node can satisfy the policy (self-endorse): at least one
    ///    registered org must be in the required set, or no policy is configured.
    /// 3. Enqueue the transaction in the ordering service.
    /// 4. Cut a block immediately and persist it to the store.
    /// 5. Return [`TxResult`] with the transaction ID and the committed block height.
    pub fn submit(
        &self,
        chaincode_id: &str,
        tx: Transaction,
    ) -> Result<TxResult, GatewayError> {
        // ── Step 1: resolve endorsement policy ────────────────────────────────
        let policy = self.policy_store.get_policy(chaincode_id).ok();

        // ── Step 2: single-node self-endorsement check ────────────────────────
        if let Some(ref p) = policy {
            let registered_orgs = self
                .org_registry
                .list_orgs()
                .unwrap_or_default();
            let org_ids: Vec<&str> = registered_orgs.iter().map(|o| o.org_id.as_str()).collect();

            let satisfied = match p {
                // For Or/And composites we fall back to a full evaluate() call.
                EndorsementPolicy::And(_, _) | EndorsementPolicy::Or(_, _) => {
                    p.evaluate(&org_ids)
                }
                // For flat policies we just check the org set intersection.
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

        // ── Step 3: enqueue in ordering service ───────────────────────────────
        let tx_id = tx.id.clone();
        self.ordering_service
            .submit_tx(tx)
            .map_err(|e| GatewayError::Ordering(e.to_string()))?;

        // ── Step 4: cut block and commit to store ─────────────────────────────
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

        // ── Step 5: return result ─────────────────────────────────────────────
        Ok(TxResult { tx_id, block_height })
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
        let result = gw.submit("cc-nopolicy", make_tx("tx-1")).unwrap();

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

        let result = gw.submit("cc1", make_tx("tx-2")).unwrap();
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

        let err = gw_no_orgs.submit("cc-strict", make_tx("tx-3")).unwrap_err();
        assert!(matches!(err, GatewayError::PolicyNotSatisfied(_)));
    }

    #[test]
    fn multiple_submits_produce_sequential_block_heights() {
        let gw = make_gateway();

        let r1 = gw.submit("cc", make_tx("tx-a")).unwrap();
        let r2 = gw.submit("cc", make_tx("tx-b")).unwrap();

        assert_eq!(r1.block_height, 1);
        assert_eq!(r2.block_height, 2);
        assert_eq!(gw.store.get_latest_height().unwrap(), 2);
    }
}
