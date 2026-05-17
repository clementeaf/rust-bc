//! CRUD endpoints for institutional governance entities (Cerulean Voto).
//!
//! Scopes, assemblies, sessions, actas — persisted to BlockStore
//! with channel-aware routing via X-Channel-Id header.

use actix_web::{delete, get, post, put, web, HttpRequest, HttpResponse};

use crate::api::errors::{enforce_acl, ApiError, ApiResponse, ApiResult};
use crate::api::handlers::channels::{channel_id_from_req, get_channel_store};
use crate::app_state::AppState;
use crate::storage::traits::{Acta, Assembly, Scope, Session};

// ── Scopes ──────────────────────────────────────────────────────────────

#[post("/store/scopes")]
pub async fn create_scope(
    state: web::Data<AppState>,
    body: web::Json<Scope>,
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
        .write_scope(&body)
        .map_err(|e| ApiError::StorageError {
            reason: e.to_string(),
        })?;
    Ok(HttpResponse::Created().json(ApiResponse::success(&*body, trace)))
}

#[get("/store/scopes")]
pub async fn list_scopes(state: web::Data<AppState>, req: HttpRequest) -> ApiResult<HttpResponse> {
    let trace = uuid::Uuid::new_v4().to_string();
    let store = get_channel_store(&state, channel_id_from_req(&req))?;
    let scopes = store.list_scopes().map_err(|e| ApiError::StorageError {
        reason: e.to_string(),
    })?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(scopes, trace)))
}

#[get("/store/scopes/{id}")]
pub async fn get_scope(
    state: web::Data<AppState>,
    id: web::Path<String>,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    let trace = uuid::Uuid::new_v4().to_string();
    let store = get_channel_store(&state, channel_id_from_req(&req))?;
    let scope = store.read_scope(&id).map_err(|e| ApiError::StorageError {
        reason: e.to_string(),
    })?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(scope, trace)))
}

#[put("/store/scopes/{id}")]
pub async fn update_scope(
    state: web::Data<AppState>,
    id: web::Path<String>,
    body: web::Json<Scope>,
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
    let mut scope = body.into_inner();
    scope.id = id.into_inner();
    store
        .write_scope(&scope)
        .map_err(|e| ApiError::StorageError {
            reason: e.to_string(),
        })?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(scope, trace)))
}

#[delete("/store/scopes/{id}")]
pub async fn remove_scope(
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
        .delete_scope(&id)
        .map_err(|e| ApiError::StorageError {
            reason: e.to_string(),
        })?;
    Ok(HttpResponse::Ok().json(ApiResponse::success("deleted", trace)))
}

// ── Assemblies ──────────────────────────────────────────────────────────

#[post("/store/assemblies")]
pub async fn create_assembly(
    state: web::Data<AppState>,
    body: web::Json<Assembly>,
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
        .write_assembly(&body)
        .map_err(|e| ApiError::StorageError {
            reason: e.to_string(),
        })?;
    Ok(HttpResponse::Created().json(ApiResponse::success(&*body, trace)))
}

#[get("/store/assemblies")]
pub async fn list_assemblies(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> ApiResult<HttpResponse> {
    let trace = uuid::Uuid::new_v4().to_string();
    let store = get_channel_store(&state, channel_id_from_req(&req))?;
    let assemblies = if let Some(scope_id) = query.get("scope_id") {
        store.list_assemblies_by_scope(scope_id)
    } else {
        store.list_assemblies()
    }
    .map_err(|e| ApiError::StorageError {
        reason: e.to_string(),
    })?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(assemblies, trace)))
}

#[get("/store/assemblies/{id}")]
pub async fn get_assembly(
    state: web::Data<AppState>,
    id: web::Path<String>,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    let trace = uuid::Uuid::new_v4().to_string();
    let store = get_channel_store(&state, channel_id_from_req(&req))?;
    let assembly = store
        .read_assembly(&id)
        .map_err(|e| ApiError::StorageError {
            reason: e.to_string(),
        })?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(assembly, trace)))
}

#[put("/store/assemblies/{id}")]
pub async fn update_assembly(
    state: web::Data<AppState>,
    id: web::Path<String>,
    body: web::Json<Assembly>,
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
    let mut assembly = body.into_inner();
    assembly.id = id.into_inner();
    store
        .write_assembly(&assembly)
        .map_err(|e| ApiError::StorageError {
            reason: e.to_string(),
        })?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(assembly, trace)))
}

