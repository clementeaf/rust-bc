//! API Tier (Tier 4): REST Gateway and HTTP Interface
//!
//! Responsibilities:
//! - REST API endpoint definitions
//! - Request/response serialization (JSON, binary)
//! - Parameter validation and error formatting
//! - JWT authentication and rate limiting
//! - API versioning and backward compatibility

use thiserror::Error;

pub mod handlers;
pub mod models;
pub mod errors;
pub mod middleware;
pub mod routes;
pub mod traits;
pub mod openapi;

pub use errors::{ApiError, ApiResult};
pub use routes::ApiRoutes;
pub use traits::ApiGateway;

/// API configuration
#[derive(Clone, Debug)]
pub struct ApiConfig {
    pub host: String,
    pub port: u16,
    pub rate_limit_per_minute: u32,
    pub max_request_size_bytes: usize,
    pub jwt_secret: String,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            rate_limit_per_minute: 1000,
            max_request_size_bytes: 10 * 1024 * 1024, // 10MB
            jwt_secret: "change-me-in-production".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let cfg = ApiConfig::default();
        assert_eq!(cfg.port, 8080);
    }
}
