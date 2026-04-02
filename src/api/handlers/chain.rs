use actix_web::{get, web, HttpResponse};

use crate::api::errors::{ApiResponse, ApiResult};
use crate::api::models::{ChainInfoResponse, ChainVerifyResponse};
use crate::app_state::AppState;

/// GET /api/v1/chain/verify — comprobación de integridad de la cadena local.
#[get("/verify")]
pub async fn verify_chain(state: web::Data<AppState>) -> ApiResult<HttpResponse> {
    let trace_id = uuid::Uuid::new_v4().to_string();
    let blockchain = state.blockchain.lock().unwrap_or_else(|e| e.into_inner());
    let valid = blockchain.is_chain_valid();
    let block_count = blockchain.chain.len();
    drop(blockchain);

    let data = ChainVerifyResponse {
        valid,
        block_count,
    };
    let body = ApiResponse::success(data, trace_id);
    Ok(HttpResponse::Ok().json(body))
}

/// GET /api/v1/chain/info — metadatos y validez de la cadena.
#[get("/info")]
pub async fn get_blockchain_info(state: web::Data<AppState>) -> ApiResult<HttpResponse> {
    let trace_id = uuid::Uuid::new_v4().to_string();
    let blockchain = state.blockchain.lock().unwrap_or_else(|e| e.into_inner());
    let latest_hash = blockchain.get_latest_block().hash.clone();
    let data = ChainInfoResponse {
        block_count: blockchain.chain.len(),
        difficulty: blockchain.difficulty,
        latest_block_hash: latest_hash,
        is_valid: blockchain.is_chain_valid(),
    };
    drop(blockchain);

    let body = ApiResponse::success(data, trace_id);
    Ok(HttpResponse::Ok().json(body))
}

#[cfg(test)]
mod tests {
    use super::{get_blockchain_info, verify_chain};

    #[test]
    fn chain_gateway_handlers_are_public() {
        let _ = (verify_chain, get_blockchain_info);
    }
}
