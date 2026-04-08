use actix_web::{get, web, HttpRequest, HttpResponse};
// Note: `get` macro still used by `get_metrics` below

use crate::api::errors::ApiResult;
use crate::api::models::{BlockchainHealthResponse, HealthChecks, HealthResponse, VersionResponse};
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

    // Check storage health
    let storage_ok = {
        let stores = state.store.read().unwrap_or_else(|e| e.into_inner());
        stores
            .get("default")
            .and_then(|s| s.get_latest_height().ok())
            .is_some()
    };

    // Check peer connectivity
    let peer_count = state
        .node
        .as_ref()
        .map(|n| n.peers.lock().unwrap_or_else(|e| e.into_inner()).len())
        .unwrap_or(0);

    // Check ordering service
    let ordering_ok = state.ordering_backend.is_some();

    let storage_status = if storage_ok { "ok" } else { "unavailable" };
    let peers_status = if peer_count > 0 {
        format!("ok ({peer_count} connected)")
    } else {
        "none".to_string()
    };
    let ordering_status = if ordering_ok { "ok" } else { "unavailable" };

    let degraded = !storage_ok || !ordering_ok;
    let overall_status = if degraded { "degraded" } else { "healthy" };

    let response = HealthResponse {
        status: overall_status.to_string(),
        uptime_seconds,
        blockchain: BlockchainHealthResponse {
            height,
            last_block_hash,
            validators_count,
        },
        checks: Some(HealthChecks {
            storage: storage_status.to_string(),
            peers: peers_status,
            ordering: ordering_status.to_string(),
        }),
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

/// GET /swagger — Swagger UI for interactive API exploration.
pub async fn swagger_ui(_req: HttpRequest) -> HttpResponse {
    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8"/>
  <meta name="viewport" content="width=device-width, initial-scale=1"/>
  <title>rust-bc API — Swagger UI</title>
  <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css"/>
</head>
<body>
  <div id="swagger-ui"></div>
  <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
  <script>
    SwaggerUIBundle({
      url: '/api/v1/openapi.json',
      dom_id: '#swagger-ui',
      presets: [SwaggerUIBundle.presets.apis, SwaggerUIBundle.SwaggerUIStandalonePreset],
      layout: 'StandaloneLayout',
      deepLinking: true,
    });
  </script>
</body>
</html>"#;
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html)
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
