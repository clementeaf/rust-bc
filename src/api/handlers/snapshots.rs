//! Snapshot API handlers: create, list, and download state snapshots.

use actix_web::{get, post, web, HttpRequest, HttpResponse};

use crate::api::errors::{enforce_acl, ApiError, ApiResponse, ApiResult};
use crate::app_state::AppState;
use crate::storage::snapshot::{self, StateSnapshot};

/// `POST /api/v1/snapshots/{channel_id}` — trigger snapshot creation.
#[post("/snapshots/{channel_id}")]
pub async fn create_snapshot(
    req: HttpRequest,
    path: web::Path<String>,
    state: web::Data<AppState>,
) -> ApiResult<HttpResponse> {
    enforce_acl(state.acl_provider.as_deref(), state.policy_store.as_deref(), "qscc/Snapshot.Admin", &req)?;
    let channel_id = path.into_inner();
    let trace_id = uuid::Uuid::new_v4().to_string();

    let store = {
        let map = state.store.read().unwrap();
        map.get(&channel_id).cloned()
    };

    let store = store.ok_or_else(|| ApiError::NotFound {
        resource: format!("channel '{channel_id}'"),
    })?;

    let world_state = state.world_state.as_ref().ok_or_else(|| ApiError::NotFound {
        resource: "world_state".to_string(),
    })?;

    let base_dir = std::env::var("SNAPSHOT_DIR")
        .unwrap_or_else(|_| "./data".to_string());

    let snap = snapshot::create_snapshot(
        store.as_ref(),
        world_state.as_ref(),
        &channel_id,
        std::path::Path::new(&base_dir),
    )
    .map_err(|e| ApiError::StorageError {
        reason: e.to_string(),
    })?;

    Ok(HttpResponse::Created().json(ApiResponse::success(snap, trace_id)))
}

/// `GET /api/v1/snapshots/{channel_id}` — list available snapshots.
#[get("/snapshots/{channel_id}")]
pub async fn list_snapshots(
    path: web::Path<String>,
    _state: web::Data<AppState>,
) -> ApiResult<HttpResponse> {
    let channel_id = path.into_inner();
    let trace_id = uuid::Uuid::new_v4().to_string();

    let base_dir = std::env::var("SNAPSHOT_DIR")
        .unwrap_or_else(|_| "./data".to_string());

    let snap_dir = std::path::Path::new(&base_dir)
        .join("snapshots")
        .join(&channel_id);

    let mut snapshots: Vec<serde_json::Value> = Vec::new();

    if snap_dir.exists() {
        let entries = std::fs::read_dir(&snap_dir)
            .map_err(|e| ApiError::StorageError { reason: e.to_string() })?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("snap") {
                let file_name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown");
                let metadata = std::fs::metadata(&path)
                    .map_err(|e| ApiError::StorageError { reason: e.to_string() })?;
                snapshots.push(serde_json::json!({
                    "snapshot_id": format!("{channel_id}-{file_name}"),
                    "channel_id": channel_id,
                    "block_height": file_name.parse::<u64>().unwrap_or(0),
                    "file_size": metadata.len(),
                }));
            }
        }
    }

    Ok(HttpResponse::Ok().json(ApiResponse::success(snapshots, trace_id)))
}

/// `GET /api/v1/snapshots/{channel_id}/{snapshot_id}` — download a snapshot file.
#[get("/snapshots/{channel_id}/{snapshot_id}")]
pub async fn download_snapshot(
    path: web::Path<(String, String)>,
    _state: web::Data<AppState>,
) -> ApiResult<HttpResponse> {
    let (channel_id, snapshot_id) = path.into_inner();

    // snapshot_id format: "{channel_id}-{height}" → extract height.
    let height_str = snapshot_id
        .strip_prefix(&format!("{channel_id}-"))
        .unwrap_or(&snapshot_id);

    let base_dir = std::env::var("SNAPSHOT_DIR")
        .unwrap_or_else(|_| "./data".to_string());

    let file_path = std::path::Path::new(&base_dir)
        .join("snapshots")
        .join(&channel_id)
        .join(format!("{height_str}.snap"));

    if !file_path.exists() {
        return Err(ApiError::NotFound {
            resource: format!("snapshot '{snapshot_id}'"),
        });
    }

    let content = std::fs::read(&file_path)
        .map_err(|e| ApiError::StorageError { reason: e.to_string() })?;

    Ok(HttpResponse::Ok()
        .content_type("application/octet-stream")
        .insert_header((
            "Content-Disposition",
            format!("attachment; filename=\"{height_str}.snap\""),
        ))
        .body(content))
}
