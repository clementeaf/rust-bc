use actix_web::{get, post, web, HttpResponse};

use crate::api::errors::{ApiError, ApiResponse, ApiResult};
use crate::api::models::CreateBlockRequest;
use crate::app_state::AppState;
use crate::block_creation;

/// POST /api/v1/blocks — crea un bloque (lógica en `block_creation::try_create_block`).
#[post("")]
pub async fn create_block(
    state: web::Data<AppState>,
    req: web::Json<CreateBlockRequest>,
) -> ApiResult<HttpResponse> {
    let trace_id = uuid::Uuid::new_v4().to_string();
    match block_creation::try_create_block(state.get_ref(), &*req) {
        Ok(hash) => {
            let body = ApiResponse::success(hash, trace_id);
            Ok(HttpResponse::Created().json(body))
        }
        Err(reason) => Err(ApiError::ValidationError {
            field: "block".to_string(),
            reason,
        }),
    }
}

/// GET /api/v1/blocks — lista la cadena completa.
#[get("")]
pub async fn list_blocks(state: web::Data<AppState>) -> ApiResult<HttpResponse> {
    let trace_id = uuid::Uuid::new_v4().to_string();
    let blockchain = state.blockchain.lock().unwrap_or_else(|e| e.into_inner());
    let chain = blockchain.chain.clone();
    drop(blockchain);
    let body = ApiResponse::success(chain, trace_id);
    Ok(HttpResponse::Ok().json(body))
}

/// GET /api/v1/blocks/index/{index} — bloque por altura (antes de `/{hash}`).
#[get("/index/{index}")]
pub async fn get_block_by_index(
    state: web::Data<AppState>,
    path: web::Path<u64>,
) -> ApiResult<HttpResponse> {
    let idx = *path;
    let trace_id = uuid::Uuid::new_v4().to_string();
    let blockchain = state.blockchain.lock().unwrap_or_else(|e| e.into_inner());
    let result = blockchain.get_block_by_index(idx).cloned();
    drop(blockchain);
    match result {
        Some(block) => {
            let body = ApiResponse::success(block, trace_id);
            Ok(HttpResponse::Ok().json(body))
        }
        None => Err(ApiError::NotFound {
            resource: format!("block index {}", idx),
        }),
    }
}

/// GET /api/v1/blocks/{hash} — bloque por hash.
#[get("/{hash}")]
pub async fn get_block_by_hash(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> ApiResult<HttpResponse> {
    let hash = path.into_inner();
    let trace_id = uuid::Uuid::new_v4().to_string();
    let blockchain = state.blockchain.lock().unwrap_or_else(|e| e.into_inner());
    let result = blockchain.get_block_by_hash(&hash).cloned();
    drop(blockchain);
    match result {
        Some(block) => {
            let body = ApiResponse::success(block, trace_id);
            Ok(HttpResponse::Ok().json(body))
        }
        None => Err(ApiError::NotFound {
            resource: format!("block {}", hash),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::{create_block, get_block_by_hash, get_block_by_index, list_blocks};

    #[test]
    fn blocks_gateway_handlers_are_public() {
        let _ = (create_block, list_blocks, get_block_by_index, get_block_by_hash);
    }
}
