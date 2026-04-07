use actix_web::{web, HttpResponse, get, post};

use actix_web::HttpRequest;
use crate::api::errors::{enforce_acl, ApiError, ApiResponse, ApiResult};
use crate::endorsement::org::Organization;
use crate::endorsement::policy::EndorsementPolicy;
use crate::app_state::AppState;

// ── Organizations ─────────────────────────────────────────────────────────────

/// POST /api/v1/store/organizations — register an organization
#[post("/store/organizations")]
pub async fn store_create_organization(
    state: web::Data<AppState>,
    body: web::Json<Organization>,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    enforce_acl(state.acl_provider.as_deref(), state.policy_store.as_deref(), "peer/Admin", &req)?;
    let trace_id = uuid::Uuid::new_v4().to_string();
    match &state.org_registry {
        None => Err(ApiError::NotFound { resource: "org_registry".to_string() }),
        Some(reg) => {
            reg.register_org(&body)
                .map_err(|e| ApiError::StorageError { reason: e.to_string() })?;
            Ok(HttpResponse::Created().json(ApiResponse::success(body.into_inner(), trace_id)))
        }
    }
}

/// GET /api/v1/store/organizations — list all organizations
#[get("/store/organizations")]
pub async fn store_list_organizations(
    state: web::Data<AppState>,
) -> ApiResult<HttpResponse> {
    let trace_id = uuid::Uuid::new_v4().to_string();
    match &state.org_registry {
        None => Err(ApiError::NotFound { resource: "org_registry".to_string() }),
        Some(reg) => {
            let orgs = reg.list_orgs()
                .map_err(|e| ApiError::StorageError { reason: e.to_string() })?;
            Ok(HttpResponse::Ok().json(ApiResponse::success(orgs, trace_id)))
        }
    }
}

/// GET /api/v1/store/organizations/{org_id} — get a single organization
#[get("/store/organizations/{org_id}")]
pub async fn store_get_organization(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> ApiResult<HttpResponse> {
    let org_id = path.into_inner();
    let trace_id = uuid::Uuid::new_v4().to_string();
    match &state.org_registry {
        None => Err(ApiError::NotFound { resource: "org_registry".to_string() }),
        Some(reg) => match reg.get_org(&org_id) {
            Ok(org) => Ok(HttpResponse::Ok().json(ApiResponse::success(org, trace_id))),
            Err(_) => Err(ApiError::NotFound { resource: format!("organization {org_id}") }),
        },
    }
}

// ── Policies ──────────────────────────────────────────────────────────────────

#[derive(Debug, serde::Deserialize)]
pub struct SetPolicyRequest {
    pub resource_id: String,
    pub policy: EndorsementPolicy,
}

/// POST /api/v1/store/policies — create or update an endorsement policy
#[post("/store/policies")]
pub async fn store_set_policy(
    state: web::Data<AppState>,
    body: web::Json<SetPolicyRequest>,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    enforce_acl(state.acl_provider.as_deref(), state.policy_store.as_deref(), "peer/Admin", &req)?;
    let trace_id = uuid::Uuid::new_v4().to_string();
    match &state.policy_store {
        None => Err(ApiError::NotFound { resource: "policy_store".to_string() }),
        Some(ps) => {
            ps.set_policy(&body.resource_id, &body.policy)
                .map_err(|e| ApiError::StorageError { reason: e.to_string() })?;
            Ok(HttpResponse::Created().json(ApiResponse::success(
                serde_json::json!({ "resource_id": body.resource_id }),
                trace_id,
            )))
        }
    }
}

/// GET /api/v1/store/policies/{resource_id} — get an endorsement policy
#[get("/store/policies/{resource_id}")]
pub async fn store_get_policy(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> ApiResult<HttpResponse> {
    let resource_id = path.into_inner();
    let trace_id = uuid::Uuid::new_v4().to_string();
    match &state.policy_store {
        None => Err(ApiError::NotFound { resource: "policy_store".to_string() }),
        Some(ps) => match ps.get_policy(&resource_id) {
            Ok(policy) => Ok(HttpResponse::Ok().json(ApiResponse::success(policy, trace_id))),
            Err(_) => Err(ApiError::NotFound { resource: format!("policy {resource_id}") }),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn org_handlers_are_public() {
        let _ = (
            store_create_organization,
            store_list_organizations,
            store_get_organization,
            store_set_policy,
            store_get_policy,
        );
    }
}
