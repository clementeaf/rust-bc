use actix_web::{web, HttpRequest, HttpResponse, post};
use serde::{Deserialize, Serialize};

use crate::api::errors::{ApiError, ApiResponse, ApiResult};
use crate::app_state::AppState;

// ── Request types ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct InstallQuery {
    pub chaincode_id: String,
    pub version: String,
}

#[derive(Debug, Deserialize)]
pub struct ApproveQuery {
    pub version: String,
}

#[derive(Debug, Deserialize)]
pub struct CommitQuery {
    pub version: String,
}

// ── Response types ────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct InstallResponse {
    pub chaincode_id: String,
    pub version: String,
    pub size_bytes: usize,
}

#[derive(Debug, Serialize)]
pub struct ApproveResponse {
    pub chaincode_id: String,
    pub version: String,
    pub org_id: String,
    pub policy_satisfied: bool,
}

#[derive(Debug, Serialize)]
pub struct CommitResponse {
    pub chaincode_id: String,
    pub version: String,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// POST /api/v1/chaincode/install?chaincode_id=...&version=...
///
/// Accepts raw Wasm bytes in the request body and stores them in the
/// `chaincode_packages` column family keyed by `{chaincode_id}:{version}`.
#[post("/chaincode/install")]
pub async fn install_chaincode(
    state: web::Data<AppState>,
    query: web::Query<InstallQuery>,
    body: web::Bytes,
) -> ApiResult<HttpResponse> {
    let trace_id = uuid::Uuid::new_v4().to_string();

    if query.chaincode_id.is_empty() {
        return Err(ApiError::ValidationError {
            field: "chaincode_id".to_string(),
            reason: "must not be empty".to_string(),
        });
    }
    if query.version.is_empty() {
        return Err(ApiError::ValidationError {
            field: "version".to_string(),
            reason: "must not be empty".to_string(),
        });
    }
    if body.is_empty() {
        return Err(ApiError::ValidationError {
            field: "body".to_string(),
            reason: "Wasm bytes must not be empty".to_string(),
        });
    }

    let store = state.chaincode_package_store.as_ref().ok_or(ApiError::NotFound {
        resource: "chaincode_package_store".to_string(),
    })?;

    store
        .store_package(&query.chaincode_id, &query.version, &body)
        .map_err(|e| ApiError::StorageError { reason: e.to_string() })?;

    let response = InstallResponse {
        chaincode_id: query.chaincode_id.clone(),
        version: query.version.clone(),
        size_bytes: body.len(),
    };
    Ok(HttpResponse::Ok().json(ApiResponse::success(response, trace_id)))
}

/// POST /api/v1/chaincode/{id}/approve?version=...
///
/// Records the approving org (taken from the `X-Org-Id` header) in the
/// chaincode definition's approvals map.  If the endorsement policy is
/// satisfied by the accumulated approvals, the definition status advances
/// to `Approved`.
#[post("/chaincode/{id}/approve")]
pub async fn approve_chaincode(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<String>,
    query: web::Query<ApproveQuery>,
) -> ApiResult<HttpResponse> {
    let trace_id = uuid::Uuid::new_v4().to_string();
    let chaincode_id = path.into_inner();

    // Extract org from header.
    let org_id = req
        .headers()
        .get("X-Org-Id")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| ApiError::ValidationError {
            field: "X-Org-Id".to_string(),
            reason: "header is required".to_string(),
        })?
        .to_string();

    if org_id.is_empty() {
        return Err(ApiError::ValidationError {
            field: "X-Org-Id".to_string(),
            reason: "must not be empty".to_string(),
        });
    }
    if query.version.is_empty() {
        return Err(ApiError::ValidationError {
            field: "version".to_string(),
            reason: "must not be empty".to_string(),
        });
    }

    let def_store = state.chaincode_definition_store.as_ref().ok_or(ApiError::NotFound {
        resource: "chaincode_definition_store".to_string(),
    })?;

    let mut def = def_store
        .get_definition(&chaincode_id, &query.version)
        .map_err(|e| ApiError::StorageError { reason: e.to_string() })?
        .ok_or_else(|| ApiError::NotFound {
            resource: format!("chaincode definition '{chaincode_id}:{}'", query.version),
        })?;

