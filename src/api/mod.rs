//! API Tier (Tier 4): REST Gateway and HTTP Interface
//!
//! Responsibilities:
//! - REST API endpoint definitions
//! - Request/response serialization (JSON, binary)
//! - Parameter validation and error formatting
//! - JWT authentication and rate limiting
//! - API versioning and backward compatibility

pub mod cors;
pub mod errors;
pub mod handlers;
pub mod metrics;
pub mod middleware;
pub mod models;
pub mod openapi;
pub mod pagination;
pub mod rate_limit;
pub mod routes;
pub mod traits;
pub mod versioning;

/// API configuration
#[allow(dead_code)] // Config struct fields read via from_env()
#[derive(Clone, Debug)]
pub struct ApiConfig {
    pub host: String,
    pub port: u16,
    pub rate_limit_per_minute: u32,
    pub max_request_size_bytes: usize,
    /// Reserved for future JWT bearer-token middleware. Currently loaded at
    /// startup and validated in production but **not used for request
    /// authentication** — mTLS + ACL is the active auth mechanism.
    pub jwt_secret: String,
}

/// Default JWT secret used only in development/test when `JWT_SECRET` is unset.
const DEV_JWT_SECRET: &str = "change-me-in-production";

impl ApiConfig {
    /// Build config from environment. Panics if `JWT_SECRET` is missing or
    /// matches the default value when `RUST_BC_ENV=production`.
    pub fn from_env() -> Self {
        let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| {
            log::warn!("JWT_SECRET not set — using insecure default (dev only)");
            DEV_JWT_SECRET.to_string()
        });

        let env_mode = std::env::var("RUST_BC_ENV").unwrap_or_default();
        if env_mode == "production" && jwt_secret == DEV_JWT_SECRET {
            panic!(
                "FATAL: JWT_SECRET must be set to a unique value in production. \
                 Set RUST_BC_ENV=development to use the insecure default."
            );
        }

        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            rate_limit_per_minute: 1000,
            max_request_size_bytes: 10 * 1024 * 1024,
            jwt_secret,
        }
    }
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self::from_env()
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
