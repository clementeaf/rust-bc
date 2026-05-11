//! Legal oracle endpoints:
//!   POST /api/v1/oracle/legal/query       — query a legal source
//!   GET  /api/v1/oracle/legal/records     — list oracle records
//!   GET  /api/v1/oracle/legal/records/{id} — get a specific record

use actix_web::{get, post, web, HttpRequest, HttpResponse};
use serde::Deserialize;

use crate::api::errors::{ApiError, ApiResponse, ApiResult};
use crate::app_state::AppState;

#[derive(Deserialize)]
pub struct QueryRequest {
    pub source: String,
    pub query: String,
}

#[derive(Deserialize)]
pub struct ListQuery {
    pub source: Option<String>,
    pub limit: Option<usize>,
}

/// POST /api/v1/oracle/legal/query — query a legal data source.
#[post("/oracle/legal/query")]
pub async fn query_legal_oracle(
    state: web::Data<AppState>,
    body: web::Json<QueryRequest>,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    let trace_id = uuid::Uuid::new_v4().to_string();

    let oracle = state.legal_oracle.lock().unwrap_or_else(|e| e.into_inner());

    // Use a stub fetch that returns a placeholder — real HTTP fetch would go here.
    // In production, this closure would call reqwest/hyper against the source URL.
    let result = oracle.query(
        &body.source,
        &body.query,
        state.legal_oracle_store.as_ref(),
        |config, query_text| {
            // Stub: return a JSON response with the query embedded.
            // Replace with real HTTP client in production.
            let response = serde_json::json!({
                "source": config.id,
                "base_url": config.base_url,
                "query": query_text,
                "result": "legal_data_placeholder",
                "timestamp": std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            });
            Ok(serde_json::to_vec(&response).unwrap_or_default())
        },
    );

    match result {
        Ok(record) => {
            crate::audit::emit_if_present(
                &state.audit_store,
                crate::audit::AuditAction::ProposalSubmitted, // Reuse closest action
                req.headers()
                    .get("X-Org-Id")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("unknown"),
                Some(format!(
                    "legal_oracle_query,source={},query={}",
                    body.source, body.query
                )),
            );
            Ok(HttpResponse::Ok().json(ApiResponse::success(record, trace_id)))
        }
        Err(crate::legal_oracle::OracleError::SourceNotConfigured(s)) => Err(ApiError::NotFound {
            resource: format!("legal oracle source '{s}'"),
        }),
        Err(e) => Err(ApiError::StorageError {
            reason: e.to_string(),
        }),
    }
}

/// GET /api/v1/oracle/legal/records — list oracle records.
#[get("/oracle/legal/records")]
pub async fn list_legal_oracle_records(
    state: web::Data<AppState>,
    query: web::Query<ListQuery>,
) -> ApiResult<HttpResponse> {
    let trace_id = uuid::Uuid::new_v4().to_string();
    let records = state
        .legal_oracle_store
        .list(query.source.as_deref(), query.limit.unwrap_or(100))
        .map_err(|e| ApiError::StorageError {
            reason: e.to_string(),
        })?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(records, trace_id)))
}

/// GET /api/v1/oracle/legal/records/{id} — get a specific oracle record.
#[get("/oracle/legal/records/{id}")]
pub async fn get_legal_oracle_record(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> ApiResult<HttpResponse> {
    let id = path.into_inner();
    let trace_id = uuid::Uuid::new_v4().to_string();
    match state.legal_oracle_store.get(&id) {
        Ok(Some(record)) => Ok(HttpResponse::Ok().json(ApiResponse::success(record, trace_id))),
        Ok(None) => Err(ApiError::NotFound {
            resource: format!("oracle record '{id}'"),
        }),
        Err(e) => Err(ApiError::StorageError {
            reason: e.to_string(),
        }),
    }
}
