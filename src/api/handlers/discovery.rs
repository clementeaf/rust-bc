//! Discovery handlers:
//!   GET  /api/v1/discovery/endorsers?chaincode={id}&channel={id}
//!   GET  /api/v1/discovery/peers?channel={id}
//!   POST /api/v1/discovery/register

use actix_web::{get, post, web, HttpResponse};
use serde::{Deserialize, Serialize};

use crate::api::errors::{ApiError, ApiResponse, ApiResult};
use crate::app_state::AppState;
use crate::discovery::PeerDescriptor;
use crate::discovery::service::DiscoveryError;

// ── Query params ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct EndorsersQuery {
    pub chaincode: String,
    pub channel: String,
}

// ── Response type ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct EndorsersResponse {
    pub peers: Vec<PeerDescriptor>,
}

// ── Handler ───────────────────────────────────────────────────────────────────

/// GET /api/v1/discovery/endorsers?chaincode={id}&channel={id}
///
/// Returns the minimum set of peers needed to satisfy the endorsement policy
/// for the given chaincode on the given channel.
#[get("/discovery/endorsers")]
pub async fn get_endorsers(
    state: web::Data<AppState>,
    query: web::Query<EndorsersQuery>,
) -> ApiResult<HttpResponse> {
    if query.chaincode.is_empty() {
        return Err(ApiError::ValidationError {
            field: "chaincode".to_string(),
            reason: "must not be empty".to_string(),
        });
    }
    if query.channel.is_empty() {
        return Err(ApiError::ValidationError {
            field: "channel".to_string(),
            reason: "must not be empty".to_string(),
        });
    }

    let svc = state
        .discovery_service
        .as_ref()
        .ok_or_else(|| ApiError::NotFound { resource: "discovery_service".to_string() })?;

    let peers = svc
        .endorsement_plan(&query.chaincode, &query.channel)
        .map_err(|e| match e {
            DiscoveryError::PolicyNotFound(_) => ApiError::NotFound {
                resource: format!("policy for {}/{}", query.channel, query.chaincode),
            },
            DiscoveryError::InsufficientPeers => ApiError::InternalError {
                reason: "cannot satisfy endorsement policy: insufficient peers".to_string(),
            },
            DiscoveryError::PeerNotFound(addr) => ApiError::NotFound {
                resource: format!("peer {addr}"),
            },
        })?;

    let trace_id = uuid::Uuid::new_v4().to_string();
    Ok(HttpResponse::Ok().json(ApiResponse::success(EndorsersResponse { peers }, trace_id)))
}

// ── channel peers ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ChannelPeersQuery {
    pub channel: String,
}

#[derive(Debug, Serialize)]
pub struct ChannelPeersResponse {
    pub peers: Vec<PeerDescriptor>,
}

/// GET /api/v1/discovery/peers?channel={id}
///
/// Returns all peers that participate in the given channel.
#[get("/discovery/peers")]
pub async fn get_channel_peers(
    state: web::Data<AppState>,
    query: web::Query<ChannelPeersQuery>,
) -> ApiResult<HttpResponse> {
    if query.channel.is_empty() {
        return Err(ApiError::ValidationError {
            field: "channel".to_string(),
            reason: "must not be empty".to_string(),
        });
    }

    let svc = state
        .discovery_service
        .as_ref()
        .ok_or_else(|| ApiError::NotFound { resource: "discovery_service".to_string() })?;

    let peers = svc.channel_peers(&query.channel);

    let trace_id = uuid::Uuid::new_v4().to_string();
    Ok(HttpResponse::Ok().json(ApiResponse::success(ChannelPeersResponse { peers }, trace_id)))
}

// ── register ──────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub peer_address: String,
}

