//! WebSocket and long-polling handlers for block event subscriptions.
//!
//! ## WebSocket (`GET /api/v1/events/blocks`)
//!
//! Upgrades the HTTP connection to WebSocket and forwards every [`BlockEvent`]
//! published on the node's [`EventBus`] as a JSON Text frame.
//!
//! After connecting, the client may send a JSON filter message:
//! ```json
//! { "channel_id": "mychannel", "chaincode_id": "basic" }
//! ```
//! Both fields are optional. When set:
//! - `channel_id` — only events whose `channel_id` matches are forwarded.
//! - `chaincode_id` — only [`BlockEvent::ChaincodeEvent`]s whose `chaincode_id`
//!   matches are forwarded; other event types are unaffected.
//!
//! ## Long-polling (`GET /api/v1/events/blocks?from_height=N`)
//!
//! Returns all stored blocks from height `N` (inclusive) up to the latest as
//! a JSON array.  Intended for clients that cannot use WebSocket.

use actix_web::{get, web, HttpRequest, HttpResponse};
use actix_ws::Message;
use serde::Deserialize;

use crate::api::errors::{ApiError, ApiResponse, ApiResult};
use crate::api::handlers::channels::{channel_id_from_req, get_channel_store};
use crate::app_state::AppState;
use crate::events::BlockEvent;

// ── Filter (WebSocket) ────────────────────────────────────────────────────────

/// Optional filter sent by the WebSocket client as a JSON text frame.
#[derive(Debug, Default, Deserialize)]
struct WsFilter {
    channel_id: Option<String>,
    chaincode_id: Option<String>,
}

/// Returns `true` when `event` passes the given `filter`.
///
/// - If `filter.channel_id` is set, the event's `channel_id` must match.
/// - If `filter.chaincode_id` is set, only [`BlockEvent::ChaincodeEvent`]s with
///   a matching `chaincode_id` pass; other event types are unaffected by this field.
fn event_passes_filter(event: &BlockEvent, filter: &WsFilter) -> bool {
    if let Some(ref fch) = filter.channel_id {
        let event_channel = match event {
            BlockEvent::BlockCommitted { channel_id, .. } => channel_id.as_str(),
            BlockEvent::TransactionCommitted { channel_id, .. } => channel_id.as_str(),
            BlockEvent::ChaincodeEvent { channel_id, .. } => channel_id.as_str(),
        };
        if event_channel != fch {
            return false;
        }
    }
    if let Some(ref fcc) = filter.chaincode_id {
        if let BlockEvent::ChaincodeEvent { chaincode_id, .. } = event {
            if chaincode_id != fcc {
                return false;
            }
        }
    }
    true
}

// ── Query params (long-polling) ───────────────────────────────────────────────

#[derive(Debug, Default, Deserialize)]
struct BlocksQuery {
    from_height: Option<u64>,
}

// ── Combined handler ──────────────────────────────────────────────────────────

/// `GET /api/v1/events/blocks[?from_height=N]`
///
/// - **With `from_height`**: returns stored blocks from height `N` to latest as JSON.
/// - **Without `from_height`**: upgrades to WebSocket and streams [`BlockEvent`]s.
#[get("/events/blocks")]
pub async fn events_blocks(
    req: HttpRequest,
    stream: web::Payload,
    query: web::Query<BlocksQuery>,
    state: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    if let Some(from_height) = query.from_height {
        poll_blocks(req, state, from_height)
            .await
            .map_err(actix_web::error::Error::from)
    } else {
        ws_stream(req, stream, state).await
    }
}

