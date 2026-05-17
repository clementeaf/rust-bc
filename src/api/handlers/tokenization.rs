//! HTTP endpoints for RWA Tokenization.

use actix_web::{delete, get, post, put, web, HttpRequest, HttpResponse};

use crate::api::errors::{enforce_acl, ApiError, ApiResponse, ApiResult};
use crate::api::handlers::channels::{channel_id_from_req, get_channel_store};
use crate::app_state::AppState;
use crate::registry::tokenization::AssetToken;

#[post("/store/tokens")]
pub async fn create_token(
    state: web::Data<AppState>,
    body: web::Json<AssetToken>,
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
        .write_asset_token(&body)
        .map_err(|e| ApiError::StorageError {
            reason: e.to_string(),
        })?;
    Ok(HttpResponse::Created().json(ApiResponse::success(&*body, trace)))
}

#[get("/store/tokens")]
pub async fn list_tokens(state: web::Data<AppState>, req: HttpRequest) -> ApiResult<HttpResponse> {
    let trace = uuid::Uuid::new_v4().to_string();
    let store = get_channel_store(&state, channel_id_from_req(&req))?;
    let tokens = store
        .list_asset_tokens()
        .map_err(|e| ApiError::StorageError {
            reason: e.to_string(),
        })?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(tokens, trace)))
}

#[get("/store/tokens/{id}")]
pub async fn get_token(
    state: web::Data<AppState>,
    id: web::Path<String>,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    let trace = uuid::Uuid::new_v4().to_string();
    let store = get_channel_store(&state, channel_id_from_req(&req))?;
    let token = store
        .read_asset_token(&id)
        .map_err(|e| ApiError::StorageError {
            reason: e.to_string(),
        })?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(token, trace)))
}

#[put("/store/tokens/{id}")]
pub async fn update_token(
    state: web::Data<AppState>,
    id: web::Path<String>,
    body: web::Json<AssetToken>,
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
    let mut token = body.into_inner();
    token.id = id.into_inner();
    store
        .write_asset_token(&token)
        .map_err(|e| ApiError::StorageError {
            reason: e.to_string(),
        })?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(token, trace)))
}

#[delete("/store/tokens/{id}")]
pub async fn remove_token(
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
        .delete_asset_token(&id)
        .map_err(|e| ApiError::StorageError {
            reason: e.to_string(),
        })?;
    Ok(HttpResponse::Ok().json(ApiResponse::success("deleted", trace)))
}
