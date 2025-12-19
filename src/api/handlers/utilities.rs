use actix_web::{HttpRequest, HttpResponse, get};
use crate::api::errors::ApiResult;
use crate::api::models::*;
use crate::api::openapi::OpenApi;

/// GET /health - Health check
#[get("/health")]
async fn health_check(_req: HttpRequest) -> ApiResult<HttpResponse> {
    let trace_id = uuid::Uuid::new_v4().to_string();
    
    // TODO: Check storage connectivity
    // TODO: Check consensus state availability
    
    let response = HealthResponse {
        status: "healthy".to_string(),
        uptime_seconds: 3600,
        blockchain: BlockchainHealthResponse {
            height: 1000,
            last_block_hash: "hash_placeholder".to_string(),
            validators_count: 1,
        },
    };

    let api_response = crate::api::errors::ApiResponse::success(response, trace_id);
    Ok(HttpResponse::Ok().json(api_response))
}

/// GET /version - Get API version
#[get("/version")]
async fn get_version(_req: HttpRequest) -> ApiResult<HttpResponse> {
    let trace_id = uuid::Uuid::new_v4().to_string();
    
    let response = VersionResponse {
        api_version: "1.0.0".to_string(),
        rust_bc_version: "0.1.0".to_string(),
        blockchain_height: 1000,
    };

    let api_response = crate::api::errors::ApiResponse::success(response, trace_id);
    Ok(HttpResponse::Ok().json(api_response))
}

/// GET /openapi.json - OpenAPI specification
#[get("/openapi.json")]
async fn get_openapi(_req: HttpRequest) -> ApiResult<HttpResponse> {
    let spec = OpenApi::spec();
    Ok(HttpResponse::Ok().json(spec))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_utilities_handlers_module_compiles() {
        assert!(true);
    }
}
