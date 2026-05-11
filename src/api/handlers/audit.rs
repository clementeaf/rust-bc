//! Audit trail endpoints:
//!   GET /api/v1/audit/requests      — list audit entries
//!   GET /api/v1/audit/export        — export as CSV

use actix_web::{get, web, HttpRequest, HttpResponse};
use serde::Deserialize;

use crate::api::errors::{ApiError, ApiResponse, ApiResult};
use crate::app_state::AppState;

#[derive(Deserialize)]
pub struct AuditQuery {
    pub from: Option<String>,
    pub to: Option<String>,
    pub org_id: Option<String>,
    pub limit: Option<usize>,
}

/// GET /api/v1/audit/requests — query audit log entries.
#[get("/audit/requests")]
pub async fn list_audit_entries(
    state: web::Data<AppState>,
    query: web::Query<AuditQuery>,
    _req: HttpRequest,
) -> ApiResult<HttpResponse> {
    let trace_id = uuid::Uuid::new_v4().to_string();
    let store = state.audit_store.as_ref().ok_or(ApiError::NotFound {
        resource: "audit_store".to_string(),
    })?;

    let entries = store
        .query(
            query.from.as_deref(),
            query.to.as_deref(),
            query.org_id.as_deref(),
            query.limit.unwrap_or(1000),
        )
        .map_err(|e| ApiError::StorageError {
            reason: e.to_string(),
        })?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(entries, trace_id)))
}

/// GET /api/v1/audit/export — export audit log as CSV.
#[get("/audit/export")]
pub async fn export_audit_csv(
    state: web::Data<AppState>,
    query: web::Query<AuditQuery>,
    _req: HttpRequest,
) -> ApiResult<HttpResponse> {
    let store = state.audit_store.as_ref().ok_or(ApiError::NotFound {
        resource: "audit_store".to_string(),
    })?;

    let csv = store
        .export_csv(query.from.as_deref(), query.to.as_deref())
        .map_err(|e| ApiError::StorageError {
            reason: e.to_string(),
        })?;

    Ok(HttpResponse::Ok()
        .content_type("text/csv; charset=utf-8")
        .insert_header(("Content-Disposition", "attachment; filename=audit.csv"))
        .body(csv))
}
