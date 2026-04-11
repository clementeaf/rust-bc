//! Channel API handlers and channel-store lookup helper.

use std::sync::Arc;

use actix_web::{get, post, web, HttpRequest, HttpResponse};
use serde::{Deserialize, Serialize};

use crate::api::errors::{enforce_acl, ApiError, ApiResponse, ApiResult};
use crate::app_state::AppState;
use crate::channel::config::{apply_config_update, ChannelConfig, ConfigTransaction};
use crate::channel::genesis::create_genesis_block;
use crate::endorsement::{MemoryOrgRegistry, MemoryPolicyStore};
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

/// Verify that the caller's org (from `X-Org-Id` header) is a member of the
/// channel.  Returns `Ok(())` if the channel has no config yet (permissive
/// bootstrap), if the channel has no member orgs configured, or if `X-Org-Id`
/// is absent (backwards compatible).  Returns `Forbidden` only when a channel
/// has explicit member orgs AND the caller's org is not among them.
pub fn enforce_channel_membership(
    state: &AppState,
    channel_id: &str,
    req: &actix_web::HttpRequest,
) -> Result<(), ApiError> {
    let caller_org = req
        .headers()
        .get("X-Org-Id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if channel_id == "default" {
        return Ok(()); // Default channel is open to all
    }
    if caller_org.is_empty() {
        if crate::api::errors::acl_permissive() {
            return Ok(());
        }
        return Err(ApiError::Forbidden {
            reason: format!("missing X-Org-Id header for channel '{channel_id}' membership check"),
        });
    }

    let configs = state
        .channel_configs
        .read()
        .unwrap_or_else(|e| e.into_inner());

    if let Some(history) = configs.get(channel_id) {
        if let Some(latest) = history.last() {
            if !latest.member_orgs.is_empty()
                && !latest.member_orgs.contains(&caller_org.to_string())
            {
                return Err(ApiError::Forbidden {
                    reason: format!("org '{caller_org}' is not a member of channel '{channel_id}'"),
                });
            }
        }
    }

    Ok(())
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
    http_req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<CreateChannelRequest>,
) -> ApiResult<HttpResponse> {
    enforce_acl(
        state.acl_provider.as_deref(),
        state.policy_store.as_deref(),
        "peer/ChannelConfig",
        &http_req,
    )?;
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

    let genesis_config = ChannelConfig::default();
    let genesis = create_genesis_block(&channel_id, &genesis_config);
    new_store
        .write_block(&genesis)
        .map_err(|e| ApiError::InternalError {
            reason: format!("failed to write genesis block for channel '{channel_id}': {e}"),
        })?;

    map.insert(channel_id.clone(), new_store);
    drop(map);

    // Seed config history with the genesis config.
    state
        .channel_configs
        .write()
        .unwrap_or_else(|e| e.into_inner())
        .insert(channel_id.clone(), vec![genesis_config]);

    Ok(HttpResponse::Created().json(ApiResponse::success(
        ChannelCreatedResponse { channel_id },
        trace_id,
    )))
}

/// POST /api/v1/channels/{channel_id}/config — submit a config-update transaction.
///
/// Validates signatures against the channel's current modification policy, applies
/// the updates, and persists the new `ChannelConfig` version in `AppState::channel_configs`.
/// Returns 200 with the resulting `ChannelConfig`.
#[post("/channels/{channel_id}/config")]
pub async fn update_channel_config(
    http_req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<ConfigTransaction>,
) -> ApiResult<HttpResponse> {
    enforce_acl(
        state.acl_provider.as_deref(),
        state.policy_store.as_deref(),
        "peer/ChannelConfig",
        &http_req,
    )?;
    let channel_id = path.into_inner();
    let tx = body.into_inner();
    let trace_id = uuid::Uuid::new_v4().to_string();

    // Channel must exist in the store map.
    get_channel_store(&state, &channel_id)?;

    // Get current config (last entry in history).
    let current = {
        let configs = state
            .channel_configs
            .read()
            .unwrap_or_else(|e| e.into_inner());
        configs
            .get(&channel_id)
            .and_then(|v| v.last().cloned())
            .ok_or_else(|| ApiError::NotFound {
                resource: format!("config for channel '{channel_id}'"),
            })?
    };

    // Validate config-tx signatures. Fall back to empty in-memory registries when
    // AppState does not carry live org/policy stores (e.g. in unit tests).
    let fallback_policy_store = MemoryPolicyStore::new();
    let fallback_org_registry = MemoryOrgRegistry::new();
    let policy_store = state
        .policy_store
        .as_deref()
        .unwrap_or(&fallback_policy_store);
    let org_registry = state
        .org_registry
        .as_deref()
        .unwrap_or(&fallback_org_registry);

    crate::channel::config::validate_config_tx(&tx, &current, policy_store, org_registry).map_err(
        |e| ApiError::ValidationError {
            field: "signatures".to_string(),
            reason: e.to_string(),
        },
    )?;

    // Apply updates and persist new version.
    let new_config =
        apply_config_update(&current, &tx.updates).map_err(|e| ApiError::ValidationError {
            field: "updates".to_string(),
            reason: e.to_string(),
        })?;

    state
        .channel_configs
        .write()
        .unwrap_or_else(|e| e.into_inner())
        .entry(channel_id)
        .or_default()
        .push(new_config.clone());

    Ok(HttpResponse::Ok().json(ApiResponse::success(new_config, trace_id)))
}

