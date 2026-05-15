//! Vault endpoints — encrypted wallet backup storage.
//!
//! The node stores encrypted wallet blobs keyed by DID. The wallet content
//! is opaque — the node never parses or decrypts it.

use crate::api::errors::{ApiResponse, ApiResult, ErrorDto};
use crate::api::handlers::channels::{channel_id_from_req, get_channel_store};
use crate::app_state::AppState;
use actix_web::{get, post, web, HttpRequest, HttpResponse};
use serde::Deserialize;

fn err_dto(msg: &str) -> ErrorDto {
    ErrorDto {
        code: "VAULT_ERROR".to_string(),
        message: msg.to_string(),
        field: None,
    }
}

#[derive(Deserialize)]
pub struct VaultStoreRequest {
    pub did: String,
    pub encrypted_wallet: serde_json::Value,
}

/// POST /api/v1/vault/store — store an encrypted wallet backup
#[post("/vault/store")]
pub async fn vault_store(
    state: web::Data<AppState>,
    body: web::Json<VaultStoreRequest>,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    let trace = uuid::Uuid::new_v4().to_string();

    if body.did.is_empty() {
        return Ok(HttpResponse::BadRequest()
            .json(ApiResponse::<()>::error(err_dto("did is required"), 400)));
    }

    let channel = channel_id_from_req(&req);
    let store = get_channel_store(&state, channel)?;

    store
        .write_vault(&body.did, &body.encrypted_wallet)
        .map_err(|e| crate::api::errors::ApiError::StorageError {
            reason: e.to_string(),
        })?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(
        serde_json::json!({ "did": body.did }),
        trace,
    )))
}

/// GET /api/v1/vault/{did} — retrieve an encrypted wallet backup
#[get("/vault/{did}")]
pub async fn vault_get(
    state: web::Data<AppState>,
    path: web::Path<String>,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    let trace = uuid::Uuid::new_v4().to_string();
    let did_raw = path.into_inner();
    let did = urlencoding::decode(&did_raw)
        .unwrap_or_default()
        .to_string();

    let channel = channel_id_from_req(&req);
    let store = get_channel_store(&state, channel)?;

    match store.read_vault(&did) {
        Ok(encrypted_wallet) => Ok(HttpResponse::Ok().json(ApiResponse::success(
            serde_json::json!({
                "did": did,
                "encrypted_wallet": encrypted_wallet,
            }),
            trace,
        ))),
        Err(_) => Ok(HttpResponse::NotFound().json(ApiResponse::<()>::error(
            err_dto(&format!("vault entry not found: {did}")),
            404,
        ))),
    }
}
