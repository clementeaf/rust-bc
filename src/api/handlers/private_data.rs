use actix_web::{web, HttpRequest, HttpResponse, get, put};
use serde::{Deserialize, Serialize};

use crate::api::errors::{ApiError, ApiResponse, ApiResult};
use crate::app_state::AppState;

// ── Request / response types ──────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct PutPrivateDataBody {
    pub value: String,
}

#[derive(Debug, Serialize)]
pub struct PutPrivateDataResponse {
    pub collection: String,
    pub key: String,
    pub hash: String,
}

#[derive(Debug, Serialize)]
pub struct GetPrivateDataResponse {
    pub collection: String,
    pub key: String,
    pub value: String,
}

// ── Helper: extract and validate org membership ───────────────────────────────

/// Reads `X-Org-Id` from the request headers and verifies the org is a member
/// of the named collection.  Returns the org_id on success or an `ApiError` on
/// failure.
fn check_membership(
    req: &HttpRequest,
    state: &AppState,
    collection_name: &str,
) -> ApiResult<String> {
    let org_id = req
        .headers()
        .get("X-Org-Id")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| ApiError::ValidationError {
            field: "X-Org-Id".to_string(),
            reason: "header is required".to_string(),
        })?
        .to_string();

    let registry = state.collection_registry.as_ref().ok_or(ApiError::NotFound {
        resource: "collection_registry".to_string(),
    })?;

    let collection = registry.get(collection_name).ok_or_else(|| ApiError::NotFound {
        resource: format!("collection '{collection_name}'"),
    })?;

    if !collection.is_member(&org_id) {
        return Err(ApiError::Forbidden {
            reason: format!("org '{org_id}' is not a member of collection '{collection_name}'"),
        });
    }

    Ok(org_id)
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// PUT /api/v1/private-data/{collection}/{key}
///
/// Store private data for the given collection + key.  The caller must supply
/// `X-Org-Id` matching one of the collection's `member_org_ids`; otherwise the
/// request is rejected with HTTP 403.
///
/// Body: `{ "value": "<base64 or plain string>" }`
/// Returns the SHA-256 hash of the stored bytes so it can be embedded on-chain.
#[put("/private-data/{collection}/{key}")]
pub async fn put_private_data(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
    body: web::Json<PutPrivateDataBody>,
) -> ApiResult<HttpResponse> {
    let (collection, key) = path.into_inner();
    let trace_id = uuid::Uuid::new_v4().to_string();

    check_membership(&req, &state, &collection)?;

    let store = state.private_data_store.as_ref().ok_or(ApiError::NotFound {
        resource: "private_data_store".to_string(),
    })?;

    let hash = store
        .put_private_data(&collection, &key, body.value.as_bytes())
        .map_err(|e| ApiError::StorageError { reason: e.to_string() })?;

    let response = PutPrivateDataResponse {
        collection,
        key,
        hash: hex::encode(hash),
    };
    Ok(HttpResponse::Ok().json(ApiResponse::success(response, trace_id)))
}

