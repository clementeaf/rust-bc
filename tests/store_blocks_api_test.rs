//! T5: Actix-web integration tests for /api/v1/store/blocks/* endpoints.
//!
//! Verifies that blocks accepted via `ConsensusEngine` are persisted to the
//! shared `MemoryStore` and are readable through the REST API.

use std::sync::{Arc, RwLock};

use actix_web::{test, web, App};
use rust_bc::{
    api::{errors::ApiResponse, routes::ApiRoutes},
    consensus::{
        dag::DagBlock, engine::ConsensusEngine, fork_choice::ForkChoiceRule, ConsensusConfig,
    },
    storage::{traits::Block as StorageBlock, BlockStore, MemoryStore},
    AppState,
};

// ── helpers ──────────────────────────────────────────────────────────────────

fn mk(id: u8) -> [u8; 32] {
    let mut h = [0u8; 32];
    h[0] = id;
    h
}

/// Build a DagBlock that passes ConsensusEngine validation:
/// slot=0, timestamp in [0,6), proposer="v1".
fn dag_block(hash: u8, parent: u8, height: u64) -> DagBlock {
    DagBlock::new(
        mk(hash),
        mk(parent),
        height,
        0,
        0,
        "v1".to_string(),
        vec![2u8; 64],
    )
}

/// Minimal AppState wired to the given store.
fn make_state(store: Arc<MemoryStore>) -> AppState {
    let mut state = AppState::test_default();
    let mut m = std::collections::HashMap::new();
    m.insert("default".to_string(), store as Arc<dyn BlockStore>);
    state.store = Arc::new(RwLock::new(m));
    state
}

/// AppState with empty store map.
fn make_state_no_store() -> AppState {
    let mut state = AppState::test_default();
    state.store = Arc::new(RwLock::new(std::collections::HashMap::new()));
    state
}

/// Accept two blocks through the engine and return the pre-populated store.
fn store_with_two_blocks() -> Arc<MemoryStore> {
    let store = Arc::new(MemoryStore::new());
    let mut engine = ConsensusEngine::new(
        ConsensusConfig::default(),
        ForkChoiceRule::HeaviestSubtree,
        vec!["v1".to_string()],
        0,
    )
    .with_store(Box::new(Arc::clone(&store)));

    engine
        .accept_block(dag_block(1, 0, 0))
        .expect("height 0 accepted");
    engine
        .accept_block(dag_block(2, 1, 1))
        .expect("height 1 accepted");
    store
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[actix_web::test]
async fn get_block_by_height_returns_stored_block() {
    let store = store_with_two_blocks();
    let state = make_state(store);

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(state))
            .configure(ApiRoutes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/store/blocks/0")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status().as_u16(), 200);

    let body: ApiResponse<StorageBlock> = test::read_body_json(resp).await;
    let block = body.data.expect("response has data");
    assert_eq!(block.height, 0);
    assert_eq!(block.proposer, "v1");
}

#[actix_web::test]
async fn get_block_height_1_returns_correct_block() {
    let store = store_with_two_blocks();
    let state = make_state(store);

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(state))
            .configure(ApiRoutes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/store/blocks/1")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status().as_u16(), 200);

    let body: ApiResponse<StorageBlock> = test::read_body_json(resp).await;
    assert_eq!(body.data.expect("data present").height, 1);
}

#[actix_web::test]
async fn get_latest_height_returns_tip() {
    let store = store_with_two_blocks();
    let state = make_state(store);

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(state))
            .configure(ApiRoutes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/store/blocks/latest")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status().as_u16(), 200);

    let body: ApiResponse<u64> = test::read_body_json(resp).await;
    assert_eq!(body.data, Some(1));
}

#[actix_web::test]
async fn get_missing_block_returns_404() {
    let store = store_with_two_blocks();
    let state = make_state(store);

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(state))
            .configure(ApiRoutes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/store/blocks/99")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status().as_u16(), 404);
}

#[actix_web::test]
async fn no_store_latest_returns_404() {
    let state = make_state_no_store();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(state))
            .configure(ApiRoutes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/store/blocks/latest")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status().as_u16(), 404);
}

#[actix_web::test]
async fn no_store_get_block_returns_404() {
    let state = make_state_no_store();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(state))
            .configure(ApiRoutes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/store/blocks/0")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status().as_u16(), 404);
}

#[actix_web::test]
async fn latest_route_not_confused_with_height_param() {
    // "latest" must NOT be parsed as a numeric height — that would fail with
    // a 400/404 for a wrong reason.  The correct response is 200 with u64 data.
    let store = store_with_two_blocks();
    let state = make_state(store);

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(state))
            .configure(ApiRoutes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/store/blocks/latest")
        .to_request();
    let resp = test::call_service(&app, req).await;

    // Must be the /latest handler, not the /{height} handler trying to parse "latest"
    assert_eq!(
        resp.status().as_u16(),
        200,
        "latest route must resolve before /{{height}}"
    );
}
