use actix_web::{get, post, web, HttpRequest, HttpResponse};
use serde::{Deserialize, Serialize};

use crate::api::errors::{enforce_acl, ApiError, ApiResponse, ApiResult};
use crate::app_state::AppState;
use crate::msp::CrlStore;

#[derive(Debug, Serialize, Deserialize)]
pub struct RevokeBody {
    pub serial: String,
}

#[derive(Debug, Serialize)]
pub struct MspInfo {
    pub msp_id: String,
    pub crl_size: usize,
}

/// POST /api/v1/msp/{msp_id}/revoke — add a serial to the MSP's CRL
#[post("/msp/{msp_id}/revoke")]
pub async fn revoke_serial(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<RevokeBody>,
) -> ApiResult<HttpResponse> {
    enforce_acl(
        state.acl_provider.as_deref(),
        state.policy_store.as_deref(),
        "peer/MSP.Admin",
        &req,
    )?;
    let msp_id = path.into_inner();
    let trace_id = uuid::Uuid::new_v4().to_string();
    let store = state.crl_store.as_ref().ok_or(ApiError::NotFound {
        resource: "crl_store".to_string(),
    })?;

    let mut serials = store
        .read_crl(&msp_id)
        .map_err(|e| ApiError::StorageError {
            reason: e.to_string(),
        })?;

    if !serials.contains(&body.serial) {
        serials.push(body.serial.clone());
        store
            .write_crl(&msp_id, &serials)
            .map_err(|e| ApiError::StorageError {
                reason: e.to_string(),
            })?;
    }

    Ok(HttpResponse::Ok().json(ApiResponse::success(
        serde_json::json!({ "msp_id": msp_id, "revoked": body.serial }),
        trace_id,
    )))
}

/// GET /api/v1/msp/{msp_id} — return MSP info (CRL size)
#[get("/msp/{msp_id}")]
pub async fn get_msp_info(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> ApiResult<HttpResponse> {
    let msp_id = path.into_inner();
    let trace_id = uuid::Uuid::new_v4().to_string();
    let store = state.crl_store.as_ref().ok_or(ApiError::NotFound {
        resource: "crl_store".to_string(),
    })?;

    let serials = store
        .read_crl(&msp_id)
        .map_err(|e| ApiError::StorageError {
            reason: e.to_string(),
        })?;

    let info = MspInfo {
        msp_id,
        crl_size: serials.len(),
    };
    Ok(HttpResponse::Ok().json(ApiResponse::success(info, trace_id)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex, RwLock};

    use crate::airdrop::AirdropManager;
    use crate::app_state::AppState;
    use crate::billing::BillingManager;
    use crate::blockchain::Blockchain;
    use crate::cache::BalanceCache;
    use crate::metrics::MetricsCollector;
    use crate::models::{Mempool, WalletManager};
    use crate::smart_contracts::ContractManager;
    use crate::staking::StakingManager;
    use crate::storage::errors::StorageResult;
    use crate::transaction_validation::TransactionValidator;

    struct MemCrl(Mutex<HashMap<String, Vec<String>>>);
    impl CrlStore for MemCrl {
        fn write_crl(&self, msp_id: &str, serials: &[String]) -> StorageResult<()> {
            self.0
                .lock()
                .unwrap()
                .insert(msp_id.to_string(), serials.to_vec());
            Ok(())
        }
        fn read_crl(&self, msp_id: &str) -> StorageResult<Vec<String>> {
            Ok(self
                .0
                .lock()
                .unwrap()
                .get(msp_id)
                .cloned()
                .unwrap_or_default())
        }
    }

    fn make_state(crl: Arc<dyn CrlStore>) -> web::Data<AppState> {
        std::env::set_var("ACL_MODE", "permissive");
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
            crl_store: Some(crl),
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
        })
    }

    #[actix_web::test]
    async fn revoke_adds_serial() {
        let crl: Arc<dyn CrlStore> = Arc::new(MemCrl(Mutex::new(HashMap::new())));
        let state = make_state(crl.clone());
        let app = test::init_service(
            App::new()
                .app_data(state)
                .service(web::scope("/api/v1").service(revoke_serial)),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/v1/msp/Org1MSP/revoke")
            .set_json(RevokeBody {
                serial: "abc123".to_string(),
            })
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let loaded = crl.read_crl("Org1MSP").unwrap();
        assert!(loaded.contains(&"abc123".to_string()));
    }

    #[actix_web::test]
    async fn get_msp_info_returns_crl_size() {
        let crl: Arc<dyn CrlStore> = Arc::new(MemCrl(Mutex::new(HashMap::new())));
        crl.write_crl("Org2MSP", &["s1".to_string(), "s2".to_string()])
            .unwrap();
        let state = make_state(crl);
        let app = test::init_service(
            App::new()
                .app_data(state)
                .service(web::scope("/api/v1").service(get_msp_info)),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/v1/msp/Org2MSP")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["data"]["crl_size"], 2);
    }
}
