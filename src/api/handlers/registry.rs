//! HTTP endpoints for the Asset Registry module.

use actix_web::{delete, get, post, put, web, HttpRequest, HttpResponse};

use crate::api::errors::{enforce_acl, ApiError, ApiResponse, ApiResult};
use crate::api::handlers::channels::{channel_id_from_req, get_channel_store};
use crate::app_state::AppState;
use crate::registry::types::{Asset, AssetEvent};

// ── Assets ──────────────────────────────────────────────────────────────

#[post("/store/assets")]
pub async fn create_asset(
    state: web::Data<AppState>,
    body: web::Json<Asset>,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    enforce_acl(
        state.acl_provider.as_deref(),
        state.policy_store.as_deref(),
        "peer/Propose",
        &req,
    )?;
    let trace = uuid::Uuid::new_v4().to_string();
    let store = get_channel_store(&state, channel_id_from_req(&req))?;
    store
        .write_asset(&body)
        .map_err(|e| ApiError::StorageError {
            reason: e.to_string(),
        })?;
    Ok(HttpResponse::Created().json(ApiResponse::success(&*body, trace)))
}

#[get("/store/assets")]
pub async fn list_assets(state: web::Data<AppState>, req: HttpRequest) -> ApiResult<HttpResponse> {
    let trace = uuid::Uuid::new_v4().to_string();
    let store = get_channel_store(&state, channel_id_from_req(&req))?;
    let assets = store.list_assets().map_err(|e| ApiError::StorageError {
        reason: e.to_string(),
    })?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(assets, trace)))
}

#[get("/store/assets/{id}")]
pub async fn get_asset(
    state: web::Data<AppState>,
    id: web::Path<String>,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    let trace = uuid::Uuid::new_v4().to_string();
    let store = get_channel_store(&state, channel_id_from_req(&req))?;
    let asset = store.read_asset(&id).map_err(|e| ApiError::StorageError {
        reason: e.to_string(),
    })?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(asset, trace)))
}

#[put("/store/assets/{id}")]
pub async fn update_asset(
    state: web::Data<AppState>,
    id: web::Path<String>,
    body: web::Json<Asset>,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    enforce_acl(
        state.acl_provider.as_deref(),
        state.policy_store.as_deref(),
        "peer/Propose",
        &req,
    )?;
    let trace = uuid::Uuid::new_v4().to_string();
    let store = get_channel_store(&state, channel_id_from_req(&req))?;
    let mut asset = body.into_inner();
    asset.id = id.into_inner();
    store
        .write_asset(&asset)
        .map_err(|e| ApiError::StorageError {
            reason: e.to_string(),
        })?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(asset, trace)))
}

#[delete("/store/assets/{id}")]
pub async fn remove_asset(
    state: web::Data<AppState>,
    id: web::Path<String>,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    enforce_acl(
        state.acl_provider.as_deref(),
        state.policy_store.as_deref(),
        "peer/Propose",
        &req,
    )?;
    let trace = uuid::Uuid::new_v4().to_string();
    let store = get_channel_store(&state, channel_id_from_req(&req))?;
    store
        .delete_asset(&id)
        .map_err(|e| ApiError::StorageError {
            reason: e.to_string(),
        })?;
    Ok(HttpResponse::Ok().json(ApiResponse::success("deleted", trace)))
}

// ── Asset Events ────────────────────────────────────────────────────────

#[post("/store/assets/{asset_id}/events")]
pub async fn create_asset_event(
    state: web::Data<AppState>,
    asset_id: web::Path<String>,
    body: web::Json<AssetEvent>,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    enforce_acl(
        state.acl_provider.as_deref(),
        state.policy_store.as_deref(),
        "peer/Propose",
        &req,
    )?;
    let trace = uuid::Uuid::new_v4().to_string();
    let store = get_channel_store(&state, channel_id_from_req(&req))?;
    let mut event = body.into_inner();
    event.asset_id = asset_id.into_inner();
    store
        .write_asset_event(&event)
        .map_err(|e| ApiError::StorageError {
            reason: e.to_string(),
        })?;
    Ok(HttpResponse::Created().json(ApiResponse::success(event, trace)))
}

