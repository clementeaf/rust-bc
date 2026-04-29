//! HTTP handler for testnet faucet — drip NOTA tokens to test addresses.

use actix_web::{get, post, web, HttpRequest, HttpResponse};
use serde::{Deserialize, Serialize};

use crate::api::errors::{ApiError, ApiResponse, ApiResult};
use crate::app_state::AppState;

#[derive(Debug, Deserialize)]
pub struct FaucetRequest {
    pub address: String,
}

#[derive(Debug, Serialize)]
pub struct FaucetResponse {
    pub address: String,
    pub amount: u64,
    pub new_balance: u64,
}

/// POST /faucet/drip — request tokens from the testnet faucet.
#[post("/faucet/drip")]
async fn faucet_drip(
    _req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<FaucetRequest>,
) -> ApiResult<HttpResponse> {
    let trace_id = uuid::Uuid::new_v4().to_string();

    if body.address.is_empty() {
        return Err(ApiError::ValidationError {
            field: "address".to_string(),
            reason: "address must not be empty".to_string(),
        });
    }

    let account_store = state.account_store.as_ref().ok_or(ApiError::NotFound {
        resource: "account store not configured".to_string(),
    })?;

    // Get current block height for cooldown check
    let current_height = {
        let econ = state
            .economics_state
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        econ.height
    };

    // Use faucet if available, otherwise just credit directly (devnet mode)
    let drip_amount = if let Some(ref faucet) = state.faucet {
        match faucet.drip(&body.address, current_height) {
            Ok(result) => result.amount,
            Err(e) => {
                return Err(ApiError::ValidationError {
                    field: "faucet".to_string(),
                    reason: e.to_string(),
                });
            }
        }
    } else {
        // No faucet configured — default drip of 1000
        1_000
    };

    // Credit account
    let acc = account_store
        .credit(&body.address, drip_amount)
        .map_err(|e| ApiError::InternalError {
            reason: format!("failed to credit account: {e}"),
        })?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(
        FaucetResponse {
            address: body.address.clone(),
            amount: drip_amount,
            new_balance: acc.balance,
        },
        trace_id,
    )))
}

/// GET /faucet/status — faucet availability info.
#[get("/faucet/status")]
async fn faucet_status(_req: HttpRequest, state: web::Data<AppState>) -> ApiResult<HttpResponse> {
    let trace_id = uuid::Uuid::new_v4().to_string();

    #[derive(Serialize)]
    struct FaucetStatus {
        enabled: bool,
        drip_amount: u64,
    }

    let status = if let Some(ref faucet) = state.faucet {
        FaucetStatus {
            enabled: true,
            drip_amount: faucet.config().drip_amount,
        }
    } else {
        FaucetStatus {
            enabled: false,
            drip_amount: 0,
        }
    };

    Ok(HttpResponse::Ok().json(ApiResponse::success(status, trace_id)))
}
