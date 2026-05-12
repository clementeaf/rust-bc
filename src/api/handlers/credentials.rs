use crate::api::errors::{ApiError, ApiResponse, ApiResult};
use crate::api::handlers::channels::{
    channel_id_from_req, enforce_channel_membership, get_channel_store,
};
use crate::api::models::*;
use crate::app_state::AppState;
use actix_web::{get, post, web, HttpRequest, HttpResponse};
use chrono::Utc;

/// POST /credentials/issue - Issue a credential and persist to store.
#[post("/credentials/issue")]
async fn issue_credential(
    state: web::Data<AppState>,
    body: web::Json<IssueCredentialRequest>,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    let trace_id = uuid::Uuid::new_v4().to_string();
    let credential_id = uuid::Uuid::new_v4().to_string();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Validate issuer DID exists
    let _channel = channel_id_from_req(&req);
    let store = get_channel_store(&state, _channel)?;
    if store.read_identity(&body.issuer_did).is_err() {
        return Err(ApiError::ValidationError {
            field: "issuer_did".to_string(),
            reason: format!(
                "issuer DID '{}' not found — register identity before issuing credentials",
                body.issuer_did
            ),
        });
    }

    // Persist to store
    let record = crate::storage::traits::Credential {
        id: credential_id.clone(),
        issuer_did: body.issuer_did.clone(),
        subject_did: body.subject_did.clone(),
        cred_type: "VerifiableCredential".to_string(),
        issued_at: now,
        expires_at: body.expires_at.map(|dt| dt.timestamp() as u64).unwrap_or(0),
        revoked_at: None,
        claims: body.claims.clone(),
        signature: String::new(),
        status: "active".to_string(),
    };
    store
        .write_credential(&record)
        .map_err(|e| ApiError::StorageError {
            reason: e.to_string(),
        })?;

    crate::audit::emit_if_present(
        &state.audit_store,
        crate::audit::AuditAction::CredentialStored,
        req.headers()
            .get("X-Org-Id")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown"),
        Some(format!("credential_id={credential_id}")),
    );

    let response = IssueCredentialResponse {
        credential_id,
        issuer_did: body.issuer_did.clone(),
        subject_did: body.subject_did.clone(),
        issued_at: Utc::now(),
        proof: ProofResponse {
            verification_method: format!("{}#key-0", body.issuer_did),
            signature_value: String::new(),
            created: Utc::now(),
        },
    };
    Ok(HttpResponse::Created().json(ApiResponse::success(response, trace_id)))
}

/// GET /credentials/{id} - Fetch credential from store.
#[get("/credentials/{id}")]
async fn get_credential(
    state: web::Data<AppState>,
    path: web::Path<String>,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    let id = path.into_inner();
    let trace_id = uuid::Uuid::new_v4().to_string();
    let _channel = channel_id_from_req(&req);
    let store = get_channel_store(&state, _channel)?;

    let cred = store.read_credential(&id).map_err(|_| ApiError::NotFound {
        resource: format!("credential {id}"),
    })?;

    let response = CredentialResponse {
        id: cred.id,
        issuer_did: cred.issuer_did,
        subject_did: cred.subject_did,
        claims: cred.claims,
        issued_at: chrono::DateTime::from_timestamp(cred.issued_at as i64, 0)
            .unwrap_or_else(Utc::now),
        expires_at: if cred.expires_at > 0 {
            chrono::DateTime::from_timestamp(cred.expires_at as i64, 0)
        } else {
            None
        },
        proof: ProofResponse {
            verification_method: String::new(),
            signature_value: cred.signature,
            created: Utc::now(),
        },
    };
    Ok(HttpResponse::Ok().json(ApiResponse::success(response, trace_id)))
}

/// POST /credentials/{id}/verify - Verify credential (check status + expiry).
#[post("/credentials/{id}/verify")]
async fn verify_credential(
    state: web::Data<AppState>,
    path: web::Path<String>,
    _body: web::Json<VerifyCredentialRequest>,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    let id = path.into_inner();
    let trace_id = uuid::Uuid::new_v4().to_string();
    let _channel = channel_id_from_req(&req);
    let store = get_channel_store(&state, _channel)?;

    let cred = store.read_credential(&id).map_err(|_| ApiError::NotFound {
        resource: format!("credential {id}"),
    })?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let valid = cred.status == "active"
        && cred.revoked_at.is_none()
        && (cred.expires_at == 0 || now <= cred.expires_at);

    let response = VerifyCredentialResponse {
        valid,
        issuer_did: cred.issuer_did,
        subject_did: cred.subject_did,
        verified_at: Utc::now(),
    };
    Ok(HttpResponse::Ok().json(ApiResponse::success(response, trace_id)))
}

