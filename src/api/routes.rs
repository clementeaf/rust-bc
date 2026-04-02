use actix_web::{web, Scope};

use crate::api::handlers::{blocks, chain, transactions, utilities};

/// API routes configuration
pub struct ApiRoutes;

impl ApiRoutes {
    pub fn configure(cfg: &mut web::ServiceConfig) {
        cfg.service(
            web::scope("/api/v1")
                .service(Self::identity_routes())
                .service(Self::blocks_routes())
                .service(Self::chain_routes())
                .service(Self::credentials_routes())
                .service(Self::transaction_routes())
                .service(Self::utilities_routes()),
        );
    }

    fn identity_routes() -> Scope {
        web::scope("/identity")
    }

    fn blocks_routes() -> Scope {
        web::scope("/blocks")
            .service(blocks::create_block)
            .service(blocks::list_blocks)
            .service(blocks::get_block_by_index)
            .service(blocks::get_block_by_hash)
    }

    fn chain_routes() -> Scope {
        web::scope("/chain")
            .service(chain::verify_chain)
            .service(chain::get_blockchain_info)
    }

    fn credentials_routes() -> Scope {
        web::scope("/credentials")
    }

    fn transaction_routes() -> Scope {
        web::scope("")
            .service(transactions::create_transaction)
            .service(transactions::get_mempool)
    }

    fn utilities_routes() -> Scope {
        web::scope("")
            .service(utilities::health_check)
            .service(utilities::get_version)
            .service(utilities::get_openapi)
    }
}

#[cfg(test)]
mod tests {
    use super::ApiRoutes;

    #[test]
    fn test_routes_structure() {
        assert_eq!(std::mem::size_of::<ApiRoutes>(), 0);
    }
}
