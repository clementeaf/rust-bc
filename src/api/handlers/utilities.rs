use actix_web::{get, web, HttpRequest, HttpResponse};
// Note: `get` macro still used by `get_metrics` below

use crate::api::errors::ApiResult;
use crate::api::models::{BlockchainHealthResponse, HealthResponse, VersionResponse};
use crate::api::openapi::OpenApi;
use crate::app_state::AppState;

lazy_static::lazy_static! {
    static ref SCAFFOLD_HTTP_SINCE: std::time::Instant = std::time::Instant::now();
}

/// GET /api/v1/health — health check (NeuroAccess-style envelope + live chain/staking).
pub async fn health_check(state: web::Data<AppState>) -> ApiResult<HttpResponse> {
    let trace_id = uuid::Uuid::new_v4().to_string();
    let uptime_seconds = SCAFFOLD_HTTP_SINCE.elapsed().as_secs();

    let blockchain = state.blockchain.lock().unwrap_or_else(|e| e.into_inner());
    let latest = blockchain.get_latest_block().clone();
    let height = latest.index;
    let last_block_hash = latest.hash;
    drop(blockchain);

    let validators_count = state.staking_manager.get_active_validators().len();

    let response = HealthResponse {
        status: "healthy".to_string(),
        uptime_seconds,
        blockchain: BlockchainHealthResponse {
            height,
            last_block_hash,
            validators_count,
        },
    };

    let api_response = crate::api::errors::ApiResponse::success(response, trace_id);
    Ok(HttpResponse::Ok().json(api_response))
}

/// GET /api/v1/version — API and crate version metadata with live chain height.
pub async fn get_version(state: web::Data<AppState>) -> ApiResult<HttpResponse> {
    let trace_id = uuid::Uuid::new_v4().to_string();
    let blockchain = state.blockchain.lock().unwrap_or_else(|e| e.into_inner());
    let blockchain_height = blockchain.get_latest_block().index;
    drop(blockchain);

    let response = VersionResponse {
        api_version: "1.0.0".to_string(),
        rust_bc_version: env!("CARGO_PKG_VERSION").to_string(),
        blockchain_height,
    };

    let api_response = crate::api::errors::ApiResponse::success(response, trace_id);
    Ok(HttpResponse::Ok().json(api_response))
}

/// GET /api/v1/openapi.json — OpenAPI specification.
pub async fn get_openapi(_req: HttpRequest) -> ApiResult<HttpResponse> {
    let spec = OpenApi::spec();
    Ok(HttpResponse::Ok().json(spec))
}

/// GET /metrics — Prometheus text exposition format (0.0.4).
///
/// Intentionally outside `/api/v1` so Prometheus scrapers can use the
/// conventional path without a prefix.
#[get("/metrics")]
pub async fn get_metrics(state: web::Data<AppState>) -> HttpResponse {
    let body = state.metrics.collect_metrics();
    HttpResponse::Ok()
        .content_type("text/plain; version=0.0.4; charset=utf-8")
        .body(body)
}

#[cfg(test)]
mod tests {
    use super::SCAFFOLD_HTTP_SINCE;

    #[test]
    fn test_scaffold_uptime_clock_started() {
        assert!(SCAFFOLD_HTTP_SINCE.elapsed().as_secs() < 60);
    }
}
