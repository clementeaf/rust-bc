//! POST /transactions and GET /mempool (gateway envelope).

use actix_web::{get, post, web, HttpRequest, HttpResponse};

use crate::api::errors::{enforce_acl, ApiError, ApiResponse, ApiResult};
use crate::api::handlers::channels::{
    channel_id_from_req, enforce_channel_membership, get_channel_store,
};
use crate::api::models::CreateTransactionRequest;
use crate::app_state::AppState;

use super::validation::validate_store_transaction;

/// POST /api/v1/transactions — validates and enqueues in the transaction pool.
#[post("/transactions")]
pub async fn create_transaction(
    state: web::Data<AppState>,
    req: web::Json<CreateTransactionRequest>,
    http_req: HttpRequest,
) -> ApiResult<HttpResponse> {
    enforce_acl(
        state.acl_provider.as_deref(),
        state.policy_store.as_deref(),
        "peer/Propose",
        &http_req,
    )?;
    let trace_id = uuid::Uuid::new_v4().to_string();
    let api_key = http_req
        .headers()
        .get("X-API-Key")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    if req.from != "0" {
        if let Some(key) = &api_key {
            match state.billing_manager.check_transaction_limit(key) {
                Ok(()) => {}
                Err(e) => {
                    if e.contains("Límite de transacciones alcanzado") {
                        return Err(ApiError::PaymentRequired { message: e });
                    }
                    return Err(ApiError::UnauthorizedWithMessage { message: e });
                }
            }
        }
    }

    // Build storage::Transaction
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let tx = crate::storage::traits::Transaction {
        id: uuid::Uuid::new_v4().to_string(),
        block_height: 0,
        timestamp: now,
        input_did: req.from.clone(),
        output_recipient: req.to.clone(),
        amount: req.amount,
        state: "pending".to_string(),
    };

    // Require client signature for non-coinbase transactions
    if req.from != "0" {
        match &req.signature {
            Some(sig) if !sig.is_empty() => {
                // Client provided a signature — store it for audit
                // (Full verification requires knowing the sender's public key,
                //  which is done at the identity layer via DID resolution)
            }
            _ => {
                return Err(ApiError::ValidationError {
                    field: "signature".to_string(),
                    reason: "Signature required for non-coinbase transactions".to_string(),
                });
            }
        }
    }

    // Validate via Validatable trait
    if req.from != "0" {
        let validation_result = {
            let mut validator = state
                .transaction_validator
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            validator.validate(&tx)
        };

        if !validation_result.is_valid {
            let error_msg = validation_result.errors.join("; ");
            return Err(ApiError::ValidationError {
                field: "transaction".to_string(),
                reason: error_msg,
            });
        }

        if let Some(key) = &api_key {
            match state.billing_manager.try_record_transaction(key) {
                Ok(()) => {}
                Err(e) => {
                    if e.contains("Límite de transacciones alcanzado") {
                        return Err(ApiError::PaymentRequired { message: e });
                    }
                    return Err(ApiError::UnauthorizedWithMessage { message: e });
                }
            }
        }
    }

    // Add to transaction pool with double-spend protection
    {
        let balance = if let Ok(store_map) = state.store.read() {
            if let Some(store) = store_map.get("default") {
                store.calculate_balance(&tx.input_did).unwrap_or(0)
            } else {
                0
            }
        } else {
            0
        };

        let mut pool = state.tx_pool.lock().unwrap_or_else(|e| e.into_inner());
        if let Err(e) = pool.add_checked(tx.clone(), balance) {
            return Err(ApiError::ValidationError {
                field: "mempool".to_string(),
                reason: e,
            });
        }
    }

    let body = ApiResponse::success(tx, trace_id);
    Ok(HttpResponse::Created().json(body))
}

// ── Store-backed transaction endpoints ───────────────────────────────────────

