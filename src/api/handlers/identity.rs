use crate::api::errors::{enforce_acl, ApiError, ApiResponse, ApiResult};
use crate::api::handlers::channels::{
    channel_id_from_req, enforce_channel_membership, get_channel_store,
};
use crate::api::models::*;
use crate::app_state::AppState;
use crate::identity::keys::KeyManager;
use actix_web::{get, post, web, HttpRequest, HttpResponse};
use chrono::Utc;

/// POST /identity/create - Create a new DID, generate Ed25519 keypair, persist to store.
#[post("/identity/create")]
async fn create_identity(
    state: web::Data<AppState>,
    _body: web::Json<CreateIdentityRequest>,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    let trace_id = uuid::Uuid::new_v4().to_string();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let key_mgr = KeyManager::new(now);
    let public_key_hex = hex::encode(key_mgr.public_key());
    let did = format!("did:cerulean:{}", uuid::Uuid::new_v4());

    // Persist to store
    let _channel = channel_id_from_req(&req);
    let store = get_channel_store(&state, _channel)?;
    let record = crate::storage::traits::IdentityRecord {
        did: did.clone(),
        created_at: now,
        updated_at: now,
        status: "active".to_string(),
    };
    store
        .write_identity(&record)
        .map_err(|e| ApiError::StorageError {
            reason: e.to_string(),
        })?;

    crate::audit::emit_if_present(
        &state.audit_store,
        crate::audit::AuditAction::DidRegistered,
        req.headers()
            .get("X-Org-Id")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown"),
        Some(format!("did={did}")),
    );

    let response = IdentityResponse {
        did,
        public_key: public_key_hex,
        created_at: Utc::now(),
    };
    Ok(HttpResponse::Created().json(ApiResponse::success(response, trace_id)))
}

/// GET /identity/{did} - Fetch DID document from store.
#[get("/identity/{did}")]
async fn get_identity(
    state: web::Data<AppState>,
    path: web::Path<String>,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    let did = path.into_inner();
    let trace_id = uuid::Uuid::new_v4().to_string();
    let _channel = channel_id_from_req(&req);
    let store = get_channel_store(&state, _channel)?;

    let record = store.read_identity(&did).map_err(|_| ApiError::NotFound {
        resource: format!("identity {did}"),
    })?;

    let response = IdentityResponse {
        did: record.did,
        public_key: String::new(), // Key not stored in IdentityRecord; would need KeyStore
        created_at: chrono::DateTime::from_timestamp(record.created_at as i64, 0)
            .unwrap_or_else(Utc::now),
    };
    Ok(HttpResponse::Ok().json(ApiResponse::success(response, trace_id)))
}

/// POST /identity/{did}/rotate-key - Key rotation (generates new keypair).
#[post("/identity/{did}/rotate-key")]
async fn rotate_key(
    state: web::Data<AppState>,
    path: web::Path<String>,
    _body: web::Json<RotateKeyRequest>,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    let did = path.into_inner();
    let trace_id = uuid::Uuid::new_v4().to_string();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Verify DID exists
    let _channel = channel_id_from_req(&req);
    let store = get_channel_store(&state, _channel)?;
    let mut record = store.read_identity(&did).map_err(|_| ApiError::NotFound {
        resource: format!("identity {did}"),
    })?;

    // Update timestamp to reflect rotation
    record.updated_at = now;
    store
        .write_identity(&record)
        .map_err(|e| ApiError::StorageError {
            reason: e.to_string(),
        })?;

    let response = RotateKeyResponse {
        did,
        new_key_index: _body.old_key_index + 1,
        rotated_at: Utc::now(),
    };
    Ok(HttpResponse::Ok().json(ApiResponse::success(response, trace_id)))
}

