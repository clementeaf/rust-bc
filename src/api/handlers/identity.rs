use actix_web::{web, HttpRequest, HttpResponse, post, get};
use crate::api::errors::{ApiError, ApiResponse, ApiResult};
use crate::api::handlers::channels::{channel_id_from_req, get_channel_store};
use crate::api::models::*;
use crate::app_state::AppState;
use chrono::Utc;

/// POST /identity/create - Create a new DID and keypair
#[post("/identity/create")]
async fn create_identity(
    _req: HttpRequest,
    _body: web::Json<CreateIdentityRequest>,
) -> ApiResult<HttpResponse> {
    let trace_id = uuid::Uuid::new_v4().to_string();

    // TODO: Generate Ed25519 keypair
    // TODO: Create DID document
    // TODO: Store in blockchain
    
    let response = IdentityResponse {
        did: "did:bc:placeholder".to_string(),
        public_key: "placeholder_key".to_string(),
        created_at: Utc::now(),
    };

    let api_response = crate::api::errors::ApiResponse::success(response, trace_id);
    Ok(HttpResponse::Ok().json(api_response))
}

/// GET /identity/{did} - Fetch DID document
#[get("/identity/{did}")]
async fn get_identity(
    _req: HttpRequest,
    path: web::Path<String>,
) -> ApiResult<HttpResponse> {
    let did = path.into_inner();
    let trace_id = uuid::Uuid::new_v4().to_string();

    // TODO: Query storage for DID document
    
    let response = IdentityResponse {
        did,
        public_key: "placeholder_key".to_string(),
        created_at: Utc::now(),
    };

    let api_response = crate::api::errors::ApiResponse::success(response, trace_id);
    Ok(HttpResponse::Ok().json(api_response))
}

/// POST /identity/{did}/rotate-key - Key rotation
#[post("/identity/{did}/rotate-key")]
async fn rotate_key(
    _req: HttpRequest,
    path: web::Path<String>,
    _body: web::Json<RotateKeyRequest>,
) -> ApiResult<HttpResponse> {
    let did = path.into_inner();
    let trace_id = uuid::Uuid::new_v4().to_string();

    // TODO: Call identity::KeyManager::rotate_key()
    
    let response = RotateKeyResponse {
        did,
        new_key_index: 1,
        rotated_at: Utc::now(),
    };

    let api_response = crate::api::errors::ApiResponse::success(response, trace_id);
    Ok(HttpResponse::Ok().json(api_response))
}

/// POST /identity/{did}/verify-signature - Verify Ed25519 signature
#[post("/identity/{did}/verify-signature")]
async fn verify_signature(
    _req: HttpRequest,
    path: web::Path<String>,
    _body: web::Json<VerifySignatureRequest>,
) -> ApiResult<HttpResponse> {
    let _did = path.into_inner();
    let trace_id = uuid::Uuid::new_v4().to_string();

    // TODO: Lookup DID document + key history
    // TODO: Verify Ed25519 signature using current or historical key
    
    let response = VerifySignatureResponse {
        valid: true,
        key_index: 0,
        verified_at: Utc::now(),
    };

    let api_response = crate::api::errors::ApiResponse::success(response, trace_id);
    Ok(HttpResponse::Ok().json(api_response))
}

// ── Store-backed identity endpoints ──────────────────────────────────────────

/// POST /api/v1/store/identities — persiste un IdentityRecord en el store.
#[post("/store/identities")]
pub async fn store_write_identity(
    state: web::Data<AppState>,
    body: web::Json<crate::storage::traits::IdentityRecord>,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    let trace_id = uuid::Uuid::new_v4().to_string();
    let store = get_channel_store(&state, channel_id_from_req(&req))?;
    store
        .write_identity(&body)
        .map_err(|e| ApiError::StorageError { reason: e.to_string() })?;
    Ok(HttpResponse::Created().json(ApiResponse::success(body.into_inner(), trace_id)))
}

/// GET /api/v1/store/identities/{did} — lee un IdentityRecord del store.
#[get("/store/identities/{did}")]
pub async fn store_get_identity(
    state: web::Data<AppState>,
    path: web::Path<String>,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    let did = path.into_inner();
    let trace_id = uuid::Uuid::new_v4().to_string();
    let store = get_channel_store(&state, channel_id_from_req(&req))?;
    match store.read_identity(&did) {
        Ok(identity) => Ok(HttpResponse::Ok().json(ApiResponse::success(identity, trace_id))),
        Err(_) => Err(ApiError::NotFound { resource: format!("identity {did}") }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_identity_handlers_are_public() {
        let _ = (store_write_identity, store_get_identity);
    }
}