/// GET /api/v1/private-data/{collection}/{key}
///
/// Retrieve private data for the given collection + key.  The caller must supply
/// `X-Org-Id` matching one of the collection's `member_org_ids`; otherwise the
/// request is rejected with HTTP 403.
#[get("/private-data/{collection}/{key}")]
pub async fn get_private_data(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
) -> ApiResult<HttpResponse> {
    let (collection, key) = path.into_inner();
    let trace_id = uuid::Uuid::new_v4().to_string();

    check_membership(&req, &state, &collection)?;

    let store = state.private_data_store.as_ref().ok_or(ApiError::NotFound {
        resource: "private_data_store".to_string(),
    })?;

    let bytes = store
        .get_private_data(&collection, &key)
        .map_err(|e| ApiError::StorageError { reason: e.to_string() })?
        .ok_or_else(|| ApiError::NotFound { resource: format!("private-data/{collection}/{key}") })?;

    let value = String::from_utf8_lossy(&bytes).into_owned();
    let response = GetPrivateDataResponse { collection, key, value };
    Ok(HttpResponse::Ok().json(ApiResponse::success(response, trace_id)))
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
    use crate::metrics::MetricsCollector;
    use crate::models::{Mempool, WalletManager};
    use crate::private_data::{
        CollectionRegistry, MemoryCollectionRegistry, MemoryPrivateDataStore,
        PrivateDataCollection, PrivateDataStore,
    };
    use crate::smart_contracts::ContractManager;
    use crate::staking::StakingManager;
    use crate::transaction_validation::TransactionValidator;

    fn make_collection(name: &str, members: &[&str]) -> PrivateDataCollection {
        PrivateDataCollection::new(
            name,
            members.iter().map(|s| s.to_string()).collect(),
            1,
            100,
        )
        .unwrap()
    }

    fn make_state(
        registry: Arc<dyn CollectionRegistry>,
        store: Arc<dyn PrivateDataStore>,
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
            private_data_store: Some(store),
            collection_registry: Some(registry),
            chaincode_package_store: None,
            chaincode_definition_store: None,
        })
    }

    fn setup() -> web::Data<AppState> {
        let registry = Arc::new(MemoryCollectionRegistry::new());
        registry.register(make_collection("col1", &["org1", "org2"])).unwrap();
        let store: Arc<dyn PrivateDataStore> = Arc::new(MemoryPrivateDataStore::new());
        make_state(registry, store)
    }

    // ── PUT tests ─────────────────────────────────────────────────────────────

    #[actix_web::test]
    async fn member_can_put_private_data() {
        let state = setup();
        let app = test::init_service(
            App::new()
                .app_data(state)
                .service(web::scope("/api/v1").service(put_private_data)),
        )
        .await;

        let req = test::TestRequest::put()
            .uri("/api/v1/private-data/col1/k1")
            .insert_header(("X-Org-Id", "org1"))
            .set_json(PutPrivateDataBody { value: "secret".to_string() })
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["data"]["collection"], "col1");
        assert_eq!(body["data"]["key"], "k1");
        assert!(body["data"]["hash"].as_str().unwrap().len() == 64); // SHA-256 hex
    }

    #[actix_web::test]
    async fn non_member_put_is_forbidden() {
        let state = setup();
        let app = test::init_service(
            App::new()
                .app_data(state)
                .service(web::scope("/api/v1").service(put_private_data)),
        )
        .await;

        let req = test::TestRequest::put()
            .uri("/api/v1/private-data/col1/k1")
            .insert_header(("X-Org-Id", "org_outside"))
            .set_json(PutPrivateDataBody { value: "secret".to_string() })
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 403);
    }

    #[actix_web::test]
    async fn put_without_org_id_header_is_bad_request() {
        let state = setup();
        let app = test::init_service(
            App::new()
                .app_data(state)
                .service(web::scope("/api/v1").service(put_private_data)),
        )
        .await;

        let req = test::TestRequest::put()
            .uri("/api/v1/private-data/col1/k1")
            .set_json(PutPrivateDataBody { value: "secret".to_string() })
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);
    }

    // ── GET tests ─────────────────────────────────────────────────────────────

    #[actix_web::test]
    async fn member_can_get_private_data() {
        let registry = Arc::new(MemoryCollectionRegistry::new());
        registry.register(make_collection("col1", &["org1"])).unwrap();
        let store = Arc::new(MemoryPrivateDataStore::new());
        store.put_private_data("col1", "k1", b"payload").unwrap();
        let state = make_state(registry, store as Arc<dyn PrivateDataStore>);

        let app = test::init_service(
            App::new()
                .app_data(state)
                .service(web::scope("/api/v1").service(get_private_data)),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/v1/private-data/col1/k1")
            .insert_header(("X-Org-Id", "org1"))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["data"]["value"], "payload");
    }

    #[actix_web::test]
    async fn non_member_get_is_forbidden() {
        let state = setup();
        let app = test::init_service(
            App::new()
                .app_data(state)
                .service(web::scope("/api/v1").service(get_private_data)),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/v1/private-data/col1/k1")
            .insert_header(("X-Org-Id", "intruder"))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 403);
    }

    #[actix_web::test]
    async fn get_missing_key_is_not_found() {
        let state = setup();
        let app = test::init_service(
            App::new()
                .app_data(state)
                .service(web::scope("/api/v1").service(get_private_data)),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/v1/private-data/col1/no_such_key")
            .insert_header(("X-Org-Id", "org1"))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 404);
    }
}