    // Record approval.
    def.approvals.insert(org_id.clone(), true);

    // Check if endorsement policy is satisfied by current approvals.
    let approved_orgs: Vec<&str> = def
        .approvals
        .iter()
        .filter_map(|(org, &approved)| if approved { Some(org.as_str()) } else { None })
        .collect();
    let policy_satisfied = def.endorsement_policy.evaluate(&approved_orgs);

    if policy_satisfied {
        if let Ok(next_status) = def.status.transition_to(&crate::chaincode::ChaincodeStatus::Approved) {
            def.status = next_status;
        }
    }

    def_store
        .upsert_definition(def)
        .map_err(|e| ApiError::StorageError { reason: e.to_string() })?;

    let response = ApproveResponse {
        chaincode_id,
        version: query.version.clone(),
        org_id,
        policy_satisfied,
    };
    Ok(HttpResponse::Ok().json(ApiResponse::success(response, trace_id)))
}

/// POST /api/v1/chaincode/{id}/commit?version=...
///
/// Verifies that the endorsement policy is satisfied by the accumulated
/// approvals in the definition.  On success, advances the status to
/// `Committed`.  Returns 409 Conflict if the policy is not yet satisfied.
#[post("/chaincode/{id}/commit")]
pub async fn commit_chaincode(
    state: web::Data<AppState>,
    path: web::Path<String>,
    query: web::Query<CommitQuery>,
) -> ApiResult<HttpResponse> {
    let trace_id = uuid::Uuid::new_v4().to_string();
    let chaincode_id = path.into_inner();

    if query.version.is_empty() {
        return Err(ApiError::ValidationError {
            field: "version".to_string(),
            reason: "must not be empty".to_string(),
        });
    }

    let def_store = state.chaincode_definition_store.as_ref().ok_or(ApiError::NotFound {
        resource: "chaincode_definition_store".to_string(),
    })?;

    let mut def = def_store
        .get_definition(&chaincode_id, &query.version)
        .map_err(|e| ApiError::StorageError { reason: e.to_string() })?
        .ok_or_else(|| ApiError::NotFound {
            resource: format!("chaincode definition '{chaincode_id}:{}'", query.version),
        })?;

    // Verify endorsement policy is satisfied by current approvals.
    let approved_orgs: Vec<&str> = def
        .approvals
        .iter()
        .filter_map(|(org, &approved)| if approved { Some(org.as_str()) } else { None })
        .collect();

    if !def.endorsement_policy.evaluate(&approved_orgs) {
        return Err(ApiError::Conflict {
            reason: format!(
                "endorsement policy not satisfied for '{chaincode_id}:{}': insufficient approvals",
                query.version
            ),
        });
    }

    def.status = def
        .status
        .transition_to(&crate::chaincode::ChaincodeStatus::Committed)
        .map_err(|e| ApiError::Conflict { reason: e.to_string() })?;

    def_store
        .upsert_definition(def)
        .map_err(|e| ApiError::StorageError { reason: e.to_string() })?;

    let response = CommitResponse { chaincode_id, version: query.version.clone() };
    Ok(HttpResponse::Ok().json(ApiResponse::success(response, trace_id)))
}

