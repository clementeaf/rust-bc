//! Oracle endpoints:
//!   GET  /api/v1/oracle/feeds/{symbol}   — get latest price
//!   GET  /api/v1/oracle/feeds            — list all cached prices
//!   GET  /api/v1/oracle/nodes            — list registered oracle nodes

use actix_web::{get, web, HttpResponse};

use crate::api::errors::{ApiResponse, ApiResult, ErrorDto};
use crate::app_state::AppState;

/// GET /api/v1/oracle/feeds/{symbol}
#[get("/oracle/feeds/{symbol}")]
pub async fn get_oracle_feed(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> ApiResult<HttpResponse> {
    let trace = uuid::Uuid::new_v4().to_string();
    let symbol = path.into_inner();
    let registry = state.oracle_registry.lock().unwrap();

    match registry.get_price(&symbol) {
        Ok(price) => Ok(HttpResponse::Ok().json(ApiResponse::success(price, trace))),
        Err(e) => Ok(HttpResponse::NotFound().json(ApiResponse::<()>::error(
            ErrorDto {
                code: "ORACLE_ERROR".into(),
                message: e,
                field: None,
            },
            404,
        ))),
    }
}

/// GET /api/v1/oracle/feeds
#[get("/oracle/feeds")]
pub async fn list_oracle_feeds(state: web::Data<AppState>) -> ApiResult<HttpResponse> {
    let trace = uuid::Uuid::new_v4().to_string();
    let registry = state.oracle_registry.lock().unwrap();

    let feeds: Vec<_> = registry.price_cache.values().cloned().collect();

    Ok(HttpResponse::Ok().json(ApiResponse::success(feeds, trace)))
}

/// GET /api/v1/oracle/nodes
#[get("/oracle/nodes")]
pub async fn list_oracle_nodes(state: web::Data<AppState>) -> ApiResult<HttpResponse> {
    let trace = uuid::Uuid::new_v4().to_string();
    let registry = state.oracle_registry.lock().unwrap();

    let nodes: Vec<_> = registry.nodes.values().cloned().collect();
    Ok(HttpResponse::Ok().json(ApiResponse::success(nodes, trace)))
}
