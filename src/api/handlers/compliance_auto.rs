//! HTTP endpoints for Compliance Automation.

use actix_web::{delete, get, post, web, HttpRequest, HttpResponse};

use crate::api::errors::{enforce_acl, ApiError, ApiResponse, ApiResult};
use crate::api::handlers::channels::{channel_id_from_req, get_channel_store};
use crate::app_state::AppState;
use crate::registry::compliance::{self, ComplianceResult, ComplianceRule};

// ── Rules ───────────────────────────────────────────────────────────────

#[post("/store/compliance/rules")]
pub async fn create_rule(
    state: web::Data<AppState>,
    body: web::Json<ComplianceRule>,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    enforce_acl(
        state.acl_provider.as_deref(),
        state.policy_store.as_deref(),
        "peer/Propose",
        &req,
    )?;
    let trace = uuid::Uuid::new_v4().to_string();
    let store = get_channel_store(&state, channel_id_from_req(&req))?;
    store
        .write_compliance_rule(&body)
        .map_err(|e| ApiError::StorageError {
            reason: e.to_string(),
        })?;
    Ok(HttpResponse::Created().json(ApiResponse::success(&*body, trace)))
}

#[get("/store/compliance/rules")]
pub async fn list_rules(state: web::Data<AppState>, req: HttpRequest) -> ApiResult<HttpResponse> {
    let trace = uuid::Uuid::new_v4().to_string();
    let store = get_channel_store(&state, channel_id_from_req(&req))?;
    let rules = store
        .list_compliance_rules()
        .map_err(|e| ApiError::StorageError {
            reason: e.to_string(),
        })?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(rules, trace)))
}

#[get("/store/compliance/rules/{id}")]
pub async fn get_rule(
    state: web::Data<AppState>,
    id: web::Path<String>,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    let trace = uuid::Uuid::new_v4().to_string();
    let store = get_channel_store(&state, channel_id_from_req(&req))?;
    let rule = store
        .read_compliance_rule(&id)
        .map_err(|e| ApiError::StorageError {
            reason: e.to_string(),
        })?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(rule, trace)))
}

#[delete("/store/compliance/rules/{id}")]
pub async fn remove_rule(
    state: web::Data<AppState>,
    id: web::Path<String>,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    enforce_acl(
        state.acl_provider.as_deref(),
        state.policy_store.as_deref(),
        "peer/Propose",
        &req,
    )?;
    let trace = uuid::Uuid::new_v4().to_string();
    let store = get_channel_store(&state, channel_id_from_req(&req))?;
    store
        .delete_compliance_rule(&id)
        .map_err(|e| ApiError::StorageError {
            reason: e.to_string(),
        })?;
    Ok(HttpResponse::Ok().json(ApiResponse::success("deleted", trace)))
}

// ── Evaluate ────────────────────────────────────────────────────────────

/// POST /store/compliance/evaluate — run all rules against a specific asset's latest events.
#[post("/store/compliance/evaluate/{asset_id}")]
pub async fn evaluate_asset(
    state: web::Data<AppState>,
    asset_id: web::Path<String>,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    let trace = uuid::Uuid::new_v4().to_string();
    let store = get_channel_store(&state, channel_id_from_req(&req))?;
    let asset_id = asset_id.into_inner();

    let rules = store
        .list_compliance_rules()
        .map_err(|e| ApiError::StorageError {
            reason: e.to_string(),
        })?;
    let events = store
        .list_asset_events(&asset_id)
        .map_err(|e| ApiError::StorageError {
            reason: e.to_string(),
        })?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let mut results = Vec::new();

    for rule in &rules {
        if rule.target != "*" && rule.target != asset_id {
            continue;
        }
        for event in &events {
            if event.event_type != rule.event_type {
                continue;
            }
            // Extract numeric value from event data
            let actual = event
                .data
                .get(&rule.field)
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);

            let passed = compliance::evaluate_rule(rule, actual);

            let result = ComplianceResult {
                id: uuid::Uuid::new_v4().to_string(),
                rule_id: rule.id.clone(),
                asset_id: asset_id.clone(),
                event_id: event.id.clone(),
                passed,
                actual_value: actual,
                expected_value: rule.threshold,
                detail: if passed {
                    format!(
                        "{}: {} {:?} {} ✓",
                        rule.name, actual, rule.operator, rule.threshold
                    )
                } else {
                    format!(
                        "{}: {} {:?} {} ✗",
                        rule.name, actual, rule.operator, rule.threshold
                    )
                },
                evaluated_at: now,
            };
            let _ = store.write_compliance_result(&result);
            results.push(result);
        }
    }

    let passed = results.iter().filter(|r| r.passed).count();
    let failed = results.iter().filter(|r| !r.passed).count();

    Ok(HttpResponse::Ok().json(ApiResponse::success(
        serde_json::json!({
            "asset_id": asset_id,
            "rules_evaluated": results.len(),
            "passed": passed,
            "failed": failed,
            "results": results,
        }),
        trace,
    )))
}

// ── Results ─────────────────────────────────────────────────────────────

#[get("/store/compliance/results/{asset_id}")]
pub async fn get_results(
    state: web::Data<AppState>,
    asset_id: web::Path<String>,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    let trace = uuid::Uuid::new_v4().to_string();
    let store = get_channel_store(&state, channel_id_from_req(&req))?;
    let results = store
        .list_compliance_results(&asset_id)
        .map_err(|e| ApiError::StorageError {
            reason: e.to_string(),
        })?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(results, trace)))
}