/// POST /credentials/{id}/revoke - Revoke credential in store.
#[post("/credentials/{id}/revoke")]
async fn revoke_credential(
    state: web::Data<AppState>,
    path: web::Path<String>,
    _body: web::Json<RevokeCredentialRequest>,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    let id = path.into_inner();
    let trace_id = uuid::Uuid::new_v4().to_string();
    let _channel = channel_id_from_req(&req);
    let store = get_channel_store(&state, _channel)?;

    let mut cred = store.read_credential(&id).map_err(|_| ApiError::NotFound {
        resource: format!("credential {id}"),
    })?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    cred.revoked_at = Some(now);
    cred.status = "revoked".to_string();
    store
        .write_credential(&cred)
        .map_err(|e| ApiError::StorageError {
            reason: e.to_string(),
        })?;

    crate::audit::emit_if_present(
        &state.audit_store,
        crate::audit::AuditAction::CredentialRevoked,
        req.headers()
            .get("X-Org-Id")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown"),
        Some(format!("credential_id={id}")),
    );

    let response = RevokeCredentialResponse {
        credential_id: id,
        revoked: true,
        revoked_at: Utc::now(),
    };
    Ok(HttpResponse::Ok().json(ApiResponse::success(response, trace_id)))
}

// ── Store-backed credential endpoints ────────────────────────────────────────

/// POST /api/v1/store/credentials — persiste un Credential en el store.
#[post("/store/credentials")]
pub async fn store_write_credential(
    state: web::Data<AppState>,
    body: web::Json<crate::storage::traits::Credential>,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    super::validation::validate_store_credential(&body)?;
    let trace_id = uuid::Uuid::new_v4().to_string();
    let _channel = channel_id_from_req(&req);
    enforce_channel_membership(&state, _channel, &req)?;
    let store = get_channel_store(&state, _channel)?;

    // Validate issuer DID exists
    if store.read_identity(&body.issuer_did).is_err() {
        return Err(ApiError::ValidationError {
            field: "issuer_did".to_string(),
            reason: format!(
                "issuer DID '{}' not found — register identity before issuing credentials",
                body.issuer_did
            ),
        });
    }

    store
        .write_credential(&body)
        .map_err(|e| ApiError::StorageError {
            reason: e.to_string(),
        })?;
    crate::audit::emit_if_present(
        &state.audit_store,
        crate::audit::AuditAction::CredentialStored,
        req.headers()
            .get("X-Org-Id")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown"),
        Some(format!("credential_id={}", body.id)),
    );
    Ok(HttpResponse::Created().json(ApiResponse::success(body.into_inner(), trace_id)))
}

/// GET /api/v1/store/credentials/{cred_id} — lee un Credential del store.
#[get("/store/credentials/{cred_id}")]
pub async fn store_get_credential(
    state: web::Data<AppState>,
    path: web::Path<String>,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    let cred_id = path.into_inner();
    let trace_id = uuid::Uuid::new_v4().to_string();
    let _channel = channel_id_from_req(&req);
    enforce_channel_membership(&state, _channel, &req)?;
    let store = get_channel_store(&state, _channel)?;
    match store.read_credential(&cred_id) {
        Ok(cred) => Ok(HttpResponse::Ok().json(ApiResponse::success(cred, trace_id))),
        Err(_) => Err(ApiError::NotFound {
            resource: format!("credential {cred_id}"),
        }),
    }
}

/// GET /api/v1/store/credentials/by-subject/{subject_did}
#[get("/store/credentials/by-subject/{subject_did}")]
pub async fn store_get_credentials_by_subject(
    state: web::Data<AppState>,
    path: web::Path<String>,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    let subject_did = path.into_inner();
    let trace_id = uuid::Uuid::new_v4().to_string();
    let _channel = channel_id_from_req(&req);
    enforce_channel_membership(&state, _channel, &req)?;
    let store = get_channel_store(&state, _channel)?;
    let creds = store
        .credentials_by_subject_did(&subject_did)
        .map_err(|e| ApiError::StorageError {
            reason: e.to_string(),
        })?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(creds, trace_id)))
}

/// GET /api/v1/store/credentials/by-issuer/{issuer_did}
#[get("/store/credentials/by-issuer/{issuer_did}")]
pub async fn store_get_credentials_by_issuer(
    state: web::Data<AppState>,
    path: web::Path<String>,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    let issuer_did = path.into_inner();
    let trace_id = uuid::Uuid::new_v4().to_string();
    let _channel = channel_id_from_req(&req);
    enforce_channel_membership(&state, _channel, &req)?;
    let store = get_channel_store(&state, _channel)?;
    let creds =
        store
            .credentials_by_issuer_did(&issuer_did)
            .map_err(|e| ApiError::StorageError {
                reason: e.to_string(),
            })?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(creds, trace_id)))
}

/// GET /api/v1/store/credentials?limit=100&offset=0 — list credentials with pagination.
#[get("/store/credentials")]
pub async fn store_list_credentials(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<super::identity::PaginationQuery>,
) -> ApiResult<HttpResponse> {
    let trace_id = uuid::Uuid::new_v4().to_string();
    let _channel = channel_id_from_req(&req);
    enforce_channel_membership(&state, _channel, &req)?;
    let store = get_channel_store(&state, _channel)?;
    let all = store
        .list_credentials()
        .map_err(|e| ApiError::StorageError {
            reason: e.to_string(),
        })?;
    let limit = query.limit.unwrap_or(100).min(1000);
    let offset = query.offset.unwrap_or(0);
    let page: Vec<_> = all.into_iter().skip(offset).take(limit).collect();
    Ok(HttpResponse::Ok().json(ApiResponse::success(page, trace_id)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_credential_handlers_are_public() {
        let _ = (store_write_credential, store_get_credential);
    }
}
