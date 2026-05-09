//! Regulatory sandbox endpoints:
//!   GET  /api/v1/regulatory/checks   — run all compliance checks
//!   GET  /api/v1/regulatory/report   — generate full compliance report

use actix_web::{get, HttpResponse};

use crate::api::errors::{ApiResponse, ApiResult};
use crate::regulatory::{report, sandbox};

/// GET /api/v1/regulatory/checks — run compliance checks
#[get("/regulatory/checks")]
pub async fn run_checks() -> ApiResult<HttpResponse> {
    let trace = uuid::Uuid::new_v4().to_string();
    let results = sandbox::run_compliance_checks();
    let summary = sandbox::summarize(&results);

    Ok(HttpResponse::Ok().json(ApiResponse::success(
        serde_json::json!({
            "summary": summary,
            "checks": results,
        }),
        trace,
    )))
}

/// GET /api/v1/regulatory/report — generate signed compliance report
#[get("/regulatory/report")]
pub async fn compliance_report() -> ApiResult<HttpResponse> {
    let trace = uuid::Uuid::new_v4().to_string();
    let results = sandbox::run_compliance_checks();
    let summary = sandbox::summarize(&results);
    let rpt = report::generate_report(results, summary);

    Ok(HttpResponse::Ok().json(ApiResponse::success(rpt, trace)))
}
