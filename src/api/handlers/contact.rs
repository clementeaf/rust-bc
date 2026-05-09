//! Contact form endpoints:
//!   POST /api/v1/contact        — submit a contact request
//!   GET  /api/v1/contact        — list contact requests (admin only)

use actix_web::{get, post, web, HttpRequest, HttpResponse};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

use crate::api::errors::{ApiResponse, ApiResult, ErrorDto};

/// A contact form submission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactRequest {
    pub name: String,
    pub email: String,
    pub organization: Option<String>,
    pub message: String,
}

/// Stored contact entry with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactEntry {
    pub id: u64,
    pub name: String,
    pub email: String,
    pub organization: Option<String>,
    pub message: String,
    pub submitted_at: String,
}

/// In-memory contact store. Shared via AppState.
pub struct ContactStore {
    entries: Mutex<Vec<ContactEntry>>,
    next_id: Mutex<u64>,
}

impl ContactStore {
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(Vec::new()),
            next_id: Mutex::new(1),
        }
    }

    pub fn submit(&self, req: &ContactRequest) -> ContactEntry {
        let mut id = self.next_id.lock().unwrap();
        let entry = ContactEntry {
            id: *id,
            name: req.name.clone(),
            email: req.email.clone(),
            organization: req.organization.clone(),
            message: req.message.clone(),
            submitted_at: chrono::Utc::now().to_rfc3339(),
        };
        *id += 1;
        self.entries.lock().unwrap().push(entry.clone());
        entry
    }

    pub fn list(&self) -> Vec<ContactEntry> {
        self.entries.lock().unwrap().clone()
    }
}

impl Default for ContactStore {
    fn default() -> Self {
        Self::new()
    }
}

/// POST /api/v1/contact
#[post("/contact")]
pub async fn submit_contact(
    state: web::Data<crate::app_state::AppState>,
    body: web::Json<ContactRequest>,
) -> ApiResult<HttpResponse> {
    let trace = uuid::Uuid::new_v4().to_string();

    if body.name.trim().is_empty() || body.email.trim().is_empty() || body.message.trim().is_empty()
    {
        return Ok(HttpResponse::BadRequest().json(ApiResponse::<()>::error(
            ErrorDto {
                code: "VALIDATION_ERROR".into(),
                message: "name, email and message are required".into(),
                field: None,
            },
            400,
        )));
    }

    let entry = state.contact_store.submit(&body);
    Ok(HttpResponse::Created().json(ApiResponse::success(entry, trace)))
}

/// GET /api/v1/contact — admin only in strict mode
#[get("/contact")]
pub async fn list_contacts(
    req: HttpRequest,
    state: web::Data<crate::app_state::AppState>,
) -> ApiResult<HttpResponse> {
    let trace = uuid::Uuid::new_v4().to_string();

    if !crate::api::errors::acl_permissive() {
        let role = req
            .headers()
            .get("X-Msp-Role")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if role != "admin" {
            return Ok(HttpResponse::Forbidden().json(ApiResponse::<()>::error(
                ErrorDto {
                    code: "FORBIDDEN".into(),
                    message: "admin role required".into(),
                    field: None,
                },
                403,
            )));
        }
    }

    let entries = state.contact_store.list();
    Ok(HttpResponse::Ok().json(ApiResponse::success(entries, trace)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn submit_and_list() {
        let store = ContactStore::new();
        let req = ContactRequest {
            name: "Juan".into(),
            email: "juan@test.com".into(),
            organization: Some("Acme".into()),
            message: "Quiero una demo".into(),
        };
        let entry = store.submit(&req);
        assert_eq!(entry.id, 1);
        assert_eq!(entry.name, "Juan");

        let all = store.list();
        assert_eq!(all.len(), 1);
    }

    #[test]
    fn auto_increment_ids() {
        let store = ContactStore::new();
        let req = ContactRequest {
            name: "A".into(),
            email: "a@t.com".into(),
            organization: None,
            message: "msg".into(),
        };
        store.submit(&req);
        let second = store.submit(&req);
        assert_eq!(second.id, 2);
    }
}
