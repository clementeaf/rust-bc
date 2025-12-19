use actix_web::http::Method;

/// CORS policy configuration
#[derive(Clone, Debug)]
pub struct CorsPolicy {
    /// Allowed origins (e.g., "https://example.com", or "*" for all)
    pub allowed_origins: Vec<String>,
    /// Allowed HTTP methods
    pub allowed_methods: Vec<Method>,
    /// Allowed headers
    pub allowed_headers: Vec<String>,
    /// Exposed headers
    pub exposed_headers: Vec<String>,
    /// Whether credentials (cookies, auth headers) are allowed
    pub allow_credentials: bool,
    /// Max age for preflight cache in seconds
    pub max_age: u32,
}

impl Default for CorsPolicy {
    fn default() -> Self {
        Self {
            allowed_origins: vec!["*".to_string()],
            allowed_methods: vec![
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::DELETE,
                Method::PATCH,
                Method::OPTIONS,
            ],
            allowed_headers: vec![
                "content-type".to_string(),
                "authorization".to_string(),
                "x-api-version".to_string(),
                "x-trace-id".to_string(),
            ],
            exposed_headers: vec![
                "x-api-version".to_string(),
                "x-trace-id".to_string(),
                "x-ratelimit-limit".to_string(),
                "x-ratelimit-remaining".to_string(),
            ],
            allow_credentials: false,
            max_age: 3600,
        }
    }
}

impl CorsPolicy {
    /// Create a new CORS policy
    pub fn new() -> Self {
        Self::default()
    }

    /// Set allowed origins
    pub fn with_origins(mut self, origins: Vec<String>) -> Self {
        self.allowed_origins = origins;
        self
    }

    /// Set allowed methods
    pub fn with_methods(mut self, methods: Vec<Method>) -> Self {
        self.allowed_methods = methods;
        self
    }

    /// Set allowed headers
    pub fn with_headers(mut self, headers: Vec<String>) -> Self {
        self.allowed_headers = headers;
        self
    }

    /// Set exposed headers
    pub fn with_exposed_headers(mut self, headers: Vec<String>) -> Self {
        self.exposed_headers = headers;
        self
    }

    /// Enable credentials
    pub fn allow_credentials(mut self, allow: bool) -> Self {
        self.allow_credentials = allow;
        self
    }

    /// Set max age for preflight cache
    pub fn with_max_age(mut self, seconds: u32) -> Self {
        self.max_age = seconds;
        self
    }

    /// Check if origin is allowed
    pub fn is_origin_allowed(&self, origin: &str) -> bool {
        if self.allowed_origins.contains(&"*".to_string()) {
            return true;
        }
        self.allowed_origins.contains(&origin.to_string())
    }

    /// Build CORS headers for response
    pub fn build_headers(&self, origin: &str) -> Vec<(String, String)> {
        let mut headers = Vec::new();

        if self.is_origin_allowed(origin) {
            headers.push((
                "Access-Control-Allow-Origin".to_string(),
                if self.allowed_origins.contains(&"*".to_string()) {
                    "*".to_string()
                } else {
                    origin.to_string()
                },
            ));
        }

        let methods: Vec<String> = self
            .allowed_methods
            .iter()
            .map(|m| m.to_string())
            .collect();
        headers.push((
            "Access-Control-Allow-Methods".to_string(),
            methods.join(", "),
        ));

        headers.push((
            "Access-Control-Allow-Headers".to_string(),
            self.allowed_headers.join(", "),
        ));

        headers.push((
            "Access-Control-Expose-Headers".to_string(),
            self.exposed_headers.join(", "),
        ));

        if self.allow_credentials {
            headers.push((
                "Access-Control-Allow-Credentials".to_string(),
                "true".to_string(),
            ));
        }

        headers.push((
            "Access-Control-Max-Age".to_string(),
            self.max_age.to_string(),
        ));

        headers
    }
}

