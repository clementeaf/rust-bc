use actix_web::{error::ResponseError, http::StatusCode, HttpMessage, HttpResponse};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Identity extracted from TLS client certificate.
///
/// Injected into request extensions by `TlsIdentityMiddleware` when mTLS is
/// enabled. `enforce_acl` reads this before falling back to `X-Org-Id` header.
#[derive(Debug, Clone)]
pub struct TlsIdentity {
    pub org_id: String,
    pub role: Option<crate::msp::MspRole>,
}

/// API error types matching NeuroAccessMaui pattern
#[derive(Debug, Error, Clone)]
pub enum ApiError {
    #[error("Resource not found: {resource}")]
    NotFound { resource: String },

    #[error("Validation error: {field} - {reason}")]
    ValidationError { field: String, reason: String },

    #[error("Storage error: {reason}")]
    StorageError { reason: String },

    #[allow(dead_code)]
    #[error("Consensus error: {reason}")]
    ConsensusError { reason: String },

    #[allow(dead_code)]
    #[error("Cryptography error: {reason}")]
    CryptoError { reason: String },

    #[error("Conflict: {reason}")]
    Conflict { reason: String },

    #[allow(dead_code)]
    #[error("Invalid DID format")]
    InvalidDid,

    #[allow(dead_code)]
    #[error("Invalid signature")]
    InvalidSignature,

    #[allow(dead_code)]
    #[error("Credential expired")]
    CredentialExpired,

    #[allow(dead_code)]
    #[error("Credential revoked")]
    CredentialRevoked,

    #[error("Internal server error: {reason}")]
    InternalError { reason: String },

    #[allow(dead_code)]
    #[error("Unauthorized")]
    Unauthorized,

    #[error("{message}")]
    UnauthorizedWithMessage { message: String },

    #[error("{message}")]
    PaymentRequired { message: String },

    #[allow(dead_code)]
    #[error("Rate limited")]
    RateLimited,

    #[error("Forbidden: {reason}")]
    Forbidden { reason: String },
}

impl ApiError {
    /// Get error code for API response
    pub fn code(&self) -> String {
        match self {
            ApiError::NotFound { .. } => "NOT_FOUND".to_string(),
            ApiError::ValidationError { .. } => "VALIDATION_ERROR".to_string(),
            ApiError::StorageError { .. } => "STORAGE_ERROR".to_string(),
            ApiError::ConsensusError { .. } => "CONSENSUS_ERROR".to_string(),
            ApiError::CryptoError { .. } => "CRYPTO_ERROR".to_string(),
            ApiError::Conflict { .. } => "CONFLICT".to_string(),
            ApiError::InvalidDid => "INVALID_DID".to_string(),
            ApiError::InvalidSignature => "INVALID_SIGNATURE".to_string(),
            ApiError::CredentialExpired => "CREDENTIAL_EXPIRED".to_string(),
            ApiError::CredentialRevoked => "CREDENTIAL_REVOKED".to_string(),
            ApiError::InternalError { .. } => "INTERNAL_ERROR".to_string(),
            ApiError::Unauthorized => "UNAUTHORIZED".to_string(),
            ApiError::UnauthorizedWithMessage { .. } => "UNAUTHORIZED".to_string(),
            ApiError::PaymentRequired { .. } => "PAYMENT_REQUIRED".to_string(),
            ApiError::RateLimited => "RATE_LIMITED".to_string(),
            ApiError::Forbidden { .. } => "FORBIDDEN".to_string(),
        }
    }

    /// Get field name for field-specific errors
    pub fn field(&self) -> Option<String> {
        match self {
            ApiError::ValidationError { field, .. } => Some(field.clone()),
            _ => None,
        }
    }