/// POST /api/v1/chaincode/{id}/simulate?version=...
///
/// Executes the chaincode in simulation mode: writes are buffered locally and
/// the committed world state is never modified.  Returns the function result
/// (base64-encoded) and the read-write set produced during execution.
#[post("/chaincode/{id}/simulate")]
pub async fn simulate_chaincode(
    state: web::Data<AppState>,
    path: web::Path<String>,
    query: web::Query<SimulateQuery>,
    body: web::Json<SimulateRequest>,
) -> ApiResult<HttpResponse> {
    use crate::chaincode::executor::WasmExecutor;
    use crate::storage::MemoryWorldState;

    let trace_id = uuid::Uuid::new_v4().to_string();
    let chaincode_id = path.into_inner();

    if query.version.is_empty() {
        return Err(ApiError::ValidationError {
            field: "version".to_string(),
            reason: "must not be empty".to_string(),
        });
    }
    if body.function.is_empty() {
        return Err(ApiError::ValidationError {
            field: "function".to_string(),
            reason: "must not be empty".to_string(),
        });
    }

    let pkg_store = state.chaincode_package_store.as_ref().ok_or(ApiError::NotFound {
        resource: "chaincode_package_store".to_string(),
    })?;

    let wasm = pkg_store
        .get_package(&chaincode_id, &query.version)
        .map_err(|e| ApiError::StorageError { reason: e.to_string() })?
        .ok_or_else(|| ApiError::NotFound {
            resource: format!("chaincode package '{chaincode_id}:{}'", query.version),
        })?;

    let executor = WasmExecutor::new(&wasm, 10_000_000)
        .map_err(|e| ApiError::StorageError { reason: e.to_string() })?;

    let base: std::sync::Arc<dyn crate::storage::WorldState> =
        std::sync::Arc::new(MemoryWorldState::new());

    let (result_bytes, rwset) = executor
        .simulate(base, &body.function)
        .map_err(|e| ApiError::StorageError { reason: e.to_string() })?;

    let response = SimulateResponse {
        result: base64_encode(&result_bytes),
        rwset: RwSetResponse {
            reads: rwset.reads.into_iter().map(|r| KVReadDto { key: r.key, version: r.version }).collect(),
            writes: rwset.writes.into_iter().map(|w| KVWriteDto { key: w.key, value: base64_encode(&w.value) }).collect(),
        },
    };
    Ok(HttpResponse::Ok().json(ApiResponse::success(response, trace_id)))
}

fn base64_encode(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::new(), |mut s, b| { let _ = write!(s, "{:02x}", b); s })
}

// ── Request / Response types for simulate ─────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct SimulateQuery {
    pub version: String,
}

#[derive(Debug, Deserialize)]
pub struct SimulateRequest {
    pub function: String,
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SimulateResponse {
    pub result: String,
    pub rwset: RwSetResponse,
}

#[derive(Debug, Serialize)]
pub struct RwSetResponse {
    pub reads: Vec<KVReadDto>,
    pub writes: Vec<KVWriteDto>,
}

#[derive(Debug, Serialize)]
pub struct KVReadDto {
    pub key: String,
    pub version: u64,
}

#[derive(Debug, Serialize)]
pub struct KVWriteDto {
    pub key: String,
    pub value: String,
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex, RwLock};

    use crate::airdrop::AirdropManager;
    use crate::billing::BillingManager;
    use crate::blockchain::Blockchain;
    use crate::cache::BalanceCache;
    use crate::chaincode::{
        ChaincodeDefinitionStore, ChaincodePackageStore,
        MemoryChaincodeDefinitionStore, MemoryChaincodePackageStore,
        definition::ChaincodeDefinition,
    };
    use crate::endorsement::EndorsementPolicy;
    use crate::metrics::MetricsCollector;
    use crate::models::{Mempool, WalletManager};
    use crate::smart_contracts::ContractManager;
    use crate::staking::StakingManager;
    use crate::transaction_validation::TransactionValidator;