/// CORS handler for preflight OPTIONS requests
pub fn handle_preflight_request(
    origin: Option<&str>,
    cors_policy: &CorsPolicy,
) -> Vec<(String, String)> {
    let origin = origin.unwrap_or("*");
    cors_policy.build_headers(origin)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cors_policy_default() {
        let policy = CorsPolicy::default();
        assert!(policy.allowed_origins.contains(&"*".to_string()));
        assert!(policy.allowed_methods.contains(&Method::GET));
        assert!(policy.allowed_methods.contains(&Method::POST));
        assert!(!policy.allow_credentials);
    }

    #[test]
    fn test_cors_policy_builder() {
        let policy = CorsPolicy::new()
            .with_origins(vec!["https://example.com".to_string()])
            .allow_credentials(true)
            .with_max_age(7200);

        assert!(policy.is_origin_allowed("https://example.com"));
        assert!(!policy.is_origin_allowed("https://other.com"));
        assert!(policy.allow_credentials);
        assert_eq!(policy.max_age, 7200);
    }

    #[test]
    fn test_cors_wildcard_origin() {
        let policy = CorsPolicy::default();
        assert!(policy.is_origin_allowed("https://example.com"));
        assert!(policy.is_origin_allowed("https://other.com"));
        assert!(policy.is_origin_allowed("*"));
    }

    #[test]
    fn test_cors_specific_origins() {
        let policy = CorsPolicy::new().with_origins(vec![
            "https://example.com".to_string(),
            "https://app.example.com".to_string(),
        ]);

        assert!(policy.is_origin_allowed("https://example.com"));
        assert!(policy.is_origin_allowed("https://app.example.com"));
        assert!(!policy.is_origin_allowed("https://malicious.com"));
    }

    #[test]
    fn test_build_cors_headers_wildcard() {
        let policy = CorsPolicy::default();
        let headers = policy.build_headers("https://example.com");

        assert!(headers
            .iter()
            .any(|(k, v)| k == "Access-Control-Allow-Origin" && v == "*"));
        assert!(headers
            .iter()
            .any(|(k, v)| k == "Access-Control-Allow-Methods"));
        assert!(headers
            .iter()
            .any(|(k, v)| k == "Access-Control-Allow-Headers"));
    }

    #[test]
    fn test_build_cors_headers_specific_origin() {
        let policy = CorsPolicy::new().with_origins(vec!["https://example.com".to_string()]);
        let headers = policy.build_headers("https://example.com");

        assert!(headers.iter().any(|(k, v)| k == "Access-Control-Allow-Origin"
            && v == "https://example.com"));
    }

    #[test]
    fn test_cors_headers_include_custom_headers() {
        let policy = CorsPolicy::default();
        let headers = policy.build_headers("*");

        let exposed = headers
            .iter()
            .find(|(k, _)| k == "Access-Control-Expose-Headers")
            .map(|(_, v)| v.clone());

        assert!(exposed
            .as_ref()
            .map(|h| h.contains("x-api-version"))
            .unwrap_or(false));
    }

    #[test]
    fn test_cors_credentials_header() {
        let policy = CorsPolicy::new().allow_credentials(true);
        let headers = policy.build_headers("https://example.com");

        assert!(headers.iter().any(|(k, v)| k == "Access-Control-Allow-Credentials"
            && v == "true"));
    }

    #[test]
    fn test_cors_max_age_header() {
        let policy = CorsPolicy::new().with_max_age(7200);
        let headers = policy.build_headers("*");

        assert!(headers
            .iter()
            .any(|(k, v)| k == "Access-Control-Max-Age" && v == "7200"));
    }

    #[test]
    fn test_handle_preflight_request() {
        let policy = CorsPolicy::default();
        let headers = handle_preflight_request(Some("https://example.com"), &policy);

        assert!(!headers.is_empty());
        assert!(headers
            .iter()
            .any(|(k, _)| k == "Access-Control-Allow-Methods"));
    }

    #[test]
    fn test_handle_preflight_request_no_origin() {
        let policy = CorsPolicy::default();
        let headers = handle_preflight_request(None, &policy);

        assert!(!headers.is_empty());
    }
}