    /// Get HTTP status code
    pub fn status_code(&self) -> StatusCode {
        match self {
            ApiError::NotFound { .. } => StatusCode::NOT_FOUND,
            ApiError::ValidationError { .. } => StatusCode::BAD_REQUEST,
            ApiError::InvalidDid | ApiError::InvalidSignature => StatusCode::BAD_REQUEST,
            ApiError::Unauthorized | ApiError::UnauthorizedWithMessage { .. } => {
                StatusCode::UNAUTHORIZED
            }
            ApiError::PaymentRequired { .. } => StatusCode::PAYMENT_REQUIRED,
            ApiError::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            ApiError::Forbidden { .. } => StatusCode::FORBIDDEN,
            ApiError::Conflict { .. } => StatusCode::CONFLICT,
            ApiError::CredentialExpired | ApiError::CredentialRevoked => StatusCode::BAD_REQUEST,
            ApiError::StorageError { .. }
            | ApiError::ConsensusError { .. }
            | ApiError::CryptoError { .. }
            | ApiError::InternalError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        let status = self.status_code();
        let error_dto = ErrorDto {
            code: self.code(),
            message: self.to_string(),
            field: self.field(),
        };

        let response: ApiResponse<()> = ApiResponse::error(error_dto, status.as_u16() as i32);

        HttpResponse::build(status).json(response)
    }
}

/// Result type for API operations
pub type ApiResult<T> = Result<T, ApiError>;

/// Whether ACL enforcement runs in permissive mode (log-only, never deny).
///
/// Set `ACL_MODE=permissive` to allow all requests regardless of identity.
/// Any other value (or absent) defaults to **strict** mode where missing
/// identity, missing ACL entries, and missing policies all result in denial.
pub fn acl_permissive() -> bool {
    std::env::var("ACL_MODE")
        .map(|v| v.eq_ignore_ascii_case("permissive"))
        .unwrap_or(false)
}

/// Enforce ACL for a resource.
///
/// In **strict** mode (default): missing identity headers, missing ACL
/// infrastructure, and undefined ACL entries all result in `Forbidden`.
/// In **permissive** mode (`ACL_MODE=permissive`): access is always granted
/// (useful for local development and bootstrapping).
///
/// Returns `ApiError::Forbidden` when the caller's org does not satisfy the
/// policy bound to the resource.
/// Return the minimum MSP role required for a given resource.
///
/// Admin resources require `Admin`, write resources require `Client` or `Peer`,
/// and read resources return `None` (any role allowed).
fn required_role_for_resource(resource: &str) -> Option<crate::msp::MspRole> {
    use crate::msp::MspRole;
    match resource {
        // Admin operations
        "peer/Admin"
        | "peer/MSP.Admin"
        | "peer/Discovery.Admin"
        | "qscc/Snapshot.Admin"
        | "peer/ChannelConfig" => Some(MspRole::Admin),
        // Writer operations — Client or Peer role required
        "peer/Propose"
        | "peer/Identity"
        | "peer/ChaincodeToChaincode"
        | "peer/PrivateData.Write" => Some(MspRole::Client),
        // Everything else (reads) — no role requirement
        _ => None,
    }
}

/// Check whether `caller_role` satisfies `required_role`.
///
/// Admin satisfies any requirement. Client/Peer satisfy Client-level requirements.
fn role_satisfies(caller_role: crate::msp::MspRole, required: crate::msp::MspRole) -> bool {
    use crate::msp::MspRole;
    match required {
        MspRole::Admin => caller_role == MspRole::Admin,
        MspRole::Client | MspRole::Peer => matches!(
            caller_role,
            MspRole::Admin | MspRole::Client | MspRole::Peer
        ),
        _ => true,
    }
}

pub fn enforce_acl(
    acl_provider: Option<&dyn crate::acl::AclProvider>,
    policy_store: Option<&dyn crate::endorsement::policy_store::PolicyStore>,
    resource: &str,
    request: &actix_web::HttpRequest,
) -> Result<(), ApiError> {
    // ── MSP role check ───────────────────────────────────────────────────
    if let Some(required) = required_role_for_resource(resource) {
        // Try TLS-derived role first, then X-Msp-Role header.
        let tls_role = {
            let ext = request.extensions();
            ext.get::<TlsIdentity>().and_then(|id| id.role)
        };
        let caller_role_str = request
            .headers()
            .get("X-Msp-Role")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if let Some(tls_r) = tls_role {
            if !role_satisfies(tls_r, required) {
                return Err(ApiError::Forbidden {
                    reason: format!(
                        "TLS identity role '{tls_r:?}' insufficient for resource '{resource}' (requires '{required:?}')"
                    ),
                });
            }
            // TLS role satisfied — skip header check.
        } else if !caller_role_str.is_empty() {
            match serde_json::from_str::<crate::msp::MspRole>(&format!("\"{caller_role_str}\"")) {
                Ok(caller_role) => {
                    if !role_satisfies(caller_role, required) {
                        return Err(ApiError::Forbidden {
                            reason: format!(
                                "MSP role '{caller_role_str}' insufficient for resource '{resource}' (requires '{required:?}')"
                            ),
                        });
                    }
                }
                Err(_) => {
                    return Err(ApiError::Forbidden {
                        reason: format!("invalid X-Msp-Role header: '{caller_role_str}'"),
                    });
                }
            }
        }
        // No TLS identity and no X-Msp-Role header.
        if !acl_permissive() {
            return Err(ApiError::Forbidden {
                reason: format!(
                    "missing X-Msp-Role header for resource '{resource}' (requires '{required:?}')"
                ),
            });
        }
    }

    // ── Org-based ACL check ──────────────────────────────────────────────
    let (Some(acl), Some(ps)) = (acl_provider, policy_store) else {
        if acl_permissive() {
            return Ok(()); // Permissive mode — no ACL infrastructure → allow
        }
        return Err(ApiError::Forbidden {
            reason: format!(
                "ACL infrastructure not configured; cannot authorize resource '{resource}'"
            ),
        });
    };

    // Try TLS-derived identity first (set by TLS identity middleware),
    // then fall back to X-Org-Id header.
    let tls_org = {
        let ext = request.extensions();
        ext.get::<TlsIdentity>().map(|id| id.org_id.clone())
    };
    let header_org;
    let caller_org = if let Some(ref org) = tls_org {
        org.as_str()
    } else {
        header_org = request
            .headers()
            .get("X-Org-Id")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        header_org
    };

    if caller_org.is_empty() && !acl_permissive() {
        return Err(ApiError::Forbidden {
            reason: format!(
                "missing caller identity (X-Org-Id header or TLS cert) for resource '{resource}'"
            ),
        });
    }
    let orgs: Vec<&str> = if caller_org.is_empty() {
        vec![]
    } else {
        vec![caller_org]
    };

    match crate::acl::check_access(acl, ps, resource, &orgs) {
        Ok(()) => Ok(()),
        Err(crate::acl::AclError::NotDefined(_)) if acl_permissive() => Ok(()),
        Err(crate::acl::AclError::PolicyNotFound(_)) if acl_permissive() => Ok(()),
        Err(crate::acl::AclError::NotDefined(r)) => Err(ApiError::Forbidden {
            reason: format!("no ACL entry defined for resource '{r}'"),
        }),
        Err(crate::acl::AclError::PolicyNotFound(p)) => Err(ApiError::Forbidden {
            reason: format!("ACL policy '{p}' not found for resource '{resource}'"),
        }),
        Err(crate::acl::AclError::Denied(policy)) => Err(ApiError::Forbidden {
            reason: format!(
                "ACL denied: resource '{resource}', policy '{policy}', org '{caller_org}'"
            ),
        }),
    }
}

/// Generic API response wrapper (NeuroAccessMaui pattern)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub status: String,
    pub status_code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorDto>,
    pub timestamp: String,
    pub trace_id: String,
}

