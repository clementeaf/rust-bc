//! HTTP handlers for PIN generation, assignment, and verification.

use actix_web::{post, web, HttpRequest, HttpResponse};
use serde::{Deserialize, Serialize};

use crate::api::errors::{ApiError, ApiResponse, ApiResult};
use crate::app_state::AppState;
use crate::pin::generator::Pin;

#[derive(Debug, Deserialize)]
pub struct GeneratePinRequest {
    pub did: String,
    #[serde(default = "default_length")]
    pub length: u8,
}

fn default_length() -> u8 {
    4
}

#[derive(Debug, Serialize)]
pub struct GeneratePinResponse {
    pub did: String,
    pub pin: String,
}

#[derive(Debug, Deserialize)]
pub struct VerifyPinRequest {
    pub did: String,
    pub pin: String,
}

#[derive(Debug, Serialize)]
pub struct VerifyPinResponse {
    pub did: String,
    pub valid: bool,
}

/// POST /pin/generate — generate a PIN, hash it, associate with DID, return plaintext once.
#[post("/pin/generate")]
async fn generate_pin(
    _req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<GeneratePinRequest>,
) -> ApiResult<HttpResponse> {
    let trace_id = uuid::Uuid::new_v4().to_string();

    if body.did.is_empty() {
        return Err(ApiError::ValidationError {
            field: "did".to_string(),
            reason: "DID is required".to_string(),
        });
    }

    let pin_store = state
        .pin_store
        .as_ref()
        .ok_or_else(|| ApiError::InternalError {
            reason: "PIN store not configured".to_string(),
        })?;

    let pin = Pin::generate(body.length).map_err(|e| ApiError::ValidationError {
        field: "length".to_string(),
        reason: e.to_string(),
    })?;

    let hash = pin.hash().map_err(|e| ApiError::InternalError {
        reason: e.to_string(),
    })?;

    pin_store
        .set(&body.did, &hash)
        .map_err(|e| ApiError::InternalError {
            reason: e.to_string(),
        })?;

    let response = GeneratePinResponse {
        did: body.did.clone(),
        pin: pin.as_str().to_string(),
    };

    Ok(HttpResponse::Ok().json(ApiResponse::success(response, trace_id)))
}

/// POST /pin/verify — verify a plaintext PIN against stored hash for a DID.
#[post("/pin/verify")]
async fn verify_pin(
    _req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<VerifyPinRequest>,
) -> ApiResult<HttpResponse> {
    let trace_id = uuid::Uuid::new_v4().to_string();

    if body.did.is_empty() || body.pin.is_empty() {
        return Err(ApiError::ValidationError {
            field: "did/pin".to_string(),
            reason: "DID and PIN are required".to_string(),
        });
    }

    let pin_store = state
        .pin_store
        .as_ref()
        .ok_or_else(|| ApiError::InternalError {
            reason: "PIN store not configured".to_string(),
        })?;

    let valid = pin_store.verify(&body.did, &body.pin).is_ok();

    let response = VerifyPinResponse {
        did: body.did.clone(),
        valid,
    };

    Ok(HttpResponse::Ok().json(ApiResponse::success(response, trace_id)))
}
