/// OpenAPI specification and documentation
pub struct OpenApi;

impl OpenApi {
    /// Get OpenAPI specification as JSON
    pub fn spec() -> serde_json::Value {
        serde_json::json!({
            "openapi": "3.0.0",
            "info": {
                "title": "rust-bc REST API",
                "version": "1.0.0",
                "description": "REST API Gateway for rust-bc blockchain"
            },
            "servers": [
                {
                    "url": "/api/v1",
                    "description": "Main API server"
                }
            ],
            "paths": {
                "/identity/create": {
                    "post": {
                        "summary": "Create a new DID and keypair",
                        "tags": ["Identity"],
                        "requestBody": {
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object"
                                    }
                                }
                            }
                        },
                        "responses": {
                            "200": {
                                "description": "DID created successfully"
                            }
                        }
                    }
                },
                "/health": {
                    "get": {
                        "summary": "Health check",
                        "tags": ["Utilities"],
                        "responses": {
                            "200": {
                                "description": "Service is healthy"
                            }
                        }
                    }
                },
                "/version": {
                    "get": {
                        "summary": "Get API version",
                        "tags": ["Utilities"],
                        "responses": {
                            "200": {
                                "description": "Version information"
                            }
                        }
                    }
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openapi_spec_generation() {
        let spec = OpenApi::spec();
        assert!(spec.is_object());
        assert_eq!(spec["openapi"], "3.0.0");
    }
}
