//! Forensic endpoints:
//!   GET  /api/v1/forensic/timeline       — full event timeline
//!   GET  /api/v1/forensic/security       — security events only
//!   POST /api/v1/forensic/export         — signed evidence package

use actix_web::{get, post, web, HttpResponse};
use serde::Deserialize;

use crate::api::errors::{ApiResponse, ApiResult};
use crate::app_state::AppState;
use crate::forensic::ForensicEngine;

#[derive(Deserialize)]
pub struct TimelineQuery {
    pub from: Option<String>,
    pub to: Option<String>,
    pub org_id: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Deserialize)]
pub struct ExportRequest {
    pub description: String,
    pub created_by: String,
}

fn build_engine(state: &AppState, query: &TimelineQuery) -> ForensicEngine {
    let mut engine = ForensicEngine::new();

    // Ingest audit entries if store is available
    if let Some(store) = &state.audit_store {
        if let Ok(entries) = store.query(
            query.from.as_deref(),
            query.to.as_deref(),
            query.org_id.as_deref(),
            query.limit.unwrap_or(10_000),
        ) {
            engine.ingest_audit(&entries);
        }
    }

    engine
}

/// GET /api/v1/forensic/timeline
#[get("/forensic/timeline")]
pub async fn forensic_timeline(
    state: web::Data<AppState>,
    query: web::Query<TimelineQuery>,
) -> ApiResult<HttpResponse> {
    let trace = uuid::Uuid::new_v4().to_string();
    let engine = build_engine(&state, &query);
    let timeline = engine.build_timeline();
    Ok(HttpResponse::Ok().json(ApiResponse::success(timeline, trace)))
}

/// GET /api/v1/forensic/security
#[get("/forensic/security")]
pub async fn forensic_security(
    state: web::Data<AppState>,
    query: web::Query<TimelineQuery>,
) -> ApiResult<HttpResponse> {
    let trace = uuid::Uuid::new_v4().to_string();
    let engine = build_engine(&state, &query);
    let timeline = engine.security_timeline();
    let summary = engine.severity_summary();

    Ok(HttpResponse::Ok().json(ApiResponse::success(
        serde_json::json!({
            "events": timeline,
            "summary": summary,
        }),
        trace,
    )))
}

/// POST /api/v1/forensic/export
#[post("/forensic/export")]
pub async fn forensic_export(
    state: web::Data<AppState>,
    body: web::Json<ExportRequest>,
) -> ApiResult<HttpResponse> {
    let trace = uuid::Uuid::new_v4().to_string();

    let query = TimelineQuery {
        from: None,
        to: None,
        org_id: None,
        limit: Some(100_000),
    };
    let engine = build_engine(&state, &query);
    let package = engine.build_evidence_package(&body.description, &body.created_by, None);

    Ok(HttpResponse::Ok().json(ApiResponse::success(package, trace)))
}
