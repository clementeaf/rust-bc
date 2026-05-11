//! POST /api/v1/gateway/submit

use actix_web::{post, web, HttpRequest, HttpResponse};
use serde::{Deserialize, Serialize};

use crate::api::errors::{enforce_acl, ApiError, ApiResponse, ApiResult};
use crate::api::handlers::channels::enforce_channel_membership;
use crate::app_state::AppState;
use crate::gateway::TxResult;
use crate::storage::traits::Transaction;

// ── Request / response types ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct GatewaySubmitRequest {
    /// Chaincode to invoke (used to look up the endorsement policy).
    pub chaincode_id: String,
    /// Channel on which to endorse. When provided and a discovery service is
    /// configured, the gateway uses `endorsement_plan` instead of the local
    /// org registry to find endorsers.
    #[serde(default)]
    pub channel_id: String,
    /// The transaction to submit.
    pub transaction: TransactionBody,
}

#[derive(Debug, Deserialize)]
pub struct TransactionBody {
    pub id: String,
    pub input_did: String,
    pub output_recipient: String,
    pub amount: u64,
}

#[derive(Debug, Serialize)]
pub struct GatewaySubmitResponse {
    pub tx_id: String,
    pub block_height: u64,
    pub valid: bool,
}

impl From<TxResult> for GatewaySubmitResponse {
    fn from(r: TxResult) -> Self {
        Self {
            tx_id: r.tx_id,
            block_height: r.block_height,
            valid: r.valid,
        }
    }
}

// ── Handler ───────────────────────────────────────────────────────────────────

/// POST /api/v1/gateway/submit
///
/// Submits a transaction through the full endorse → order → commit pipeline.
/// Returns the committed block height and transaction ID.
#[post("/gateway/submit")]
pub async fn gateway_submit(
    http_req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<GatewaySubmitRequest>,
) -> ApiResult<HttpResponse> {
    enforce_acl(
        state.acl_provider.as_deref(),
        state.policy_store.as_deref(),
        "peer/ChaincodeToChaincode",
        &http_req,
    )?;

    let req = body.into_inner();

    // Channel membership check: reject if caller's org is not a member.
    if !req.channel_id.is_empty() {
        enforce_channel_membership(&state, &req.channel_id, &http_req)?;
    }

    if req.chaincode_id.is_empty() {
        return Err(ApiError::ValidationError {
            field: "chaincode_id".to_string(),
            reason: "must not be empty".to_string(),
        });
    }
    if req.transaction.id.is_empty() {
        return Err(ApiError::ValidationError {
            field: "transaction.id".to_string(),
            reason: "must not be empty".to_string(),
        });
    }

    let gw = state.gateway.as_ref().ok_or_else(|| ApiError::NotFound {
        resource: "gateway".to_string(),
    })?;

    let tx = Transaction {
        id: req.transaction.id,
        block_height: 0,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        input_did: req.transaction.input_did,
        output_recipient: req.transaction.output_recipient,
        amount: req.transaction.amount,
        state: "pending".to_string(),
    };

    let result = gw
        .submit(&req.chaincode_id, &req.channel_id, tx)
        .await
        .map_err(|e| ApiError::InternalError {
            reason: e.to_string(),
        })?;

    let trace_id = uuid::Uuid::new_v4().to_string();
    Ok(HttpResponse::Ok().json(ApiResponse::success(
        GatewaySubmitResponse::from(result),
        trace_id,
    )))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};
    use std::sync::Arc;

    use crate::endorsement::policy_store::MemoryPolicyStore;
    use crate::endorsement::registry::MemoryOrgRegistry;
    use crate::gateway::Gateway;
    use crate::ordering::service::OrderingService;
    use crate::storage::memory::MemoryStore;

    fn base_state(gateway: Option<Arc<Gateway>>) -> web::Data<AppState> {
        std::env::set_var("ACL_MODE", "permissive");
        let mut state = AppState::test_default();
        state.gateway = gateway;
        web::Data::new(state)
    }

    fn make_state_with_gateway() -> web::Data<AppState> {
        let gw = Gateway::new(
            Arc::new(MemoryOrgRegistry::new()),
            Arc::new(MemoryPolicyStore::new()),
            Arc::new(OrderingService::with_config(10, 500)),
            Arc::new(MemoryStore::new()),
        );
        base_state(Some(Arc::new(gw)))
    }

    fn make_state_without_gateway() -> web::Data<AppState> {
        base_state(None)
    }

    fn submit_body(tx_id: &str) -> serde_json::Value {
        serde_json::json!({
            "chaincode_id": "cc1",
            "transaction": {
                "id": tx_id,
                "input_did": "did:bc:alice",
                "output_recipient": "did:bc:bob",
                "amount": 10
            }
        })
    }

    #[actix_web::test]
    async fn submit_returns_200_and_block_height() {
        let state = make_state_with_gateway();
        let app = test::init_service(
            App::new()
                .app_data(state)
                .service(web::scope("/api/v1").service(gateway_submit)),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/v1/gateway/submit")
            .set_json(submit_body("tx-http-1"))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["data"]["tx_id"], "tx-http-1");
        assert_eq!(body["data"]["block_height"], 1);
    }

    #[actix_web::test]
    async fn submit_returns_404_when_gateway_not_configured() {
        let state = make_state_without_gateway();
        let app = test::init_service(
            App::new()
                .app_data(state)
                .service(web::scope("/api/v1").service(gateway_submit)),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/v1/gateway/submit")
            .set_json(submit_body("tx-x"))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 404);
    }

    #[actix_web::test]
    async fn submit_returns_400_when_chaincode_id_empty() {
        let state = make_state_with_gateway();
        let app = test::init_service(
            App::new()
                .app_data(state)
                .service(web::scope("/api/v1").service(gateway_submit)),
        )
        .await;

        let body = serde_json::json!({
            "chaincode_id": "",
            "transaction": {
                "id": "tx-1",
                "input_did": "did:bc:alice",
                "output_recipient": "did:bc:bob",
                "amount": 5
            }
        });

        let req = test::TestRequest::post()
            .uri("/api/v1/gateway/submit")
            .set_json(body)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);
    }
}
