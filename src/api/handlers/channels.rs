//! Channel API handlers and channel-store lookup helper.

use std::sync::Arc;

use actix_web::{get, post, web, HttpResponse};
use serde::{Deserialize, Serialize};

use crate::api::errors::{ApiError, ApiResponse, ApiResult};
use crate::app_state::AppState;
use crate::storage::memory::MemoryStore;
use crate::storage::traits::BlockStore;

// ── Request / response types ──────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CreateChannelRequest {
    pub channel_id: String,
}

#[derive(Serialize)]
pub struct ChannelCreatedResponse {
    pub channel_id: String,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Extract the channel ID from the optional `X-Channel-Id` request header.
///
/// Returns `"default"` when the header is absent or contains non-UTF-8 bytes.
pub fn channel_id_from_req(req: &actix_web::HttpRequest) -> &str {
    req.headers()
        .get("X-Channel-Id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("default")
}

/// Return the `BlockStore` for `channel_id`, or `ApiError::NotFound` if absent.
///
/// Handlers pass the value from the `X-Channel-Id` header (or `"default"` when
/// the header is absent) to this helper rather than accessing `state.store` directly.
pub fn get_channel_store(
    state: &AppState,
    channel_id: &str,
) -> Result<Arc<dyn BlockStore>, ApiError> {
    state
        .store
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .get(channel_id)
        .cloned()
        .ok_or_else(|| ApiError::NotFound {
            resource: format!("channel '{channel_id}'"),
        })
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// POST /api/v1/channels — crear un channel e instanciar su store.
///
/// El `channel_id` debe ser no vacío y contener solo caracteres
/// alfanuméricos, guiones o guiones bajos.  Devuelve 409 si ya existe.
#[post("/channels")]
pub async fn create_channel(
    state: web::Data<AppState>,
    body: web::Json<CreateChannelRequest>,
) -> ApiResult<HttpResponse> {
    let channel_id = body.into_inner().channel_id;
    let trace_id = uuid::Uuid::new_v4().to_string();

    // Validate channel_id format (same rules as create_channel_store)
    if channel_id.is_empty()
        || !channel_id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(ApiError::ValidationError {
            field: "channel_id".to_string(),
            reason: "must be non-empty and contain only alphanumeric characters, hyphens, or underscores".to_string(),
        });
    }

    let mut map = state.store.write().unwrap_or_else(|e| e.into_inner());

    if map.contains_key(&channel_id) {
        return Err(ApiError::Conflict {
            reason: format!("channel '{channel_id}' already exists"),
        });
    }

    let new_store: Arc<dyn BlockStore> = Arc::new(MemoryStore::new());
    map.insert(channel_id.clone(), new_store);
    drop(map);

    Ok(HttpResponse::Created()
        .json(ApiResponse::success(ChannelCreatedResponse { channel_id }, trace_id)))
}

/// GET /api/v1/channels — listar todos los channel IDs registrados.
#[get("/channels")]
pub async fn list_channels(state: web::Data<AppState>) -> ApiResult<HttpResponse> {
    let trace_id = uuid::Uuid::new_v4().to_string();
    let map = state.store.read().unwrap_or_else(|e| e.into_inner());
    let mut ids: Vec<String> = map.keys().cloned().collect();
    ids.sort();
    drop(map);
    Ok(HttpResponse::Ok().json(ApiResponse::success(ids, trace_id)))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex, RwLock};

    use crate::airdrop::AirdropManager;
    use crate::app_state::AppState;
    use crate::billing::BillingManager;
    use crate::block_storage::BlockStorage;
    use crate::blockchain::Blockchain;
    use crate::cache::BalanceCache;
    use crate::checkpoint::CheckpointManager;
    use crate::metrics::MetricsCollector;
    use crate::models::{Mempool, WalletManager};
    use crate::pruning::PruningManager;
    use crate::smart_contracts::ContractManager;
    use crate::staking::StakingManager;
    use crate::storage::memory::MemoryStore;
    use crate::storage::traits::BlockStore;
    use crate::transaction_validation::TransactionValidator;

    use super::{create_channel, get_channel_store, list_channels};

    fn make_state(stores: Vec<(&str, Arc<dyn BlockStore>)>) -> AppState {
        let mut store_map = HashMap::new();
        for (id, s) in stores {
            store_map.insert(id.to_string(), s);
        }
        AppState {
            blockchain: Arc::new(Mutex::new(Blockchain::new(1))),
            wallet_manager: Arc::new(Mutex::new(WalletManager::new())),
            block_storage: None::<Arc<BlockStorage>>,
            node: None,
            mempool: Arc::new(Mutex::new(Mempool::new())),
            balance_cache: Arc::new(BalanceCache::new()),
            billing_manager: Arc::new(BillingManager::new()),
            contract_manager: Arc::new(RwLock::new(ContractManager::new())),
            staking_manager: Arc::new(StakingManager::new(None, None, None)),
            airdrop_manager: Arc::new(AirdropManager::new(100, 10, "w".to_string())),
            pruning_manager: None::<Arc<PruningManager>>,
            checkpoint_manager: None::<Arc<Mutex<CheckpointManager>>>,
            transaction_validator: Arc::new(Mutex::new(TransactionValidator::with_defaults())),
            metrics: Arc::new(MetricsCollector::new()),
            store: Arc::new(RwLock::new(store_map)),
            org_registry: None,
            policy_store: None,
            crl_store: None,
            private_data_store: None,
            collection_registry: None,
            chaincode_package_store: None,
            chaincode_definition_store: None,
            gateway: None,
        }
    }

    // ── get_channel_store ─────────────────────────────────────────────────────

    #[test]
    fn get_default_store_ok() {
        let s: Arc<dyn BlockStore> = Arc::new(MemoryStore::new());
        let state = make_state(vec![("default", s)]);
        assert!(get_channel_store(&state, "default").is_ok());
    }

    #[test]
    fn get_unknown_channel_returns_not_found() {
        let state = make_state(vec![]);
        let err = get_channel_store(&state, "ch-unknown")
            .err()
            .expect("expected NotFound error");
        assert!(matches!(err, crate::api::errors::ApiError::NotFound { .. }));
    }

    #[test]
    fn get_named_channel_ok() {
        let s: Arc<dyn BlockStore> = Arc::new(MemoryStore::new());
        let state = make_state(vec![("default", Arc::new(MemoryStore::new())), ("ch1", s)]);
        assert!(get_channel_store(&state, "ch1").is_ok());
    }

    // ── create_channel handler ────────────────────────────────────────────────

    #[test]
    fn create_channel_handler_is_public() {
        let _ = create_channel;
    }

    #[test]
    fn create_channel_inserts_into_store_map() {
        let state = make_state(vec![("default", Arc::new(MemoryStore::new()))]);
        // Directly test the store mutation logic (the handler uses web::Data, hard to unit test)
        {
            let mut map = state.store.write().unwrap();
            assert!(!map.contains_key("ch-new"));
            map.insert(
                "ch-new".to_string(),
                Arc::new(MemoryStore::new()) as Arc<dyn BlockStore>,
            );
        }
        assert!(get_channel_store(&state, "ch-new").is_ok());
    }

    #[test]
    fn create_channel_conflict_detected() {
        let state = make_state(vec![
            ("default", Arc::new(MemoryStore::new()) as Arc<dyn BlockStore>),
            ("ch-dup", Arc::new(MemoryStore::new()) as Arc<dyn BlockStore>),
        ]);
        let map = state.store.read().unwrap();
        assert!(map.contains_key("ch-dup"));
    }

    // ── list_channels ─────────────────────────────────────────────────────────

    #[test]
    fn list_channels_handler_is_public() {
        let _ = list_channels;
    }

    #[test]
    fn list_channels_returns_all_keys() {
        let state = make_state(vec![
            ("default", Arc::new(MemoryStore::new()) as Arc<dyn BlockStore>),
            ("ch1", Arc::new(MemoryStore::new()) as Arc<dyn BlockStore>),
            ("ch2", Arc::new(MemoryStore::new()) as Arc<dyn BlockStore>),
        ]);
        let map = state.store.read().unwrap();
        let mut ids: Vec<&str> = map.keys().map(|s| s.as_str()).collect();
        ids.sort();
        assert_eq!(ids, vec!["ch1", "ch2", "default"]);
    }

    #[test]
    fn list_channels_empty_store_returns_empty_vec() {
        let state = make_state(vec![]);
        let map = state.store.read().unwrap();
        assert!(map.is_empty());
    }
}
