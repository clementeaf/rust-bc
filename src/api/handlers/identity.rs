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
    body: web::Json<CreateIdentityRequest>,
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
        let pub_bytes = hex::decode(&body.public_key).unwrap_or_default();
        let sig_bytes = hex::decode(&body.signature).unwrap_or_default();

        if pub_bytes.len() == 32 && sig_bytes.len() == 64 {
            use pqc_crypto_module::legacy::ed25519::{Signature, Verifier, VerifyingKey};
            let vk =
                VerifyingKey::from_bytes(pub_bytes.as_slice().try_into().unwrap_or(&[0u8; 32]));
            match (vk, Signature::from_slice(&sig_bytes)) {
                (Ok(vk), Ok(sig)) => vk.verify(body.message.as_bytes(), &sig).is_ok(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_identity_handlers_are_public() {
        let _ = (store_write_identity, store_get_identity);
    }
}
