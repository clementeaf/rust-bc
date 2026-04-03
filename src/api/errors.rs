use actix_web::{error::ResponseError, http::StatusCode, HttpResponse};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// API error types matching NeuroAccessMaui pattern
#[derive(Debug, Error, Clone)]
pub enum ApiError {
    #[error("Resource not found: {resource}")]
    NotFound { resource: String },

    #[error("Validation error: {field} - {reason}")]
    ValidationError { field: String, reason: String },

    #[error("Storage error: {reason}")]
    StorageError { reason: String },

    #[error("Consensus error: {reason}")]
    ConsensusError { reason: String },

    #[error("Cryptography error: {reason}")]
    CryptoError { reason: String },

    #[error("Conflict: {reason}")]
    Conflict { reason: String },

    #[error("Invalid DID format")]
    InvalidDid,

    #[error("Invalid signature")]
    InvalidSignature,

    #[error("Credential expired")]
    CredentialExpired,

    #[error("Credential revoked")]
    CredentialRevoked,

    #[error("Internal server error: {reason}")]
    InternalError { reason: String },

    #[error("Unauthorized")]
    Unauthorized,

    #[error("{message}")]
    UnauthorizedWithMessage { message: String },

    #[error("{message}")]
    PaymentRequired { message: String },

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
        let resp: ApiResponse<String> = ApiResponse::success("test".to_string(), "trace-1".to_string());
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
