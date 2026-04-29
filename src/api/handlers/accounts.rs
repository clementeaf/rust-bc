//! HTTP handlers for native cryptocurrency account operations.

use actix_web::{get, post, web, HttpRequest, HttpResponse};
use serde::{Deserialize, Serialize};

use crate::api::errors::{ApiError, ApiResponse, ApiResult};
use crate::app_state::AppState;
use crate::transaction::native::NativeTransaction;

// ── Request / Response types ───────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct AccountResponse {
    pub address: String,
    pub balance: u64,
    pub nonce: u64,
    pub is_contract: bool,
}

#[derive(Debug, Deserialize)]
pub struct TransferRequest {
    pub from: String,
    pub to: String,
    pub amount: u64,
    pub nonce: u64,
    pub fee: u64,
}

#[derive(Debug, Serialize)]
pub struct TransferResponse {
    pub tx_id: String,
    pub from: String,
    pub to: String,
    pub amount: u64,
    pub fee: u64,
    pub nonce: u64,
    pub queued: bool,
}

#[derive(Debug, Serialize)]
pub struct MempoolStatsResponse {
    pub pending: usize,
    pub base_fee: u64,
}

// ── Handlers ───────────────────────────────────────────────────────────────

/// GET /accounts/{address} — query account balance and nonce.
#[get("/accounts/{address}")]
async fn get_account(
    _req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> ApiResult<HttpResponse> {
    let trace_id = uuid::Uuid::new_v4().to_string();
    let address = path.into_inner();

    let account_store = state.account_store.as_ref().ok_or(ApiError::NotFound {
        resource: "account_store not configured".to_string(),
    })?;

    let acc = account_store
        .get_account(&address)
        .map_err(|e| ApiError::InternalError {
            reason: format!("failed to get account: {e}"),
        })?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(
        AccountResponse {
            address,
            balance: acc.balance,
            nonce: acc.nonce,
            is_contract: acc.is_contract(),
        },
        trace_id,
    )))
}

/// POST /transfer — submit a native token transfer to the mempool.
#[post("/transfer")]
async fn submit_transfer(
    _req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<TransferRequest>,
) -> ApiResult<HttpResponse> {
    let trace_id = uuid::Uuid::new_v4().to_string();

    if body.from.is_empty() || body.to.is_empty() {
        return Err(ApiError::ValidationError {
            field: "from/to".to_string(),
            reason: "addresses must not be empty".to_string(),
        });
    }
    if body.amount == 0 {
        return Err(ApiError::ValidationError {
            field: "amount".to_string(),
            reason: "amount must be greater than zero".to_string(),
        });
    }

    let mempool = state.native_mempool.as_ref().ok_or(ApiError::NotFound {
        resource: "native mempool not configured".to_string(),
    })?;

    // Validate fee against current base fee
    let base_fee = {
        let econ = state
            .economics_state
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        econ.base_fee
    };
    if body.fee < base_fee {
        return Err(ApiError::ValidationError {
            field: "fee".to_string(),
            reason: format!("fee {} below current base fee {base_fee}", body.fee),
        });
    }

    let tx =
        NativeTransaction::new_transfer(&body.from, &body.to, body.amount, body.nonce, body.fee);
    let tx_id = tx.id.clone();

    match mempool.add(tx) {
        Ok(true) => Ok(HttpResponse::Created().json(ApiResponse::success_with_code(
            TransferResponse {
                tx_id,
                from: body.from.clone(),
                to: body.to.clone(),
                amount: body.amount,
                fee: body.fee,
                nonce: body.nonce,
                queued: true,
            },
            201,
            trace_id,
        ))),
        Ok(false) => Err(ApiError::Conflict {
            reason: "transaction already known".to_string(),
        }),
        Err(e) => Err(ApiError::ValidationError {
            field: "mempool".to_string(),
            reason: e.to_string(),
        }),
    }
}

/// GET /mempool/stats — mempool size and current base fee.
#[get("/mempool/stats")]
async fn mempool_stats(_req: HttpRequest, state: web::Data<AppState>) -> ApiResult<HttpResponse> {
    let trace_id = uuid::Uuid::new_v4().to_string();

    let pending = state.native_mempool.as_ref().map(|m| m.len()).unwrap_or(0);

    let base_fee = {
        let econ = state
            .economics_state
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        econ.base_fee
    };

    Ok(HttpResponse::Ok().json(ApiResponse::success(
        MempoolStatsResponse { pending, base_fee },
        trace_id,
    )))
}