/// Bulk ingestion: POST multiple events at once (for telemetry feeds).
#[post("/store/assets/{asset_id}/events/batch")]
pub async fn create_asset_events_batch(
    state: web::Data<AppState>,
    asset_id: web::Path<String>,
    body: web::Json<Vec<AssetEvent>>,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    enforce_acl(
        state.acl_provider.as_deref(),
        state.policy_store.as_deref(),
        "peer/Propose",
        &req,
    )?;
    let trace = uuid::Uuid::new_v4().to_string();
    let store = get_channel_store(&state, channel_id_from_req(&req))?;
    let asset_id = asset_id.into_inner();
    let mut events = body.into_inner();
    for event in &mut events {
        event.asset_id = asset_id.clone();
    }
    let count = events.len();
    store
        .write_asset_events_batch(&events)
        .map_err(|e| ApiError::StorageError {
            reason: e.to_string(),
        })?;
    Ok(HttpResponse::Created().json(ApiResponse::success(
        serde_json::json!({"ingested": count}),
        trace,
    )))
}

#[get("/store/assets/{asset_id}/events")]
pub async fn list_asset_events(
    state: web::Data<AppState>,
    asset_id: web::Path<String>,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    let trace = uuid::Uuid::new_v4().to_string();
    let store = get_channel_store(&state, channel_id_from_req(&req))?;
    let events = store
        .list_asset_events(&asset_id)
        .map_err(|e| ApiError::StorageError {
            reason: e.to_string(),
        })?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(events, trace)))
}

#[get("/store/assets/{asset_id}/events/{event_id}")]
pub async fn get_asset_event(
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    let (_, event_id) = path.into_inner();
    let trace = uuid::Uuid::new_v4().to_string();
    let store = get_channel_store(&state, channel_id_from_req(&req))?;
    let event = store
        .read_asset_event(&event_id)
        .map_err(|e| ApiError::StorageError {
            reason: e.to_string(),
        })?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(event, trace)))
}

// ── Certified Export ────────────────────────────────────────────────────

/// GET /store/assets/{id}/export — generate a certified export of the
/// asset record + all its events, signed by the node.
#[get("/store/assets/{id}/export")]
pub async fn export_asset(
    state: web::Data<AppState>,
    id: web::Path<String>,
    req: HttpRequest,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> ApiResult<HttpResponse> {
    let trace = uuid::Uuid::new_v4().to_string();
    let store = get_channel_store(&state, channel_id_from_req(&req))?;

    let asset = store.read_asset(&id).map_err(|e| ApiError::StorageError {
        reason: e.to_string(),
    })?;
    let events = store
        .list_asset_events(&id)
        .map_err(|e| ApiError::StorageError {
            reason: e.to_string(),
        })?;

    let recipient = query
        .get("recipient")
        .cloned()
        .unwrap_or_else(|| "unspecified".to_string());

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Sign the export with the node's signing provider
    let export_data = serde_json::json!({
        "asset": &asset,
        "events_count": events.len(),
        "exported_at": now,
    });
    let signature = if let Some(ref signer) = state.signing_provider {
        let data_bytes = serde_json::to_vec(&export_data).unwrap_or_default();
        hex::encode(signer.sign(&data_bytes).unwrap_or_default())
    } else {
        String::new()
    };

    let export = crate::registry::types::CertifiedExport {
        id: uuid::Uuid::new_v4().to_string(),
        asset,
        events,
        requested_by: req
            .headers()
            .get("X-Org-Id")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("unknown")
            .to_string(),
        recipient,
        exported_at: now,
        signature,
    };

    Ok(HttpResponse::Ok().json(ApiResponse::success(export, trace)))
}
