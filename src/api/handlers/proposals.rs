//! POST /api/v1/proposals and POST /api/v1/transactions/submit

use actix_web::{post, web, HttpRequest, HttpResponse};

use crate::api::errors::{ApiError, ApiResponse, ApiResult};
use crate::api::handlers::channels::channel_id_from_req;
use crate::app_state::AppState;
use crate::endorsement::validator::validate_endorsements;
use crate::transaction::endorsed::EndorsedTransaction;
use crate::transaction::proposal::{ProposalResponse, TransactionProposal};
use crate::transaction::rwset::{KVRead, ReadWriteSet};

/// POST /api/v1/proposals — simulate a proposal and return a ProposalResponse
/// endorsed by the local peer (org_id = "local").
///
/// Simulation: for each key in the proposal rwset, add a KVRead to the
/// response rwset; forward all writes unchanged.
#[post("/proposals")]
pub async fn submit_proposal(
    state: web::Data<AppState>,
    body: web::Json<TransactionProposal>,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    let trace_id = uuid::Uuid::new_v4().to_string();
    let channel = channel_id_from_req(&req);
    let proposal = body.into_inner();

    // Simulate: build a response rwset from the proposal rwset.
    // Reads carry version = 0 (no world-state yet); writes are passed through.
    let simulated_reads: Vec<KVRead> = proposal
        .rwset
        .reads
        .iter()
        .map(|r| KVRead {
            key: r.key.clone(),
            version: r.version,
        })
        .collect();

    let response_rwset = ReadWriteSet {
        reads: simulated_reads,
        writes: proposal.rwset.writes.clone(),
    };

    // Hash the proposal tx id as the payload (placeholder — no real key material here).
    let payload_hash = {
        let mut buf = [0u8; 32];
        let id_bytes = proposal.tx.id.as_bytes();
        let len = id_bytes.len().min(32);
        buf[..len].copy_from_slice(&id_bytes[..len]);
        buf
    };

    let endorsement = crate::endorsement::types::Endorsement {
        signer_did: "did:local:peer".to_string(),
        org_id: "local".to_string(),
        signature: vec![0u8; 64],
        signature_algorithm: Default::default(),
        payload_hash,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    };

    let response = ProposalResponse {
        rwset: response_rwset,
        endorsement,
    };

    // Persist the original tx to the channel's store if available.
    if let Some(store) = state
        .store
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .get(channel)
        .cloned()
    {
        let _ = store.write_transaction(&proposal.tx);
    }

    Ok(HttpResponse::Ok().json(ApiResponse::success(response, trace_id)))
}

