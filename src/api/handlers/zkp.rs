//! ZKP identity endpoints:
//!   POST /api/v1/identity/zkp/prove   — generate a zero-knowledge proof
//!   POST /api/v1/identity/zkp/verify  — verify a zero-knowledge presentation

use actix_web::{post, web, HttpRequest, HttpResponse};
use serde::Deserialize;

use crate::api::errors::{ApiError, ApiResponse, ApiResult};
use crate::app_state::AppState;
use crate::identity::zkp::{self, Predicate, ZkPresentation, ZkpError};

#[derive(Deserialize)]
pub struct ProveRequest {
    pub credential_id: String,
    pub predicate: Predicate,
    /// For range proofs: the actual numeric value of the claim.
    pub claim_value_numeric: Option<u64>,
    /// For set membership: the actual string value of the claim.
    pub claim_value_string: Option<String>,
    /// For credential validity: status, expires_at, revoked_at.
    pub credential_status: Option<String>,
    pub credential_expires_at: Option<u64>,
    pub credential_revoked_at: Option<u64>,
}

/// POST /api/v1/identity/zkp/prove — generate a ZK proof for a predicate.
#[post("/identity/zkp/prove")]
pub async fn prove_zkp(
    state: web::Data<AppState>,
    body: web::Json<ProveRequest>,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    let trace_id = uuid::Uuid::new_v4().to_string();

    let result: Result<ZkPresentation, ZkpError> = match &body.predicate {
        Predicate::RangeProof {
            claim_key,
            threshold,
        } => {
            let value = body.claim_value_numeric.ok_or(ApiError::ValidationError {
                field: "claim_value_numeric".to_string(),
                reason: "required for range_proof".to_string(),
            })?;
            zkp::prove_range(&body.credential_id, claim_key, value, *threshold)
        }
        Predicate::SetMembership {
            claim_key,
            allowed_values,
        } => {
            let value = body
                .claim_value_string
                .as_deref()
                .ok_or(ApiError::ValidationError {
                    field: "claim_value_string".to_string(),
                    reason: "required for set_membership".to_string(),
                })?;
            zkp::prove_set_membership(&body.credential_id, claim_key, value, allowed_values)
        }
        Predicate::CredentialValidity { .. } => {
            let status = body.credential_status.as_deref().unwrap_or("active");
            let expires = body.credential_expires_at.unwrap_or(0);
            let revoked = body.credential_revoked_at;
            zkp::prove_credential_validity(&body.credential_id, status, expires, revoked)
        }
    };

    match result {
        Ok(presentation) => {
            // Audit: log proof generation (no claim data, only result)
            crate::audit::emit_if_present(
                &state.audit_store,
                crate::audit::AuditAction::CredentialStored, // Reuse closest
                req.headers()
                    .get("X-Org-Id")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("unknown"),
                Some(format!(
                    "zkp_prove,credential={},predicate_type={}",
                    body.credential_id,
                    predicate_type_name(&body.predicate)
                )),
            );
            Ok(HttpResponse::Ok().json(ApiResponse::success(presentation, trace_id)))
        }
        Err(ZkpError::PredicateNotSatisfied(reason)) => Err(ApiError::ValidationError {
            field: "predicate".to_string(),
            reason,
        }),
        Err(e) => Err(ApiError::StorageError {
            reason: e.to_string(),
        }),
    }
}

/// POST /api/v1/identity/zkp/verify — verify a ZK presentation.
#[post("/identity/zkp/verify")]
pub async fn verify_zkp(
    state: web::Data<AppState>,
    body: web::Json<ZkPresentation>,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    let trace_id = uuid::Uuid::new_v4().to_string();

    let valid = zkp::verify_presentation(&body).map_err(|e| ApiError::ValidationError {
        field: "proof".to_string(),
        reason: e.to_string(),
    })?;

    // Audit: log verification result (no claim data)
    crate::audit::emit_if_present(
        &state.audit_store,
        crate::audit::AuditAction::CredentialStored,
        req.headers()
            .get("X-Org-Id")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown"),
        Some(format!(
            "zkp_verify,credential={},valid={valid}",
            body.credential_id
        )),
    );

    #[derive(serde::Serialize)]
    struct VerifyResult {
        valid: bool,
        credential_id: String,
    }

    Ok(HttpResponse::Ok().json(ApiResponse::success(
        VerifyResult {
            valid,
            credential_id: body.credential_id.clone(),
        },
        trace_id,
    )))
}

fn predicate_type_name(p: &Predicate) -> &'static str {
    match p {
        Predicate::RangeProof { .. } => "range_proof",
        Predicate::SetMembership { .. } => "set_membership",
        Predicate::CredentialValidity { .. } => "credential_validity",
    }
}