/// REST long-polling: return all blocks from `from_height` to the latest height.
async fn poll_blocks(
    req: HttpRequest,
    state: web::Data<AppState>,
    from_height: u64,
) -> ApiResult<HttpResponse> {
    let trace_id = uuid::Uuid::new_v4().to_string();
    let channel = channel_id_from_req(&req);
    let store = get_channel_store(&state, channel)?;

    let latest = store
        .get_latest_height()
        .map_err(|e| ApiError::StorageError { reason: e.to_string() })?;

    let mut blocks = Vec::new();
    for h in from_height..=latest {
        match store.read_block(h) {
            Ok(block) => blocks.push(block),
            Err(_) => {} // height may not exist yet — skip
        }
    }

    Ok(HttpResponse::Ok().json(ApiResponse::success(blocks, trace_id)))
}

/// WebSocket upgrade: stream [`BlockEvent`]s to the client.
async fn ws_stream(
    req: HttpRequest,
    stream: web::Payload,
    state: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    let (response, mut session, mut msg_stream) = actix_ws::handle(&req, stream)?;

    let mut rx = state.event_bus.subscribe();

    actix_web::rt::spawn(async move {
        let mut filter = WsFilter::default();
        loop {
            tokio::select! {
                result = rx.recv() => {
                    match result {
                        Ok(event) => {
                            if !event_passes_filter(&event, &filter) {
                                continue;
                            }
                            let json = match serde_json::to_string(&event) {
                                Ok(j) => j,
                                Err(_) => continue,
                            };
                            if session.text(json).await.is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
                msg = msg_stream.recv() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            if let Ok(f) = serde_json::from_str::<WsFilter>(&text) {
                                filter = f;
                            }
                        }
                        Some(Ok(Message::Close(_))) | None => {
                            let _ = session.close(None).await;
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }
    });

    Ok(response)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex, RwLock};

    use actix_web::{http::StatusCode, test, App};

    use crate::{
        airdrop::AirdropManager,
        billing::BillingManager,
        blockchain::Blockchain,
        cache::BalanceCache,
        events::{BlockEvent, EventBus},
        metrics::MetricsCollector,
        models::{Mempool, WalletManager},
        smart_contracts::ContractManager,
        staking::StakingManager,
        storage::{traits::Block, BlockStore, MemoryStore},
        transaction_validation::TransactionValidator,
        AppState,
    };

    fn sample_block(height: u64) -> Block {
        Block {
            height,
            timestamp: 1_000,
            parent_hash: [0u8; 32],
            merkle_root: [1u8; 32],
            transactions: vec![],
            proposer: "node-1".to_string(),
            signature: [2u8; 64],
            endorsements: vec![],
        }
    }

    fn make_state(bus: Arc<EventBus>) -> AppState {
        let store: Arc<dyn BlockStore> = Arc::new(MemoryStore::new());
        let mut map = std::collections::HashMap::new();
        map.insert("default".to_string(), store);

        AppState {
            blockchain: Arc::new(Mutex::new(Blockchain::new(1))),
            wallet_manager: Arc::new(Mutex::new(WalletManager::new())),
            block_storage: None,
            node: None,
            mempool: Arc::new(Mutex::new(Mempool::new())),
            balance_cache: Arc::new(BalanceCache::new()),
            billing_manager: Arc::new(BillingManager::new()),
            contract_manager: Arc::new(RwLock::new(ContractManager::new())),
            staking_manager: Arc::new(StakingManager::new(None, None, None)),
            airdrop_manager: Arc::new(AirdropManager::new(100, 10, "test".to_string())),
            pruning_manager: None,
            checkpoint_manager: None,
            transaction_validator: Arc::new(Mutex::new(TransactionValidator::with_defaults())),
            metrics: Arc::new(MetricsCollector::new()),
            store: Arc::new(RwLock::new(map)),
            org_registry: None,
            policy_store: None,
            crl_store: None,
            private_data_store: None,
            collection_registry: None,
            chaincode_package_store: None,
            chaincode_definition_store: None,
            gateway: None,
            discovery_service: None,
            event_bus: bus,
        }
    }

    // ── WebSocket tests ───────────────────────────────────────────────────────

    #[actix_web::test]
    async fn ws_upgrade_returns_101() {
        let bus = Arc::new(EventBus::new());
        let state = make_state(bus);

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(state))
                .service(events_blocks),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/events/blocks")
            .insert_header(("Connection", "Upgrade"))
            .insert_header(("Upgrade", "websocket"))
            .insert_header(("Sec-WebSocket-Key", "x3JJHMbDL1EzLkh9GBhXDw=="))
            .insert_header(("Sec-WebSocket-Version", "13"))
            .insert_header(("Host", "localhost"))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::SWITCHING_PROTOCOLS);
    }

    #[tokio::test]
    async fn event_bus_subscriber_receives_block_committed() {
        let bus = Arc::new(EventBus::new());
        let mut rx = bus.subscribe();

        let event = BlockEvent::BlockCommitted { channel_id: "ch".to_string(), height: 7, tx_count: 2 };
        bus.publish(event.clone());

        let received = rx.recv().await.expect("recv");
        assert_eq!(received, event);
    }

    #[actix_web::test]
    async fn block_event_serialises_to_valid_json() {
        let event = BlockEvent::BlockCommitted { channel_id: "ch".to_string(), height: 3, tx_count: 5 };
        let json = serde_json::to_string(&event).expect("serialize");
        let back: BlockEvent = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, event);
    }

    // ── Filter unit tests ─────────────────────────────────────────────────────

    fn bc(channel_id: &str) -> BlockEvent {
        BlockEvent::BlockCommitted { channel_id: channel_id.to_string(), height: 1, tx_count: 0 }
    }

    fn tc(channel_id: &str) -> BlockEvent {
        BlockEvent::TransactionCommitted {
            channel_id: channel_id.to_string(),
            tx_id: "tx1".to_string(),
            block_height: 1,
            valid: true,
        }
    }

    fn ce(channel_id: &str, chaincode_id: &str) -> BlockEvent {
        BlockEvent::ChaincodeEvent {
            channel_id: channel_id.to_string(),
            chaincode_id: chaincode_id.to_string(),
            event_name: "evt".to_string(),
            payload: vec![],
        }
    }

    #[tokio::test]
    async fn no_filter_passes_all_events() {
        let f = WsFilter::default();
        assert!(event_passes_filter(&bc("ch-a"), &f));
        assert!(event_passes_filter(&tc("ch-b"), &f));
        assert!(event_passes_filter(&ce("ch-c", "cc1"), &f));
    }

    #[tokio::test]
    async fn channel_filter_passes_matching_channel() {
        let f = WsFilter { channel_id: Some("ch-a".to_string()), chaincode_id: None };
        assert!(event_passes_filter(&bc("ch-a"), &f));
        assert!(event_passes_filter(&tc("ch-a"), &f));
        assert!(event_passes_filter(&ce("ch-a", "cc1"), &f));
    }

    #[tokio::test]
    async fn channel_filter_drops_other_channels() {
        let f = WsFilter { channel_id: Some("ch-a".to_string()), chaincode_id: None };
        assert!(!event_passes_filter(&bc("ch-b"), &f));
        assert!(!event_passes_filter(&tc("ch-b"), &f));
        assert!(!event_passes_filter(&ce("ch-b", "cc1"), &f));
    }

    #[tokio::test]
    async fn chaincode_filter_passes_matching_chaincode_event() {
        let f = WsFilter { channel_id: None, chaincode_id: Some("cc1".to_string()) };
        assert!(event_passes_filter(&ce("ch", "cc1"), &f));
    }

    #[tokio::test]
    async fn chaincode_filter_drops_non_matching_chaincode_event() {
        let f = WsFilter { channel_id: None, chaincode_id: Some("cc1".to_string()) };
        assert!(!event_passes_filter(&ce("ch", "cc2"), &f));
    }

    #[tokio::test]
    async fn chaincode_filter_does_not_affect_block_or_tx_events() {
        let f = WsFilter { channel_id: None, chaincode_id: Some("cc1".to_string()) };
        assert!(event_passes_filter(&bc("ch"), &f));
        assert!(event_passes_filter(&tc("ch"), &f));
    }

    #[tokio::test]
    async fn combined_filter_requires_both_to_match() {
        let f = WsFilter {
            channel_id: Some("ch-a".to_string()),
            chaincode_id: Some("cc1".to_string()),
        };
        assert!(event_passes_filter(&ce("ch-a", "cc1"), &f));
        assert!(!event_passes_filter(&ce("ch-b", "cc1"), &f));
        assert!(!event_passes_filter(&ce("ch-a", "cc2"), &f));
        assert!(event_passes_filter(&bc("ch-a"), &f));
        assert!(!event_passes_filter(&bc("ch-b"), &f));
    }

    // ── Long-polling tests ────────────────────────────────────────────────────

    fn make_state_with_blocks(heights: &[u64]) -> AppState {
        let store = Arc::new(MemoryStore::new());
        for &h in heights {
            store.write_block(&sample_block(h)).unwrap();
        }
        let store_arc: Arc<dyn BlockStore> = store;
        let mut map = std::collections::HashMap::new();
        map.insert("default".to_string(), store_arc);

        let bus = Arc::new(EventBus::new());
        AppState {
            blockchain: Arc::new(Mutex::new(Blockchain::new(1))),
            wallet_manager: Arc::new(Mutex::new(WalletManager::new())),
            block_storage: None,
            node: None,
            mempool: Arc::new(Mutex::new(Mempool::new())),
            balance_cache: Arc::new(BalanceCache::new()),
            billing_manager: Arc::new(BillingManager::new()),
            contract_manager: Arc::new(RwLock::new(ContractManager::new())),
            staking_manager: Arc::new(StakingManager::new(None, None, None)),
            airdrop_manager: Arc::new(AirdropManager::new(100, 10, "test".to_string())),
            pruning_manager: None,
            checkpoint_manager: None,
            transaction_validator: Arc::new(Mutex::new(TransactionValidator::with_defaults())),
            metrics: Arc::new(MetricsCollector::new()),
            store: Arc::new(RwLock::new(map)),
            org_registry: None,
            policy_store: None,
            crl_store: None,
            private_data_store: None,
            collection_registry: None,
            chaincode_package_store: None,
            chaincode_definition_store: None,
            gateway: None,
            discovery_service: None,
            event_bus: bus,
        }
    }

    #[actix_web::test]
    async fn poll_from_height_2_returns_2_of_3_blocks() {
        let state = make_state_with_blocks(&[1, 2, 3]);

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(state))
                .service(events_blocks),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/events/blocks?from_height=2")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: serde_json::Value = test::read_body_json(resp).await;
        let data = body["data"].as_array().expect("data array");
        assert_eq!(data.len(), 2, "expected blocks at heights 2 and 3");
        assert_eq!(data[0]["height"], 2);
        assert_eq!(data[1]["height"], 3);
    }

    #[actix_web::test]
    async fn poll_from_height_1_returns_all_3_blocks() {
        let state = make_state_with_blocks(&[1, 2, 3]);

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(state))
                .service(events_blocks),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/events/blocks?from_height=1")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: serde_json::Value = test::read_body_json(resp).await;
        let data = body["data"].as_array().expect("data array");
        assert_eq!(data.len(), 3);
    }

    #[actix_web::test]
    async fn poll_from_height_above_latest_returns_empty() {
        let state = make_state_with_blocks(&[1, 2, 3]);

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(state))
                .service(events_blocks),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/events/blocks?from_height=10")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: serde_json::Value = test::read_body_json(resp).await;
        let data = body["data"].as_array().expect("data array");
        assert_eq!(data.len(), 0);
    }

    #[actix_web::test]
    async fn poll_empty_store_returns_empty() {
        let state = make_state_with_blocks(&[]);

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(state))
                .service(events_blocks),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/events/blocks?from_height=1")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: serde_json::Value = test::read_body_json(resp).await;
        let data = body["data"].as_array().expect("data array");
        assert_eq!(data.len(), 0);
    }
}
