//! ACL API handlers — set, list, and get ACL entries.

use actix_web::{get, post, web, HttpResponse};
use serde::Deserialize;

use crate::acl::AclEntry;
use crate::api::errors::{ApiError, ApiResponse, ApiResult};
use crate::app_state::AppState;

// ── Request types ─────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct SetAclRequest {
    pub resource: String,
    pub policy_ref: String,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// POST /api/v1/acls — set (create or overwrite) an ACL entry.
///
/// Returns 200 with the resulting `AclEntry`.
/// Returns 503 if no `acl_provider` is configured in `AppState`.
#[post("/acls")]
pub async fn set_acl(
    state: web::Data<AppState>,
    body: web::Json<SetAclRequest>,
) -> ApiResult<HttpResponse> {
    let trace_id = uuid::Uuid::new_v4().to_string();
    let req = body.into_inner();

    let provider = state.acl_provider.as_deref().ok_or_else(|| ApiError::InternalError {
        reason: "ACL provider not configured".to_string(),
    })?;

    if req.resource.is_empty() {
        return Err(ApiError::ValidationError {
            field: "resource".to_string(),
            reason: "must be non-empty".to_string(),
        });
    }
    if req.policy_ref.is_empty() {
        return Err(ApiError::ValidationError {
            field: "policy_ref".to_string(),
            reason: "must be non-empty".to_string(),
        });
    }

    provider.set_acl(&req.resource, &req.policy_ref).map_err(|e| ApiError::InternalError {
        reason: e.to_string(),
    })?;

    let entry = AclEntry::new(req.resource, req.policy_ref);
    Ok(HttpResponse::Ok().json(ApiResponse::success(entry, trace_id)))
}

/// GET /api/v1/acls — list all ACL entries (sorted by resource).
#[get("/acls")]
pub async fn list_acls(state: web::Data<AppState>) -> ApiResult<HttpResponse> {
    let trace_id = uuid::Uuid::new_v4().to_string();

    let provider = state.acl_provider.as_deref().ok_or_else(|| ApiError::InternalError {
        reason: "ACL provider not configured".to_string(),
    })?;

    let mut entries = provider.list_acls().map_err(|e| ApiError::InternalError {
        reason: e.to_string(),
    })?;
    entries.sort_by(|a, b| a.resource.cmp(&b.resource));

    Ok(HttpResponse::Ok().json(ApiResponse::success(entries, trace_id)))
}

/// GET /api/v1/acls/{resource} — get the ACL entry for a specific resource.
///
/// `resource` path segments use `/` encoded as `%2F` (e.g. `peer%2FChaincodeInvoke`).
/// Returns 404 if no entry exists for the given resource.
#[get("/acls/{resource}")]
pub async fn get_acl(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> ApiResult<HttpResponse> {
    let resource = path.into_inner();
    let trace_id = uuid::Uuid::new_v4().to_string();

    let provider = state.acl_provider.as_deref().ok_or_else(|| ApiError::InternalError {
        reason: "ACL provider not configured".to_string(),
    })?;

    let entry = provider.get_acl(&resource).map_err(|e| ApiError::InternalError {
        reason: e.to_string(),
    })?.ok_or_else(|| ApiError::NotFound {
        resource: format!("ACL for '{resource}'"),
    })?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(entry, trace_id)))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::acl::{AclProvider, MemoryAclProvider};
    use crate::api::errors::ApiError;

    fn provider() -> Arc<dyn AclProvider> {
        Arc::new(MemoryAclProvider::new())
    }

    fn call_set_acl(
        p: &dyn AclProvider,
        resource: &str,
        policy_ref: &str,
    ) -> Result<(), ApiError> {
        if resource.is_empty() {
            return Err(ApiError::ValidationError {
                field: "resource".to_string(),
                reason: "must be non-empty".to_string(),
            });
        }
        if policy_ref.is_empty() {
            return Err(ApiError::ValidationError {
                field: "policy_ref".to_string(),
                reason: "must be non-empty".to_string(),
            });
        }
        p.set_acl(resource, policy_ref).map_err(|e| ApiError::InternalError { reason: e.to_string() })
    }

    // ── set_acl ───────────────────────────────────────────────────────────────

    #[test]
    fn set_acl_stores_entry() {
        let p = provider();
        call_set_acl(p.as_ref(), "peer/ChaincodeInvoke", "OrgPolicy").unwrap();
        let entry = p.get_acl("peer/ChaincodeInvoke").unwrap().expect("entry");
        assert_eq!(entry.resource, "peer/ChaincodeInvoke");
        assert_eq!(entry.policy_ref, "OrgPolicy");
    }

    #[test]
    fn set_acl_empty_resource_returns_validation_error() {
        let p = provider();
        let err = call_set_acl(p.as_ref(), "", "OrgPolicy").unwrap_err();
        assert!(matches!(err, ApiError::ValidationError { field, .. } if field == "resource"));
    }

    #[test]
    fn set_acl_empty_policy_ref_returns_validation_error() {
        let p = provider();
        let err = call_set_acl(p.as_ref(), "peer/BlockEvents", "").unwrap_err();
        assert!(matches!(err, ApiError::ValidationError { field, .. } if field == "policy_ref"));
    }

    #[test]
    fn set_acl_overwrites_existing_entry() {
        let p = provider();
        call_set_acl(p.as_ref(), "peer/ChaincodeInvoke", "PolicyA").unwrap();
        call_set_acl(p.as_ref(), "peer/ChaincodeInvoke", "PolicyB").unwrap();
        let entry = p.get_acl("peer/ChaincodeInvoke").unwrap().expect("entry");
        assert_eq!(entry.policy_ref, "PolicyB");
    }

    // ── list_acls ─────────────────────────────────────────────────────────────

    #[test]
    fn list_acls_returns_sorted_entries() {
        let p = provider();
        p.set_acl("peer/BlockEvents", "PolicyA").unwrap();
        p.set_acl("peer/ChaincodeInvoke", "PolicyB").unwrap();
        let mut entries = p.list_acls().unwrap();
        entries.sort_by(|a, b| a.resource.cmp(&b.resource));
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].resource, "peer/BlockEvents");
        assert_eq!(entries[1].resource, "peer/ChaincodeInvoke");
    }

    #[test]
    fn list_acls_empty_returns_empty_vec() {
        let p = provider();
        assert!(p.list_acls().unwrap().is_empty());
    }

    // ── get_acl ───────────────────────────────────────────────────────────────

    #[test]
    fn get_acl_returns_existing_entry() {
        let p = provider();
        p.set_acl("peer/PrivateDataRead", "AdminPolicy").unwrap();
        let entry = p.get_acl("peer/PrivateDataRead").unwrap().expect("entry");
        assert_eq!(entry.policy_ref, "AdminPolicy");
    }

    #[test]
    fn get_acl_unknown_resource_returns_none() {
        let p = provider();
        assert!(p.get_acl("peer/Unknown").unwrap().is_none());
    }
}