#[delete("/store/assemblies/{id}")]
pub async fn remove_assembly(
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
        .delete_assembly(&id)
        .map_err(|e| ApiError::StorageError {
            reason: e.to_string(),
        })?;
    Ok(HttpResponse::Ok().json(ApiResponse::success("deleted", trace)))
}

// ── Sessions ────────────────────────────────────────────────────────────

#[post("/store/sessions")]
pub async fn create_session(
    state: web::Data<AppState>,
    body: web::Json<Session>,
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
        .write_session(&body)
        .map_err(|e| ApiError::StorageError {
            reason: e.to_string(),
        })?;
    Ok(HttpResponse::Created().json(ApiResponse::success(&*body, trace)))
}

#[get("/store/sessions")]
pub async fn list_sessions(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> ApiResult<HttpResponse> {
    let trace = uuid::Uuid::new_v4().to_string();
    let store = get_channel_store(&state, channel_id_from_req(&req))?;
    let sessions = if let Some(assembly_id) = query.get("assembly_id") {
        store.list_sessions_by_assembly(assembly_id)
    } else {
        // No list_all_sessions — require assembly_id filter
        Ok(vec![])
    }
    .map_err(|e| ApiError::StorageError {
        reason: e.to_string(),
    })?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(sessions, trace)))
}

#[get("/store/sessions/{id}")]
pub async fn get_session(
    state: web::Data<AppState>,
    id: web::Path<String>,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    let trace = uuid::Uuid::new_v4().to_string();
    let store = get_channel_store(&state, channel_id_from_req(&req))?;
    let session = store
        .read_session(&id)
        .map_err(|e| ApiError::StorageError {
            reason: e.to_string(),
        })?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(session, trace)))
}

#[put("/store/sessions/{id}")]
pub async fn update_session(
    state: web::Data<AppState>,
    id: web::Path<String>,
    body: web::Json<Session>,
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
    let mut session = body.into_inner();
    session.id = id.into_inner();
    store
        .write_session(&session)
        .map_err(|e| ApiError::StorageError {
            reason: e.to_string(),
        })?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(session, trace)))
}

#[delete("/store/sessions/{id}")]
pub async fn remove_session(
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
        .delete_session(&id)
        .map_err(|e| ApiError::StorageError {
            reason: e.to_string(),
        })?;
    Ok(HttpResponse::Ok().json(ApiResponse::success("deleted", trace)))
}

// ── Actas ───────────────────────────────────────────────────────────────

#[post("/store/actas")]
pub async fn create_acta(
    state: web::Data<AppState>,
    body: web::Json<Acta>,
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
        .write_acta(&body)
        .map_err(|e| ApiError::StorageError {
            reason: e.to_string(),
        })?;
    Ok(HttpResponse::Created().json(ApiResponse::success(&*body, trace)))
}

#[get("/store/actas")]
pub async fn list_actas(state: web::Data<AppState>, req: HttpRequest) -> ApiResult<HttpResponse> {
    let trace = uuid::Uuid::new_v4().to_string();
    let store = get_channel_store(&state, channel_id_from_req(&req))?;
    let actas = store.list_actas().map_err(|e| ApiError::StorageError {
        reason: e.to_string(),
    })?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(actas, trace)))
}

#[get("/store/actas/{id}")]
pub async fn get_acta(
    state: web::Data<AppState>,
    id: web::Path<String>,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    let trace = uuid::Uuid::new_v4().to_string();
    let store = get_channel_store(&state, channel_id_from_req(&req))?;
    let acta = store.read_acta(&id).map_err(|e| ApiError::StorageError {
        reason: e.to_string(),
    })?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(acta, trace)))
}

#[put("/store/actas/{id}")]
pub async fn update_acta(
    state: web::Data<AppState>,
    id: web::Path<String>,
    body: web::Json<Acta>,
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
    let mut acta = body.into_inner();
    acta.id = id.into_inner();
    store
        .write_acta(&acta)
        .map_err(|e| ApiError::StorageError {
            reason: e.to_string(),
        })?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(acta, trace)))
}

#[delete("/store/actas/{id}")]
pub async fn remove_acta(
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
    store.delete_acta(&id).map_err(|e| ApiError::StorageError {
        reason: e.to_string(),
    })?;
    Ok(HttpResponse::Ok().json(ApiResponse::success("deleted", trace)))
}