/// POST /api/v1/transactions/submit — validate endorsements against policy,
/// then forward the transaction to the ordering service.
#[post("/transactions/submit")]
pub async fn submit_endorsed_transaction(
    state: web::Data<AppState>,
    body: web::Json<EndorsedTransaction>,
) -> ApiResult<HttpResponse> {
    let trace_id = uuid::Uuid::new_v4().to_string();
    let endorsed = body.into_inner();

    // Validate endorsements against policy if both registry and policy_store are present.
    if let (Some(registry), Some(policy_store)) =
        (state.org_registry.as_ref(), state.policy_store.as_ref())
    {
        let tx_id = &endorsed.proposal.tx.id;
        if let Ok(policy) = policy_store.get_policy(tx_id) {
            validate_endorsements(&endorsed.endorsements, &policy, registry.as_ref(), None)
                .map_err(|e| ApiError::ValidationError {
                    field: "endorsements".to_string(),
                    reason: e.to_string(),
                })?;
        } // No policy registered for this tx — accept as-is.
    }

    // Forward to ordering service via node if available.
    if let Some(node) = &state.node {
        if let Some(ordering) = &node.ordering_service {
            ordering
                .submit_tx(endorsed.proposal.tx.clone())
                .map_err(|e| ApiError::InternalError {
                    reason: e.to_string(),
                })?;
        }
    }

    Ok(HttpResponse::Accepted().json(ApiResponse::success(
        serde_json::json!({ "tx_id": endorsed.proposal.tx.id, "status": "submitted" }),
        trace_id,
    )))
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex, RwLock};

    use actix_web::{test, web, App};

    use crate::airdrop::AirdropManager;
    use crate::app_state::AppState;
    use crate::billing::BillingManager;
    use crate::blockchain::Blockchain;
    use crate::cache::BalanceCache;
    use crate::endorsement::types::Endorsement;
    use crate::endorsement::{MemoryOrgRegistry, MemoryPolicyStore};
    use crate::metrics::MetricsCollector;
    use crate::models::{Mempool, WalletManager};
    use crate::smart_contracts::ContractManager;
    use crate::staking::StakingManager;
    use crate::storage::{traits::Transaction, MemoryStore};
    use crate::transaction::endorsed::EndorsedTransaction;
    use crate::transaction::proposal::TransactionProposal;
    use crate::transaction::rwset::{KVRead, KVWrite, ReadWriteSet};
    use crate::transaction_validation::{TransactionValidator, ValidationConfig};

    fn make_state() -> web::Data<AppState> {
        web::Data::new(AppState {
            blockchain: Arc::new(Mutex::new(Blockchain::new(1))),
            wallet_manager: Arc::new(Mutex::new(WalletManager::new())),
            block_storage: None,
            node: None,
            mempool: Arc::new(Mutex::new(Mempool::new())),
            balance_cache: Arc::new(BalanceCache::new()),
            billing_manager: Arc::new(BillingManager::new()),
            contract_manager: Arc::new(RwLock::new(ContractManager::new())),
            staking_manager: Arc::new(StakingManager::new(None, None, None)),
            airdrop_manager: Arc::new(AirdropManager::new(1000, 10, "airdrop".to_string())),
            pruning_manager: None,
            checkpoint_manager: None,
            transaction_validator: Arc::new(Mutex::new(TransactionValidator::new(
                ValidationConfig::default(),
            ))),
            metrics: Arc::new(MetricsCollector::new()),
            store: {
                let mut m = std::collections::HashMap::new();
                m.insert(
                    "default".to_string(),
                    Arc::new(MemoryStore::new()) as Arc<dyn crate::storage::traits::BlockStore>,
                );
                std::sync::Arc::new(std::sync::RwLock::new(m))
            },
            org_registry: Some(Arc::new(MemoryOrgRegistry::new())),
            policy_store: Some(Arc::new(MemoryPolicyStore::new())),
            crl_store: None,
            private_data_store: None,
            collection_registry: None,
            chaincode_package_store: None,
            chaincode_definition_store: None,
            gateway: None,
            discovery_service: None,
            event_bus: Arc::new(crate::events::EventBus::new()),
            channel_configs: std::sync::Arc::new(std::sync::RwLock::new(
                std::collections::HashMap::new(),
            )),
            acl_provider: None,
            ordering_backend: None,
            world_state: None,
            audit_store: None,
            proposal_store: None,
            vote_store: None,
            param_registry: None,
            pin_store: None,
            account_store: None,
            native_mempool: None,
            economics_state: std::sync::Arc::new(std::sync::Mutex::new(
                crate::tokenomics::economics::EconomicsState::default(),
            )),
            faucet: None,
        })
    }

    fn sample_tx(id: &str) -> Transaction {
        Transaction {
            id: id.to_string(),
            block_height: 0,
            timestamp: 0,
            input_did: "did:example:alice".to_string(),
            output_recipient: "did:example:bob".to_string(),
            amount: 0,
            state: "pending".to_string(),
        }
    }

    fn sample_rwset() -> ReadWriteSet {
        ReadWriteSet {
            reads: vec![KVRead {
                key: "k".to_string(),
                version: 1,
            }],
            writes: vec![KVWrite {
                key: "k".to_string(),
                value: vec![1],
            }],
        }
    }

    #[actix_web::test]
    async fn post_proposals_returns_proposal_response() {
        let state = make_state();
        let app = test::init_service(
            App::new()
                .app_data(state)
                .service(web::scope("/api/v1").service(super::submit_proposal)),
        )
        .await;

        let proposal = TransactionProposal {
            tx: sample_tx("tx-test-1"),
            creator_did: "did:example:alice".to_string(),
            creator_signature: vec![0u8; 64],
            signature_algorithm: Default::default(),
            rwset: sample_rwset(),
        };

        let req = test::TestRequest::post()
            .uri("/api/v1/proposals")
            .set_json(&proposal)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
    }

    #[actix_web::test]
    async fn post_transactions_submit_accepted_without_policy() {
        let state = make_state();
        let app = test::init_service(
            App::new()
                .app_data(state)
                .service(web::scope("/api/v1").service(super::submit_endorsed_transaction)),
        )
        .await;

        let endorsed = EndorsedTransaction {
            proposal: TransactionProposal {
                tx: sample_tx("tx-test-2"),
                creator_did: "did:example:alice".to_string(),
                creator_signature: vec![0u8; 64],
                signature_algorithm: Default::default(),
                rwset: sample_rwset(),
            },
            endorsements: vec![Endorsement {
                signer_did: "did:example:org1".to_string(),
                org_id: "Org1".to_string(),
                signature: vec![0u8; 64],
                signature_algorithm: Default::default(),
                payload_hash: [0u8; 32],
                timestamp: 0,
            }],
            rwset: sample_rwset(),
        };

        let req = test::TestRequest::post()
            .uri("/api/v1/transactions/submit")
            .set_json(&endorsed)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 202);
    }
}