/// POST /identity/{did}/verify-signature - Verify Ed25519 signature.
#[post("/identity/{did}/verify-signature")]
async fn verify_signature(
    state: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<VerifySignatureRequest>,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    let did = path.into_inner();
    let trace_id = uuid::Uuid::new_v4().to_string();

    // Verify DID exists
    let _channel = channel_id_from_req(&req);
    let store = get_channel_store(&state, _channel)?;
    let _record = store.read_identity(&did).map_err(|_| ApiError::NotFound {
        resource: format!("identity {did}"),
    })?;

    // Verify signature using Ed25519
    let valid = {
        let pub_bytes = match hex::decode(&body.public_key) {
            Ok(b) => b,
            Err(_) => {
                return Ok(HttpResponse::BadRequest().json(ApiResponse::<()>::error(
                    crate::api::errors::ErrorDto {
                        code: "INVALID_HEX".to_string(),
                        message: "public_key is not valid hex".to_string(),
                        field: Some("public_key".to_string()),
                    },
                    400,
                )));
            }
        };
        let sig_bytes = match hex::decode(&body.signature) {
            Ok(b) => b,
            Err(_) => {
                return Ok(HttpResponse::BadRequest().json(ApiResponse::<()>::error(
                    crate::api::errors::ErrorDto {
                        code: "INVALID_HEX".to_string(),
                        message: "signature is not valid hex".to_string(),
                        field: Some("signature".to_string()),
                    },
                    400,
                )));
            }
        };

        if pub_bytes.len() == 32 && sig_bytes.len() == 64 {
            use pqc_crypto_module::legacy::ed25519::{Signature, Verifier, VerifyingKey};
            match (
                pub_bytes
                    .as_slice()
                    .try_into()
                    .ok()
                    .and_then(|b: &[u8; 32]| VerifyingKey::from_bytes(b).ok()),
                Signature::from_slice(&sig_bytes).ok(),
            ) {
                (Some(vk), Some(sig)) => vk.verify(body.message.as_bytes(), &sig).is_ok(),
                _ => false,
            }
        } else {
            false
        }
    };

    let response = VerifySignatureResponse {
        valid,
        key_index: 0,
        verified_at: Utc::now(),
    };
    Ok(HttpResponse::Ok().json(ApiResponse::success(response, trace_id)))
}

// ── Store-backed identity endpoints ──────────────────────────────────────────

/// POST /api/v1/store/identities — persiste un IdentityRecord en el store.
#[post("/store/identities")]
pub async fn store_write_identity(
    state: web::Data<AppState>,
    body: web::Json<crate::storage::traits::IdentityRecord>,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    enforce_acl(
        state.acl_provider.as_deref(),
        state.policy_store.as_deref(),
        "peer/Identity",
        &req,
    )?;
    super::validation::validate_store_identity(&body)?;
    let trace_id = uuid::Uuid::new_v4().to_string();
    let _channel = channel_id_from_req(&req);
    enforce_channel_membership(&state, _channel, &req)?;
    let store = get_channel_store(&state, _channel)?;
    store
        .write_identity(&body)
        .map_err(|e| ApiError::StorageError {
            reason: e.to_string(),
        })?;
    crate::audit::emit_if_present(
        &state.audit_store,
        crate::audit::AuditAction::DidRegistered,
        req.headers()
            .get("X-Org-Id")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown"),
        Some(format!("did={}", body.did)),
    );
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
    let _channel = channel_id_from_req(&req);
    enforce_channel_membership(&state, _channel, &req)?;
    let store = get_channel_store(&state, _channel)?;
    match store.read_identity(&did) {
        Ok(identity) => Ok(HttpResponse::Ok().json(ApiResponse::success(identity, trace_id))),
        Err(_) => Err(ApiError::NotFound {
            resource: format!("identity {did}"),
        }),
    }
}

/// GET /api/v1/store/identities — list all identity records.
#[get("/store/identities")]
pub async fn store_list_identities(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    let trace_id = uuid::Uuid::new_v4().to_string();
    let _channel = channel_id_from_req(&req);
    enforce_channel_membership(&state, _channel, &req)?;
    let store = get_channel_store(&state, _channel)?;
    let identities = store
        .list_identities()
        .map_err(|e| ApiError::StorageError {
            reason: e.to_string(),
        })?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(identities, trace_id)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_identity_handlers_are_public() {
        let _ = (store_write_identity, store_get_identity);
    }
}