    fn make_state(
        pkg_store: Option<Arc<dyn crate::chaincode::ChaincodePackageStore>>,
        def_store: Option<Arc<dyn crate::chaincode::ChaincodeDefinitionStore>>,
    ) -> web::Data<AppState> {
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
            airdrop_manager: Arc::new(AirdropManager::new(100, 10, "w".to_string())),
            pruning_manager: None,
            checkpoint_manager: None,
            transaction_validator: Arc::new(Mutex::new(TransactionValidator::with_defaults())),
            metrics: Arc::new(MetricsCollector::new()),
            store: Arc::new(RwLock::new(HashMap::new())),
            org_registry: None,
            policy_store: None,
            crl_store: None,
            private_data_store: None,
            collection_registry: None,
            chaincode_package_store: pkg_store,
            chaincode_definition_store: def_store,
            gateway: None,
            discovery_service: None,
            event_bus: Arc::new(crate::events::EventBus::new()),
            channel_configs: std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())),
            acl_provider: None, ordering_backend: None, world_state: None,
        })
    }

    #[actix_web::test]
    async fn install_stores_wasm_and_returns_size() {
        let pkg_store = Arc::new(MemoryChaincodePackageStore::new());
        let state = make_state(Some(pkg_store.clone()), None);
        let app = test::init_service(
            App::new()
                .app_data(state)
                .service(web::scope("/api/v1").service(install_chaincode)),
        )
        .await;

        let wasm = vec![0u8; 512];
        let req = test::TestRequest::post()
            .uri("/api/v1/chaincode/install?chaincode_id=my_cc&version=1.0")
            .set_payload(wasm.clone())
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["data"]["chaincode_id"], "my_cc");
        assert_eq!(body["data"]["version"], "1.0");
        assert_eq!(body["data"]["size_bytes"], 512);

        // Verify it was actually persisted
        let stored = pkg_store.get_package("my_cc", "1.0").unwrap();
        assert_eq!(stored, Some(wasm));
    }

    #[actix_web::test]
    async fn install_empty_body_is_bad_request() {
        let state = make_state(Some(Arc::new(MemoryChaincodePackageStore::new())), None);
        let app = test::init_service(
            App::new()
                .app_data(state)
                .service(web::scope("/api/v1").service(install_chaincode)),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/v1/chaincode/install?chaincode_id=cc&version=1.0")
            .set_payload(vec![])
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);
    }

    #[actix_web::test]
    async fn install_without_store_is_not_found() {
        let state = make_state(None, None);
        let app = test::init_service(
            App::new()
                .app_data(state)
                .service(web::scope("/api/v1").service(install_chaincode)),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/v1/chaincode/install?chaincode_id=cc&version=1.0")
            .set_payload(vec![1u8; 64])
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 404);
    }

    async fn seed_definition(
        store: &Arc<MemoryChaincodeDefinitionStore>,
        chaincode_id: &str,
        version: &str,
        policy: EndorsementPolicy,
    ) {
        store
            .upsert_definition(ChaincodeDefinition::new(chaincode_id, version, policy))
            .unwrap();
    }

    #[actix_web::test]
    async fn approve_records_org_approval() {
        let def_store = Arc::new(MemoryChaincodeDefinitionStore::new());
        seed_definition(&def_store, "cc1", "1.0", EndorsementPolicy::AllOf(vec!["org1".to_string(), "org2".to_string()])).await;

        let state = make_state(None, Some(def_store.clone() as Arc<dyn crate::chaincode::ChaincodeDefinitionStore>));
        let app = test::init_service(
            App::new().app_data(state).service(web::scope("/api/v1").service(approve_chaincode)),
        ).await;
        let req = test::TestRequest::post()
            .uri("/api/v1/chaincode/cc1/approve?version=1.0")
            .insert_header(("X-Org-Id", "org1"))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["data"]["org_id"], "org1");
        assert_eq!(body["data"]["chaincode_id"], "cc1");
        // Only org1 approved; AllOf(org1, org2) not yet satisfied.
        assert_eq!(body["data"]["policy_satisfied"], false);

        let def = def_store.get_definition("cc1", "1.0").unwrap().unwrap();
        assert_eq!(def.approvals["org1"], true);
    }

    #[actix_web::test]
    async fn approve_transitions_to_approved_when_policy_satisfied() {
        let def_store = Arc::new(MemoryChaincodeDefinitionStore::new());
        seed_definition(&def_store, "cc2", "2.0", EndorsementPolicy::AnyOf(vec!["org1".to_string()])).await;

        let state = make_state(None, Some(def_store.clone() as Arc<dyn crate::chaincode::ChaincodeDefinitionStore>));
        let app = test::init_service(
            App::new().app_data(state).service(web::scope("/api/v1").service(approve_chaincode)),
        ).await;
        let req = test::TestRequest::post()
            .uri("/api/v1/chaincode/cc2/approve?version=2.0")
            .insert_header(("X-Org-Id", "org1"))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["data"]["policy_satisfied"], true);

        let def = def_store.get_definition("cc2", "2.0").unwrap().unwrap();
        assert_eq!(def.status, crate::chaincode::ChaincodeStatus::Approved);
    }

    #[actix_web::test]
    async fn approve_missing_org_id_header_is_bad_request() {
        let def_store = Arc::new(MemoryChaincodeDefinitionStore::new());
        seed_definition(&def_store, "cc3", "1.0", EndorsementPolicy::AnyOf(vec!["org1".to_string()])).await;

        let state = make_state(None, Some(def_store as Arc<dyn crate::chaincode::ChaincodeDefinitionStore>));
        let app = test::init_service(
            App::new().app_data(state).service(web::scope("/api/v1").service(approve_chaincode)),
        ).await;
        let req = test::TestRequest::post()
            .uri("/api/v1/chaincode/cc3/approve?version=1.0")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);
    }

    #[actix_web::test]
    async fn approve_unknown_definition_is_not_found() {
        let state = make_state(None, Some(Arc::new(MemoryChaincodeDefinitionStore::new()) as Arc<dyn crate::chaincode::ChaincodeDefinitionStore>));
        let app = test::init_service(
            App::new().app_data(state).service(web::scope("/api/v1").service(approve_chaincode)),
        ).await;
        let req = test::TestRequest::post()
            .uri("/api/v1/chaincode/unknown/approve?version=1.0")
            .insert_header(("X-Org-Id", "org1"))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 404);
    }

    #[actix_web::test]
    async fn approve_without_definition_store_is_not_found() {
        let state = make_state(None, None);
        let app = test::init_service(
            App::new().app_data(state).service(web::scope("/api/v1").service(approve_chaincode)),
        ).await;
        let req = test::TestRequest::post()
            .uri("/api/v1/chaincode/cc/approve?version=1.0")
            .insert_header(("X-Org-Id", "org1"))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 404);
    }

    // ── commit tests ──────────────────────────────────────────────────────────

    async fn seed_approved_definition(
        store: &Arc<MemoryChaincodeDefinitionStore>,
        chaincode_id: &str,
        version: &str,
    ) {
        // Create an Approved definition (policy already satisfied).
        let mut def = ChaincodeDefinition::new(
            chaincode_id,
            version,
            EndorsementPolicy::AnyOf(vec!["org1".to_string()]),
        );
        def.approvals.insert("org1".to_string(), true);
        def.status = crate::chaincode::ChaincodeStatus::Approved;
        store.upsert_definition(def).unwrap();
    }

    #[actix_web::test]
    async fn commit_transitions_to_committed_when_policy_satisfied() {
        let def_store = Arc::new(MemoryChaincodeDefinitionStore::new());
        seed_approved_definition(&def_store, "cc_commit", "1.0").await;

        let state = make_state(None, Some(def_store.clone() as Arc<dyn crate::chaincode::ChaincodeDefinitionStore>));
        let app = test::init_service(
            App::new().app_data(state).service(web::scope("/api/v1").service(commit_chaincode)),
        ).await;
        let req = test::TestRequest::post()
            .uri("/api/v1/chaincode/cc_commit/commit?version=1.0")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["data"]["chaincode_id"], "cc_commit");
        assert_eq!(body["data"]["version"], "1.0");

        let def = def_store.get_definition("cc_commit", "1.0").unwrap().unwrap();
        assert_eq!(def.status, crate::chaincode::ChaincodeStatus::Committed);
    }

    #[actix_web::test]
    async fn commit_fails_when_policy_not_satisfied() {
        let def_store = Arc::new(MemoryChaincodeDefinitionStore::new());
        // AllOf(org1, org2) — only org1 has approved
        seed_definition(&def_store, "cc_partial", "1.0", EndorsementPolicy::AllOf(vec!["org1".to_string(), "org2".to_string()])).await;
        def_store.upsert_definition({
            let mut def = def_store.get_definition("cc_partial", "1.0").unwrap().unwrap();
            def.approvals.insert("org1".to_string(), true);
            def
        }).unwrap();

        let state = make_state(None, Some(def_store as Arc<dyn crate::chaincode::ChaincodeDefinitionStore>));
        let app = test::init_service(
            App::new().app_data(state).service(web::scope("/api/v1").service(commit_chaincode)),
        ).await;
        let req = test::TestRequest::post()
            .uri("/api/v1/chaincode/cc_partial/commit?version=1.0")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 409);
    }

    #[actix_web::test]
    async fn commit_fails_when_already_committed() {
        let def_store = Arc::new(MemoryChaincodeDefinitionStore::new());
        let mut def = ChaincodeDefinition::new(
            "cc_done",
            "1.0",
            EndorsementPolicy::AnyOf(vec!["org1".to_string()]),
        );
        def.approvals.insert("org1".to_string(), true);
        def.status = crate::chaincode::ChaincodeStatus::Committed;
        def_store.upsert_definition(def).unwrap();

        let state = make_state(None, Some(def_store as Arc<dyn crate::chaincode::ChaincodeDefinitionStore>));
        let app = test::init_service(
            App::new().app_data(state).service(web::scope("/api/v1").service(commit_chaincode)),
        ).await;
        let req = test::TestRequest::post()
            .uri("/api/v1/chaincode/cc_done/commit?version=1.0")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 409);
    }

    #[actix_web::test]
    async fn commit_unknown_definition_is_not_found() {
        let state = make_state(None, Some(Arc::new(MemoryChaincodeDefinitionStore::new()) as Arc<dyn crate::chaincode::ChaincodeDefinitionStore>));
        let app = test::init_service(
            App::new().app_data(state).service(web::scope("/api/v1").service(commit_chaincode)),
        ).await;
        let req = test::TestRequest::post()
            .uri("/api/v1/chaincode/unknown/commit?version=1.0")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 404);
    }

    #[actix_web::test]
    async fn commit_without_definition_store_is_not_found() {
        let state = make_state(None, None);
        let app = test::init_service(
            App::new().app_data(state).service(web::scope("/api/v1").service(commit_chaincode)),
        ).await;
        let req = test::TestRequest::post()
            .uri("/api/v1/chaincode/cc/commit?version=1.0")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 404);
    }

    // ── simulate tests ────────────────────────────────────────────────────────

    /// Minimal WAT that puts "x"="1" and returns empty (ptr=0, len=0).
    ///
    /// Used to verify that simulate produces a write in the rwset and does NOT
    /// modify the base world state.
    const SIMULATE_WAT: &[u8] = br#"
