/// OpenAPI specification and documentation
pub struct OpenApi;

impl OpenApi {
    /// Get OpenAPI specification as JSON (embedded `openapi.json`).
    pub fn spec() -> serde_json::Value {
        serde_json::from_str(include_str!("openapi.json")).expect("openapi.json must be valid JSON")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openapi_spec_generation() {
        let spec = OpenApi::spec();
        assert!(spec.is_object());
        assert_eq!(spec["openapi"], "3.0.3");
    }

    #[test]
    fn test_openapi_has_gateway_paths() {
        let spec = OpenApi::spec();
        let paths = spec["paths"].as_object().expect("paths");
        assert!(paths.contains_key("/mempool"));
        assert!(paths.contains_key("/transactions"));
        assert!(paths.contains_key("/blocks"));
        assert!(paths.contains_key("/health"));
        assert!(paths.contains_key("/gateway/submit"));
        assert!(paths.contains_key("/channels"));
        assert!(paths.contains_key("/audit/requests"));
        assert!(paths.contains_key("/audit/export"));
    }
}