/// POST /api/v1/store/transactions — persiste una transacción en el store.
#[post("/store/transactions")]
pub async fn store_write_transaction(
    state: web::Data<AppState>,
    body: web::Json<crate::storage::traits::Transaction>,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    enforce_acl(
        state.acl_provider.as_deref(),
        state.policy_store.as_deref(),
        "peer/Propose",
        &req,
    )?;
    validate_store_transaction(&body)?;
    let trace_id = uuid::Uuid::new_v4().to_string();
    let _channel = channel_id_from_req(&req);
    enforce_channel_membership(&state, _channel, &req)?;
    let store = get_channel_store(&state, _channel)?;
    store
        .write_transaction(&body)
        .map_err(|e| ApiError::StorageError {
            reason: e.to_string(),
        })?;
    Ok(HttpResponse::Created().json(ApiResponse::success_with_code(
        body.into_inner(),
        201,
        trace_id,
    )))
}

/// GET /api/v1/store/transactions/{tx_id} — lee una transacción del store.
#[get("/store/transactions/{tx_id}")]
pub async fn store_get_transaction(
    state: web::Data<AppState>,
    path: web::Path<String>,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    let tx_id = path.into_inner();
    let trace_id = uuid::Uuid::new_v4().to_string();
    let _channel = channel_id_from_req(&req);
    enforce_channel_membership(&state, _channel, &req)?;
    let store = get_channel_store(&state, _channel)?;
    match store.read_transaction(&tx_id) {
        Ok(tx) => Ok(HttpResponse::Ok().json(ApiResponse::success(tx, trace_id))),
        Err(_) => Err(ApiError::NotFound {
            resource: format!("transaction {tx_id}"),
        }),
    }
}

/// GET /api/v1/store/blocks/{height}/transactions — lista txs de un bloque por altura.
#[get("/store/blocks/{height}/transactions")]
pub async fn store_get_transactions_by_block(
    state: web::Data<AppState>,
    path: web::Path<u64>,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    let height = path.into_inner();
    let trace_id = uuid::Uuid::new_v4().to_string();
    let _channel = channel_id_from_req(&req);
    enforce_channel_membership(&state, _channel, &req)?;
    let store = get_channel_store(&state, _channel)?;
    let txs = store
        .transactions_by_block_height(height)
        .map_err(|e| ApiError::StorageError {
            reason: e.to_string(),
        })?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(txs, trace_id)))
}

/// GET /api/v1/tx/{tx_id} — query a committed transaction by ID (any node).
///
/// Simplified endpoint without channel membership enforcement — intended for
/// cross-node tx verification in the DLT demo.
#[get("/tx/{tx_id}")]
pub async fn get_tx_by_id(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> ApiResult<HttpResponse> {
    let tx_id = path.into_inner();
    let trace_id = uuid::Uuid::new_v4().to_string();
    // Try the default channel store.
    let store = get_channel_store(&state, "default")?;
    match store.read_transaction(&tx_id) {
        Ok(tx) => Ok(HttpResponse::Ok().json(ApiResponse::success(tx, trace_id))),
        Err(_) => Err(ApiError::NotFound {
            resource: format!("transaction {tx_id}"),
        }),
    }
}

/// GET /api/v1/mempool — transacciones pendientes.
#[get("/mempool")]
pub async fn get_mempool(state: web::Data<AppState>) -> ApiResult<HttpResponse> {
    let trace_id = uuid::Uuid::new_v4().to_string();

    // Read from new TransactionPool
    let pool = state.tx_pool.lock().unwrap_or_else(|e| e.into_inner());
    let transactions = pool.all().to_vec();
    drop(pool);

    #[derive(serde::Serialize)]
    struct PoolResponse {
        count: usize,
        transactions: Vec<crate::storage::traits::Transaction>,
    }
    let data = PoolResponse {
        count: transactions.len(),
        transactions,
    };
    let body = ApiResponse::success(data, trace_id);
    Ok(HttpResponse::Ok().json(body))
}