impl<T: Serialize> ApiResponse<T> {
    /// Create successful response
    pub fn success(data: T, trace_id: String) -> Self {
        Self {
            status: "Success".to_string(),
            status_code: 200,
            message: "OK".to_string(),
            data: Some(data),
            error: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
            trace_id,
        }
    }

    /// Create successful response with custom status code
    pub fn success_with_code(data: T, status_code: i32, trace_id: String) -> Self {
        Self {
            status: "Success".to_string(),
            status_code,
            message: "OK".to_string(),
            data: Some(data),
            error: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
            trace_id,
        }
    }

    /// Create error response
    pub fn error(error: ErrorDto, status_code: i32) -> ApiResponse<T> {
        ApiResponse {
            status: "Failure".to_string(),
            status_code,
            message: error.message.clone(),
            data: None,
            error: Some(error),
            timestamp: chrono::Utc::now().to_rfc3339(),
            trace_id: uuid::Uuid::new_v4().to_string(),
        }
    }
}

/// Error detail DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDto {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_generation() {
        let err = ApiError::NotFound {
            resource: "DID".to_string(),
        };
        assert_eq!(err.code(), "NOT_FOUND");
    }

    #[test]
    fn test_error_status_codes() {
        assert_eq!(
            ApiError::NotFound {
                resource: "test".to_string()
            }
            .status_code(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            ApiError::ValidationError {
                field: "test".to_string(),
                reason: "invalid".to_string()
            }
            .status_code(),
            StatusCode::BAD_REQUEST
        );
    }

    #[test]
    fn test_validation_error_field() {
        let err = ApiError::ValidationError {
            field: "phoneNumber".to_string(),
            reason: "invalid format".to_string(),
        };
        assert_eq!(err.field(), Some("phoneNumber".to_string()));
    }

    #[test]
    fn test_api_response_success() {
        let resp: ApiResponse<String> =
            ApiResponse::success("test".to_string(), "trace-1".to_string());
        assert_eq!(resp.status, "Success");
        assert_eq!(resp.status_code, 200);
        assert_eq!(resp.data, Some("test".to_string()));
    }

    #[test]
    fn test_api_response_error() {
        let err_dto = ErrorDto {
            code: "TEST_ERROR".to_string(),
            message: "Test error message".to_string(),
            field: Some("testField".to_string()),
        };
        let resp: ApiResponse<()> = ApiResponse::error(err_dto, 400);
        assert_eq!(resp.status, "Failure");
        assert_eq!(resp.status_code, 400);
        assert!(resp.error.is_some());
    }
}