/// POST /api/v1/discovery/register
///
/// Registers a peer in the discovery service. Intended to be called by a peer
/// at boot time so it becomes visible to endorsement-plan queries.
///
/// Body: JSON-encoded `PeerDescriptor`
#[post("/discovery/register")]
pub async fn post_register_peer(
    state: web::Data<AppState>,
    body: web::Json<PeerDescriptor>,
) -> ApiResult<HttpResponse> {
    if body.peer_address.is_empty() {
        return Err(ApiError::ValidationError {
            field: "peer_address".to_string(),
            reason: "must not be empty".to_string(),
        });
    }
    if body.org_id.is_empty() {
        return Err(ApiError::ValidationError {
            field: "org_id".to_string(),
            reason: "must not be empty".to_string(),
        });
    }

    let svc = state
        .discovery_service
        .as_ref()
        .ok_or_else(|| ApiError::NotFound { resource: "discovery_service".to_string() })?;

    let peer_address = body.peer_address.clone();
    svc.register_peer(body.into_inner());

    let trace_id = uuid::Uuid::new_v4().to_string();
    Ok(HttpResponse::Created()
        .json(ApiResponse::success(RegisterResponse { peer_address }, trace_id)))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};
    use std::sync::{Arc, Mutex, RwLock};

    use crate::airdrop::AirdropManager;
    use crate::billing::BillingManager;
    use crate::blockchain::Blockchain;
    use crate::cache::BalanceCache;
    use crate::discovery::service::DiscoveryService;
    use crate::discovery::PeerDescriptor;
    use crate::endorsement::policy::EndorsementPolicy;
    use crate::endorsement::policy_store::{MemoryPolicyStore, PolicyStore};
    use crate::endorsement::registry::MemoryOrgRegistry;
    use crate::metrics::MetricsCollector;
    use crate::models::{Mempool, WalletManager};
    use crate::ordering::NodeRole;
    use crate::smart_contracts::ContractManager;
    use crate::staking::StakingManager;
    use crate::storage::memory::MemoryStore;
    use crate::storage::traits::BlockStore;
    use crate::transaction_validation::{TransactionValidator, ValidationConfig};

    fn base_state(discovery_service: Option<Arc<DiscoveryService>>) -> web::Data<AppState> {
        let mut store_map = std::collections::HashMap::new();
        store_map.insert(
            "default".to_string(),
            Arc::new(MemoryStore::new()) as Arc<dyn BlockStore>,
        );
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
            transaction_validator: Arc::new(Mutex::new(TransactionValidator::new(
                ValidationConfig::default(),
            ))),
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
            discovery_service,
            event_bus: Arc::new(crate::events::EventBus::new()),
        })
    }

    fn make_svc_with_peers() -> Arc<DiscoveryService> {
        let ps = Arc::new(MemoryPolicyStore::new());
        ps.set_policy(
            "mychannel/basic",
            &EndorsementPolicy::NOutOf {
                n: 2,
                orgs: vec!["Org1MSP".into(), "Org2MSP".into(), "Org3MSP".into()],
            },
        )
        .unwrap();

        let svc = Arc::new(DiscoveryService::new(
            Arc::new(MemoryOrgRegistry::new()),
            ps,
        ));

        for (addr, org) in [
            ("peer1:7051", "Org1MSP"),
            ("peer2:7051", "Org2MSP"),
            ("peer3:7051", "Org3MSP"),
        ] {
            svc.register_peer(PeerDescriptor {
                peer_address: addr.to_string(),
                org_id: org.to_string(),
                role: NodeRole::Peer,
                chaincodes: vec!["basic".to_string()],
                channels: vec!["mychannel".to_string()],
                last_heartbeat: 1_000,
            });
        }
        svc
    }

    #[actix_web::test]
    async fn returns_endorsement_plan() {
        let state = base_state(Some(make_svc_with_peers()));
        let app = test::init_service(
            App::new()
                .app_data(state)
                .service(web::scope("/api/v1").service(get_endorsers)),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/v1/discovery/endorsers?chaincode=basic&channel=mychannel")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: serde_json::Value = test::read_body_json(resp).await;
        let peers = body["data"]["peers"].as_array().unwrap();
        assert_eq!(peers.len(), 2);
    }

    #[actix_web::test]
    async fn returns_404_when_discovery_not_configured() {
        let state = base_state(None);
        let app = test::init_service(
            App::new()
                .app_data(state)
                .service(web::scope("/api/v1").service(get_endorsers)),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/v1/discovery/endorsers?chaincode=basic&channel=mychannel")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 404);
    }

    #[actix_web::test]
    async fn returns_400_when_chaincode_empty() {
        let state = base_state(Some(make_svc_with_peers()));
        let app = test::init_service(
            App::new()
                .app_data(state)
                .service(web::scope("/api/v1").service(get_endorsers)),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/v1/discovery/endorsers?chaincode=&channel=mychannel")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);
    }

    // ── channel_peers handler tests ────────────────────────────────────────────

    fn make_svc_5_peers() -> Arc<DiscoveryService> {
        let svc = Arc::new(DiscoveryService::new(
            Arc::new(MemoryOrgRegistry::new()),
            Arc::new(MemoryPolicyStore::new()),
        ));
        for (addr, org, channel) in [
            ("peer1:7051", "Org1MSP", "mychannel"),
            ("peer2:7051", "Org2MSP", "mychannel"),
            ("peer3:7051", "Org3MSP", "mychannel"),
            ("peer4:7051", "Org1MSP", "otherchannel"),
            ("peer5:7051", "Org2MSP", "otherchannel"),
        ] {
            svc.register_peer(PeerDescriptor {
                peer_address: addr.to_string(),
                org_id: org.to_string(),
                role: NodeRole::Peer,
                chaincodes: vec!["basic".to_string()],
                channels: vec![channel.to_string()],
                last_heartbeat: 1_000,
            });
        }
        svc
    }

    #[actix_web::test]
    async fn channel_peers_returns_3_for_mychannel() {
        let state = base_state(Some(make_svc_5_peers()));
        let app = test::init_service(
            App::new()
                .app_data(state)
                .service(web::scope("/api/v1").service(get_channel_peers)),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/v1/discovery/peers?channel=mychannel")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: serde_json::Value = test::read_body_json(resp).await;
        let peers = body["data"]["peers"].as_array().unwrap();
        assert_eq!(peers.len(), 3);
    }

    #[actix_web::test]
    async fn channel_peers_returns_404_without_service() {
        let state = base_state(None);
        let app = test::init_service(
            App::new()
                .app_data(state)
                .service(web::scope("/api/v1").service(get_channel_peers)),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/v1/discovery/peers?channel=mychannel")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 404);
    }

    #[actix_web::test]
    async fn channel_peers_returns_400_when_channel_empty() {
        let state = base_state(Some(make_svc_5_peers()));
        let app = test::init_service(
            App::new()
                .app_data(state)
                .service(web::scope("/api/v1").service(get_channel_peers)),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/v1/discovery/peers?channel=")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);
    }

    // ── post_register_peer tests ───────────────────────────────────────────────

    fn valid_peer_json() -> serde_json::Value {
        serde_json::json!({
            "peer_address": "peer10:7051",
            "org_id": "Org1MSP",
            "role": "Peer",
            "chaincodes": ["basic"],
            "channels": ["mychannel"],
            "last_heartbeat": 0
        })
    }

    #[actix_web::test]
    async fn register_peer_returns_201_and_peer_appears_in_channel() {
        let svc = Arc::new(DiscoveryService::new(
            Arc::new(MemoryOrgRegistry::new()),
            Arc::new(MemoryPolicyStore::new()),
        ));
        let state = base_state(Some(svc));
        let app = test::init_service(
            App::new()
                .app_data(state.clone())
                .service(web::scope("/api/v1")
                    .service(post_register_peer)
                    .service(get_channel_peers)),
        )
        .await;

        // Register the peer
        let req = test::TestRequest::post()
            .uri("/api/v1/discovery/register")
            .set_json(&valid_peer_json())
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["data"]["peer_address"], "peer10:7051");

        // Verify it shows up in channel query
        let req2 = test::TestRequest::get()
            .uri("/api/v1/discovery/peers?channel=mychannel")
            .to_request();
        let resp2 = test::call_service(&app, req2).await;
        assert_eq!(resp2.status(), 200);

        let body2: serde_json::Value = test::read_body_json(resp2).await;
        let peers = body2["data"]["peers"].as_array().unwrap();
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0]["peer_address"], "peer10:7051");
    }

    #[actix_web::test]
    async fn register_peer_returns_404_without_service() {
        let state = base_state(None);
        let app = test::init_service(
            App::new()
                .app_data(state)
                .service(web::scope("/api/v1").service(post_register_peer)),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/v1/discovery/register")
            .set_json(&valid_peer_json())
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 404);
    }

    #[actix_web::test]
    async fn register_peer_returns_400_when_peer_address_empty() {
        let svc = Arc::new(DiscoveryService::new(
            Arc::new(MemoryOrgRegistry::new()),
            Arc::new(MemoryPolicyStore::new()),
        ));
        let state = base_state(Some(svc));
        let app = test::init_service(
            App::new()
                .app_data(state)
                .service(web::scope("/api/v1").service(post_register_peer)),
        )
        .await;

        let mut payload = valid_peer_json();
        payload["peer_address"] = serde_json::json!("");
        let req = test::TestRequest::post()
            .uri("/api/v1/discovery/register")
            .set_json(&payload)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);
    }

    #[actix_web::test]
    async fn register_peer_returns_400_when_org_id_empty() {
        let svc = Arc::new(DiscoveryService::new(
            Arc::new(MemoryOrgRegistry::new()),
            Arc::new(MemoryPolicyStore::new()),
        ));
        let state = base_state(Some(svc));
        let app = test::init_service(
            App::new()
                .app_data(state)
                .service(web::scope("/api/v1").service(post_register_peer)),
        )
        .await;

        let mut payload = valid_peer_json();
        payload["org_id"] = serde_json::json!("");
        let req = test::TestRequest::post()
            .uri("/api/v1/discovery/register")
            .set_json(&payload)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);
    }
}
