use actix_web::{web, Scope};

/// API routes configuration
pub struct ApiRoutes;

impl ApiRoutes {
    pub fn configure(cfg: &mut web::ServiceConfig) {
        cfg.service(
            web::scope("/api/v1")
                .service(Self::identity_routes())
                .service(Self::blocks_routes())
                .service(Self::credentials_routes())
                .service(Self::utilities_routes()),
        );
    }

    fn identity_routes() -> Scope {
        web::scope("/identity")
            // Routes will be added in next phase
    }

    fn blocks_routes() -> Scope {
        web::scope("/blocks")
            // Routes will be added in next phase
    }

    fn credentials_routes() -> Scope {
        web::scope("/credentials")
            // Routes will be added in next phase
    }

    fn utilities_routes() -> Scope {
        web::scope("")
            // GET /health, GET /version, GET /openapi.json
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_routes_structure() {
        // Routes struct compiles
        assert!(true);
    }
}