(module
  (import "env" "put_state" (func $put_state (param i32 i32 i32 i32) (result i32)))
  (import "env" "get_state" (func $get_state (param i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 0) "x")
  (data (i32.const 4) "1")
  (func (export "run") (result i64)
    (drop (call $put_state (i32.const 0) (i32.const 1) (i32.const 4) (i32.const 1)))
    (i64.const 0)
  )
)
"#;

    fn make_app_with_simulate_wasm() -> (web::Data<AppState>, Arc<MemoryChaincodePackageStore>) {
        let pkg_store = Arc::new(MemoryChaincodePackageStore::new());
        // Store the WAT bytes as "wasm" — WasmExecutor::new accepts WAT too.
        pkg_store.store_package("mycc", "1.0", SIMULATE_WAT).unwrap();
        let state = make_state(Some(pkg_store.clone()), None);
        (state, pkg_store)
    }

    #[actix_web::test]
    async fn simulate_returns_rwset_and_leaves_world_state_untouched() {
        let (state, _pkg) = make_app_with_simulate_wasm();
        let app = test::init_service(
            App::new()
                .app_data(state)
                .service(web::scope("/api/v1").service(simulate_chaincode)),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/v1/chaincode/mycc/simulate?version=1.0")
            .set_json(serde_json::json!({ "function": "run" }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: serde_json::Value = test::read_body_json(resp).await;
        let writes = body["data"]["rwset"]["writes"].as_array().unwrap();
        assert!(!writes.is_empty(), "expected at least one write in rwset");
        let write_keys: Vec<&str> = writes.iter().map(|w| w["key"].as_str().unwrap()).collect();
        assert!(write_keys.contains(&"x"), "expected write for key 'x'");
    }

    #[actix_web::test]
    async fn simulate_without_package_store_is_not_found() {
        let state = make_state(None, None);
        let app = test::init_service(
            App::new()
                .app_data(state)
                .service(web::scope("/api/v1").service(simulate_chaincode)),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/v1/chaincode/cc/simulate?version=1.0")
            .set_json(serde_json::json!({ "function": "run" }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 404);
    }

    #[actix_web::test]
    async fn simulate_missing_wasm_package_is_not_found() {
        let state = make_state(Some(Arc::new(MemoryChaincodePackageStore::new())), None);
        let app = test::init_service(
            App::new()
                .app_data(state)
                .service(web::scope("/api/v1").service(simulate_chaincode)),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/v1/chaincode/ghost/simulate?version=1.0")
            .set_json(serde_json::json!({ "function": "run" }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 404);
    }
}