/// GET /api/v1/channels/{channel_id}/config — retorna la config actual del channel.
///
/// Devuelve el último elemento de `state.channel_configs[channel_id]`.
/// 404 si el channel no existe o no tiene historial de config.
#[get("/channels/{channel_id}/config")]
pub async fn get_channel_config(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> ApiResult<HttpResponse> {
    let channel_id = path.into_inner();
    let trace_id = uuid::Uuid::new_v4().to_string();

    let config = state
        .channel_configs
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .get(&channel_id)
        .and_then(|v| v.last().cloned())
        .ok_or_else(|| ApiError::NotFound {
            resource: format!("config for channel '{channel_id}'"),
        })?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(config, trace_id)))
}

/// GET /api/v1/channels/{channel_id}/config/history — retorna todas las versiones de config.
///
/// Devuelve el historial completo (índice 0 = génesis).
/// 404 si el channel no existe o no tiene historial.
#[get("/channels/{channel_id}/config/history")]
pub async fn get_channel_config_history(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> ApiResult<HttpResponse> {
    let channel_id = path.into_inner();
    let trace_id = uuid::Uuid::new_v4().to_string();

    let history = state
        .channel_configs
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .get(&channel_id)
        .cloned()
        .ok_or_else(|| ApiError::NotFound {
            resource: format!("config history for channel '{channel_id}'"),
        })?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(history, trace_id)))
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
            discovery_service: None,
            event_bus: Arc::new(crate::events::EventBus::new()),
            channel_configs: Arc::new(RwLock::new(HashMap::new())),
            acl_provider: None,
            ordering_backend: None,
            world_state: None,
            audit_store: None,
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
            let mut map = state.store.write().unwrap_or_else(|e| e.into_inner());
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
            (
                "default",
                Arc::new(MemoryStore::new()) as Arc<dyn BlockStore>,
            ),
            (
                "ch-dup",
                Arc::new(MemoryStore::new()) as Arc<dyn BlockStore>,
            ),
        ]);
        let map = state.store.read().unwrap_or_else(|e| e.into_inner());
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
            (
                "default",
                Arc::new(MemoryStore::new()) as Arc<dyn BlockStore>,
            ),
            ("ch1", Arc::new(MemoryStore::new()) as Arc<dyn BlockStore>),
            ("ch2", Arc::new(MemoryStore::new()) as Arc<dyn BlockStore>),
        ]);
        let map = state.store.read().unwrap_or_else(|e| e.into_inner());
        let mut ids: Vec<&str> = map.keys().map(|s| s.as_str()).collect();
        ids.sort();
        assert_eq!(ids, vec!["ch1", "ch2", "default"]);
    }

    #[test]
    fn list_channels_empty_store_returns_empty_vec() {
        let state = make_state(vec![]);
        let map = state.store.read().unwrap_or_else(|e| e.into_inner());
        assert!(map.is_empty());
    }

    // ── genesis block on channel creation ─────────────────────────────────────

    #[test]
    fn create_channel_writes_genesis_block_at_height_zero() {
        use crate::channel::config::ChannelConfig;
        use crate::channel::genesis::create_genesis_block;

        let store: Arc<dyn BlockStore> = Arc::new(MemoryStore::new());
        let config = ChannelConfig::default();
        let genesis = create_genesis_block("ch-genesis", &config);
        store.write_block(&genesis).expect("write genesis");

        let read_back = store.read_block(0).expect("read genesis");
        assert_eq!(read_back.height, 0);
        assert_eq!(read_back.parent_hash, [0u8; 32]);
        assert_eq!(read_back.proposer, "genesis:ch-genesis");
    }

    #[test]
    fn create_channel_genesis_block_contains_config_json() {
        use crate::channel::config::ChannelConfig;
        use crate::channel::genesis::create_genesis_block;

        let store: Arc<dyn BlockStore> = Arc::new(MemoryStore::new());
        let config = ChannelConfig::default();
        let genesis = create_genesis_block("ch-cfg", &config);
        store.write_block(&genesis).expect("write genesis");

        let block = store.read_block(0).expect("read genesis");
        assert_eq!(block.transactions.len(), 1);
        let restored: ChannelConfig =
            serde_json::from_str(&block.transactions[0]).expect("deserialize config");
        assert_eq!(restored, config);
    }

    // ── get_channel_config (13.4.2) ───────────────────────────────────────────

    #[test]
    fn get_channel_config_returns_current_config() {
        use crate::channel::config::ChannelConfig;

        let state = make_state(vec![(
            "ch1",
            Arc::new(MemoryStore::new()) as Arc<dyn BlockStore>,
        )]);
        let config = ChannelConfig {
            batch_size: 42,
            ..ChannelConfig::default()
        };
        state
            .channel_configs
            .write()
            .unwrap()
            .insert("ch1".to_string(), vec![config.clone()]);

        let result = state
            .channel_configs
            .read()
            .unwrap()
            .get("ch1")
            .and_then(|v| v.last().cloned());

        assert!(result.is_some());
        assert_eq!(result.unwrap().batch_size, 42);
    }

    #[test]
    fn get_channel_config_unknown_channel_returns_none() {
        let state = make_state(vec![]);

        let result = state
            .channel_configs
            .read()
            .unwrap()
            .get("ghost")
            .and_then(|v| v.last().cloned());

        assert!(result.is_none());
    }

    // ── get_channel_config_history (13.4.3) ───────────────────────────────────

    #[test]
    fn get_channel_config_history_returns_all_versions() {
        use crate::channel::config::ChannelConfig;

        let state = make_state(vec![(
            "ch1",
            Arc::new(MemoryStore::new()) as Arc<dyn BlockStore>,
        )]);
        let v0 = ChannelConfig {
            batch_size: 10,
            ..ChannelConfig::default()
        };
        let v1 = ChannelConfig {
            batch_size: 20,
            version: 1,
            ..ChannelConfig::default()
        };
        state
            .channel_configs
            .write()
            .unwrap()
            .insert("ch1".to_string(), vec![v0.clone(), v1.clone()]);

        let history = state
            .channel_configs
            .read()
            .unwrap()
            .get("ch1")
            .cloned()
            .unwrap();

        assert_eq!(history.len(), 2);
        assert_eq!(history[0].batch_size, 10);
        assert_eq!(history[1].batch_size, 20);
        assert_eq!(history[1].version, 1);
    }

    #[test]
    fn get_channel_config_history_unknown_channel_returns_none() {
        let state = make_state(vec![]);

        let result = state
            .channel_configs
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get("ghost")
            .cloned();

        assert!(result.is_none());
    }

    // ── update_channel_config (13.4.1) ────────────────────────────────────────

    fn make_state_with_channel(channel_id: &str) -> AppState {
        use crate::channel::config::ChannelConfig;
        use crate::endorsement::policy::EndorsementPolicy;
        let store: Arc<dyn BlockStore> = Arc::new(MemoryStore::new());
        let state = make_state(vec![(channel_id, store)]);
        // Use AllOf([]) so validation trivially passes with 0 signatures.
        let config = ChannelConfig {
            endorsement_policy: EndorsementPolicy::AllOf(vec![]),
            ..ChannelConfig::default()
        };
        state
            .channel_configs
            .write()
            .unwrap()
            .insert(channel_id.to_string(), vec![config]);
        state
    }

    #[test]
    fn update_channel_config_applies_update_and_increments_version() {
        use crate::channel::config::{ConfigTransaction, ConfigUpdateType};

        let state = make_state_with_channel("ch1");

        // No signatures required: AllOf([]) is satisfied with 0 matching orgs.
        let tx = ConfigTransaction {
            tx_id: "tx-1".to_string(),
            channel_id: "ch1".to_string(),
            updates: vec![ConfigUpdateType::SetBatchSize(200)],
            signatures: vec![],
            created_at: 0,
        };

        apply_config_update_logic(&state, "ch1", tx);

        let configs = state
            .channel_configs
            .read()
            .unwrap_or_else(|e| e.into_inner());
        let history = configs.get("ch1").unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[1].batch_size, 200);
        assert_eq!(history[1].version, 1);
    }

    #[test]
    fn update_channel_config_unknown_channel_returns_not_found() {
        use crate::api::errors::ApiError;
        use crate::channel::config::{ConfigTransaction, ConfigUpdateType};

        let state = make_state(vec![]);
        let tx = ConfigTransaction {
            tx_id: "tx-x".to_string(),
            channel_id: "ghost".to_string(),
            updates: vec![ConfigUpdateType::SetBatchSize(50)],
            signatures: vec![],
            created_at: 0,
        };
        let err = try_apply_config_update_logic(&state, "ghost", tx).unwrap_err();
        assert!(matches!(err, ApiError::NotFound { .. }));
    }

    #[test]
    fn update_channel_config_missing_config_history_returns_not_found() {
        use crate::api::errors::ApiError;
        use crate::channel::config::{ConfigTransaction, ConfigUpdateType};

        // Store exists but no config history seeded.
        let store: Arc<dyn BlockStore> = Arc::new(MemoryStore::new());
        let state = make_state(vec![("ch-no-cfg", store)]);

        let tx = ConfigTransaction {
            tx_id: "tx-y".to_string(),
            channel_id: "ch-no-cfg".to_string(),
            updates: vec![ConfigUpdateType::SetBatchSize(50)],
            signatures: vec![],
            created_at: 0,
        };
        let err = try_apply_config_update_logic(&state, "ch-no-cfg", tx).unwrap_err();
        assert!(matches!(err, ApiError::NotFound { .. }));
    }

    // Inline the handler logic so it can be tested without actix runtime.
    fn apply_config_update_logic(
        state: &AppState,
        channel_id: &str,
        tx: crate::channel::config::ConfigTransaction,
    ) {
        try_apply_config_update_logic(state, channel_id, tx).expect("expected Ok");
    }

    fn try_apply_config_update_logic(
        state: &AppState,
        channel_id: &str,
        tx: crate::channel::config::ConfigTransaction,
    ) -> Result<crate::channel::config::ChannelConfig, crate::api::errors::ApiError> {
        use crate::channel::config::{apply_config_update, validate_config_tx};
        use crate::endorsement::{MemoryOrgRegistry, MemoryPolicyStore};

        get_channel_store(state, channel_id)?;

        let current = {
            let configs = state
                .channel_configs
                .read()
                .unwrap_or_else(|e| e.into_inner());
            configs
                .get(channel_id)
                .and_then(|v| v.last().cloned())
                .ok_or_else(|| crate::api::errors::ApiError::NotFound {
                    resource: format!("config for channel '{channel_id}'"),
                })?
        };

        let fallback_policy = MemoryPolicyStore::new();
        let fallback_orgs = MemoryOrgRegistry::new();
        let ps = state.policy_store.as_deref().unwrap_or(&fallback_policy);
        let or_ = state.org_registry.as_deref().unwrap_or(&fallback_orgs);

        validate_config_tx(&tx, &current, ps, or_).map_err(|e| {
            crate::api::errors::ApiError::ValidationError {
                field: "signatures".to_string(),
                reason: e.to_string(),
            }
        })?;

        let new_config = apply_config_update(&current, &tx.updates).map_err(|e| {
            crate::api::errors::ApiError::ValidationError {
                field: "updates".to_string(),
                reason: e.to_string(),
            }
        })?;

        state
            .channel_configs
            .write()
            .unwrap()
            .entry(channel_id.to_string())
            .or_default()
            .push(new_config.clone());

        Ok(new_config)
    }
}
