use actix_web::{web, HttpRequest, HttpResponse, post, get};
use crate::api::errors::{ApiError, ApiResponse, ApiResult};
use crate::api::models::*;
use crate::app_state::AppState;
use chrono::Utc;

/// POST /credentials/issue - Issue a credential
#[post("/credentials/issue")]
async fn issue_credential(
    _req: HttpRequest,
    body: web::Json<IssueCredentialRequest>,
) -> ApiResult<HttpResponse> {
    let trace_id = uuid::Uuid::new_v4().to_string();
    
    // TODO: Generate Ed25519 proof
    // TODO: Create VerifiableCredential
    // TODO: Store in blockchain
    
    let response = IssueCredentialResponse {
        credential_id: uuid::Uuid::new_v4().to_string(),
        issuer_did: body.issuer_did.clone(),
        subject_did: body.subject_did.clone(),
        issued_at: Utc::now(),
        proof: ProofResponse {
            verification_method: format!("{}#key-0", body.issuer_did),
            signature_value: "signature_placeholder".to_string(),
            created: Utc::now(),
        },
    };

    let api_response = crate::api::errors::ApiResponse::success(response, trace_id);
    Ok(HttpResponse::Ok().json(api_response))
}

/// GET /credentials/{id} - Fetch credential
#[get("/credentials/{id}")]
async fn get_credential(
    _req: HttpRequest,
    path: web::Path<String>,
) -> ApiResult<HttpResponse> {
    let id = path.into_inner();
    let trace_id = uuid::Uuid::new_v4().to_string();
    
    // TODO: Query storage for credential
    
    let response = CredentialResponse {
        id,
        issuer_did: "did:bc:issuer".to_string(),
        subject_did: "did:bc:subject".to_string(),
        claims: serde_json::json!({}),
        issued_at: Utc::now(),
        expires_at: None,
        proof: ProofResponse {
            verification_method: "did:bc:issuer#key-0".to_string(),
            signature_value: "signature_placeholder".to_string(),
            created: Utc::now(),
        },
    };

    let api_response = crate::api::errors::ApiResponse::success(response, trace_id);
    Ok(HttpResponse::Ok().json(api_response))
}

/// POST /credentials/{id}/verify - Verify credential
#[post("/credentials/{id}/verify")]
async fn verify_credential(
    _req: HttpRequest,
    path: web::Path<String>,
    _body: web::Json<VerifyCredentialRequest>,
) -> ApiResult<HttpResponse> {
    let _id = path.into_inner();
    let trace_id = uuid::Uuid::new_v4().to_string();
    
    // TODO: Lookup credential + issuer DID
    // TODO: Verify Ed25519 signature + timestamp + expiration
    
    let response = VerifyCredentialResponse {
        valid: true,
        issuer_did: "did:bc:issuer".to_string(),
        subject_did: "did:bc:subject".to_string(),
        verified_at: Utc::now(),
    };

    let api_response = crate::api::errors::ApiResponse::success(response, trace_id);
    Ok(HttpResponse::Ok().json(api_response))
}

/// POST /credentials/{id}/revoke - Revoke credential
#[post("/credentials/{id}/revoke")]
async fn revoke_credential(
    _req: HttpRequest,
    path: web::Path<String>,
    _body: web::Json<RevokeCredentialRequest>,
) -> ApiResult<HttpResponse> {
    let id = path.into_inner();
    let trace_id = uuid::Uuid::new_v4().to_string();
    
    // TODO: Mark in revocation registry
    
    let response = RevokeCredentialResponse {
        credential_id: id,
        revoked: true,
        revoked_at: Utc::now(),
    };

    let api_response = crate::api::errors::ApiResponse::success(response, trace_id);
    Ok(HttpResponse::Ok().json(api_response))
}

// ── Store-backed credential endpoints ────────────────────────────────────────

/// POST /api/v1/store/credentials — persiste un Credential en el store.
#[post("/store/credentials")]
pub async fn store_write_credential(
    state: web::Data<AppState>,
    body: web::Json<crate::storage::traits::Credential>,
) -> ApiResult<HttpResponse> {
    let trace_id = uuid::Uuid::new_v4().to_string();
    match &state.store {
        None => Err(ApiError::NotFound { resource: "store".to_string() }),
        Some(store) => {
            store
                .write_credential(&body)
                .map_err(|e| ApiError::StorageError { reason: e.to_string() })?;
            Ok(HttpResponse::Created().json(ApiResponse::success(body.into_inner(), trace_id)))
        }
    }
}

/// GET /api/v1/store/credentials/{cred_id} — lee un Credential del store.
#[get("/store/credentials/{cred_id}")]
pub async fn store_get_credential(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> ApiResult<HttpResponse> {
    let cred_id = path.into_inner();
    let trace_id = uuid::Uuid::new_v4().to_string();
    match &state.store {
        None => Err(ApiError::NotFound { resource: "store".to_string() }),
        Some(store) => match store.read_credential(&cred_id) {
            Ok(cred) => Ok(HttpResponse::Ok().json(ApiResponse::success(cred, trace_id))),
            Err(_) => Err(ApiError::NotFound { resource: format!("credential {cred_id}") }),
        },
    }
}

/// GET /api/v1/store/credentials/by-subject/{subject_did} — devuelve todos los
/// Credentials cuyo `subject_did` coincide con el parámetro de ruta.
#[get("/store/credentials/by-subject/{subject_did}")]
pub async fn store_get_credentials_by_subject(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> ApiResult<HttpResponse> {
    let subject_did = path.into_inner();
    let trace_id = uuid::Uuid::new_v4().to_string();
    match &state.store {
        None => Err(ApiError::NotFound { resource: "store".to_string() }),
        Some(store) => {
            let creds = store
                .credentials_by_subject_did(&subject_did)
                .map_err(|e| ApiError::StorageError { reason: e.to_string() })?;
            Ok(HttpResponse::Ok().json(ApiResponse::success(creds, trace_id)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_credential_handlers_are_public() {
        let _ = (store_write_credential, store_get_credential);
    }
}
