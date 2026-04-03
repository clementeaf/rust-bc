//! POST /transactions and GET /mempool (gateway envelope).

use actix_web::{get, post, web, HttpRequest, HttpResponse};

use crate::api::errors::{ApiError, ApiResponse, ApiResult};
use crate::api::models::{CreateTransactionRequest, MempoolResponse};
use crate::app_state::AppState;
use crate::models::Transaction;

/// POST /api/v1/transactions — valida y encola en el mempool.
#[post("/transactions")]
pub async fn create_transaction(
    state: web::Data<AppState>,
    req: web::Json<CreateTransactionRequest>,
    http_req: HttpRequest,
) -> ApiResult<HttpResponse> {
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

    let fee = req.fee.unwrap_or(0);
    if req.from != "0" && fee == 0 {
        return Err(ApiError::ValidationError {
            field: "fee".to_string(),
            reason: "Fee requerido: todas las transacciones deben incluir un fee > 0".to_string(),
        });
    }

    let mut tx = Transaction::new_with_fee(
        req.from.clone(),
        req.to.clone(),
        req.amount,
        fee,
        req.data.clone(),
    );

    if !tx.is_valid() {
        return Err(ApiError::ValidationError {
            field: "transaction".to_string(),
            reason: "Transacción inválida".to_string(),
        });
    }

    if req.from != "0" {
        let wallet_manager = state
            .wallet_manager
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let wallet = match wallet_manager.get_wallet_for_signing(&req.from) {
            Some(w) => w,
            None => {
                return Err(ApiError::ValidationError {
                    field: "from".to_string(),
                    reason: "Wallet no encontrado para firmar".to_string(),
                });
            }
        };

        if let Some(sig) = &req.signature {
            if !sig.is_empty() {
                tx.signature = sig.clone();
            } else {
                wallet.sign_transaction(&mut tx);
            }
        } else {
            wallet.sign_transaction(&mut tx);
        }
        drop(wallet_manager);

        let (balance, validation_result) = {
            let blockchain = state.blockchain.lock().unwrap_or_else(|e| e.into_inner());
            let wallet_manager = state
                .wallet_manager
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            let bal = blockchain.calculate_balance(&req.from);
            let validation = blockchain.validate_transaction(&tx, &wallet_manager);
            (bal, validation)
        };

        if let Err(e) = validation_result {
            return Err(ApiError::ValidationError {
                field: "transaction".to_string(),
                reason: e,
            });
        }

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

        let mut mempool = state.mempool.lock().unwrap_or_else(|e| e.into_inner());

        if mempool.has_double_spend(&tx) {
            return Err(ApiError::ValidationError {
                field: "transaction".to_string(),
                reason: "Doble gasto detectado en mempool".to_string(),
            });
        }

        let pending_spent = mempool.calculate_pending_spent(&req.from);
        let total_required = tx.amount + tx.fee;
        let available_balance = balance.saturating_sub(pending_spent);

        if available_balance < total_required {
            return Err(ApiError::ValidationError {
                field: "balance".to_string(),
                reason: format!(
                    "Saldo insuficiente de token nativo. Disponible: {}, Requerido: {} (amount: {} + fee: {}). Pendiente en mempool: {}. Los fees solo se pueden pagar con el token nativo.",
                    available_balance, total_required, tx.amount, tx.fee, pending_spent
                ),
            });
        }

        if let Err(e) = mempool.add_transaction(tx.clone()) {
            drop(mempool);
            return Err(ApiError::ValidationError {
                field: "mempool".to_string(),
                reason: e,
            });
        }
        drop(mempool);

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

    if let Some(node) = &state.node {
        let tx_clone = tx.clone();
        let node_clone = node.clone();
        tokio::spawn(async move {
            node_clone.broadcast_transaction(&tx_clone).await;
        });
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
) -> ApiResult<HttpResponse> {
    let trace_id = uuid::Uuid::new_v4().to_string();
    match &state.store {
        None => Err(ApiError::NotFound { resource: "store".to_string() }),
        Some(store) => {
            store
                .write_transaction(&body)
                .map_err(|e| ApiError::StorageError { reason: e.to_string() })?;
            Ok(HttpResponse::Created().json(ApiResponse::success(body.into_inner(), trace_id)))
        }
    }
}

/// GET /api/v1/store/transactions/{tx_id} — lee una transacción del store.
#[get("/store/transactions/{tx_id}")]
pub async fn store_get_transaction(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> ApiResult<HttpResponse> {
    let tx_id = path.into_inner();
    let trace_id = uuid::Uuid::new_v4().to_string();
    match &state.store {
        None => Err(ApiError::NotFound { resource: "store".to_string() }),
        Some(store) => match store.read_transaction(&tx_id) {
            Ok(tx) => Ok(HttpResponse::Ok().json(ApiResponse::success(tx, trace_id))),
            Err(_) => Err(ApiError::NotFound { resource: format!("transaction {tx_id}") }),
        },
    }
}

/// GET /api/v1/mempool — transacciones pendientes.
#[get("/mempool")]
pub async fn get_mempool(state: web::Data<AppState>) -> ApiResult<HttpResponse> {
    let trace_id = uuid::Uuid::new_v4().to_string();
    let mempool = state.mempool.lock().unwrap_or_else(|e| e.into_inner());
    let transactions = mempool.get_all_transactions().to_vec();
    drop(mempool);
    let data = MempoolResponse {
        count: transactions.len(),
        transactions,
    };
    let body = ApiResponse::success(data, trace_id);
    Ok(HttpResponse::Ok().json(body))
}
