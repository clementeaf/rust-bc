//! EVM contract endpoints — deploy, call, and list Solidity contracts.

use crate::api::errors::{ApiResponse, ApiResult, ErrorDto};
use crate::evm_compat::executor::EvmExecutor;
use actix_web::{get, post, web, HttpResponse};
use std::sync::Mutex;

/// Shared EVM executor state.
pub struct EvmState {
    pub executor: Mutex<EvmExecutor>,
}

impl EvmState {
    pub fn new() -> Self {
        Self {
            executor: Mutex::new(EvmExecutor::new()),
        }
    }
}

#[derive(serde::Deserialize)]
pub struct DeployRequest {
    /// Hex-encoded init bytecode (with or without 0x prefix).
    pub bytecode: String,
}

#[derive(serde::Deserialize)]
pub struct CallRequest {
    /// Hex-encoded contract address.
    pub address: String,
    /// Hex-encoded calldata (ABI-encoded function call).
    #[serde(default)]
    pub calldata: String,
}

/// POST /evm/deploy — deploy a Solidity contract from init bytecode.
#[post("/evm/deploy")]
async fn evm_deploy(
    evm: web::Data<EvmState>,
    body: web::Json<DeployRequest>,
) -> ApiResult<HttpResponse> {
    let trace_id = uuid::Uuid::new_v4().to_string();
    let mut exec = evm.executor.lock().unwrap();

    match exec.deploy(&body.bytecode) {
        Ok(result) => {
            let resp = ApiResponse::success(result, trace_id);
            Ok(HttpResponse::Ok().json(resp))
        }
        Err(e) => {
            let resp = ApiResponse::<()>::error(ErrorDto { code: "EVM_ERROR".into(), message: e.to_string(), field: None }, 400);
            Ok(HttpResponse::BadRequest().json(resp))
        }
    }
}

/// POST /evm/call — call a deployed contract (state-mutating).
#[post("/evm/call")]
async fn evm_call(
    evm: web::Data<EvmState>,
    body: web::Json<CallRequest>,
) -> ApiResult<HttpResponse> {
    let trace_id = uuid::Uuid::new_v4().to_string();
    let mut exec = evm.executor.lock().unwrap();

    match exec.call(&body.address, &body.calldata) {
        Ok(result) => {
            let resp = ApiResponse::success(result, trace_id);
            Ok(HttpResponse::Ok().json(resp))
        }
        Err(e) => {
            let resp = ApiResponse::<()>::error(ErrorDto { code: "EVM_ERROR".into(), message: e.to_string(), field: None }, 400);
            Ok(HttpResponse::BadRequest().json(resp))
        }
    }
}

/// POST /evm/static-call — read-only call (no state mutation).
#[post("/evm/static-call")]
async fn evm_static_call(
    evm: web::Data<EvmState>,
    body: web::Json<CallRequest>,
) -> ApiResult<HttpResponse> {
    let trace_id = uuid::Uuid::new_v4().to_string();
    let mut exec = evm.executor.lock().unwrap();

    match exec.static_call(&body.address, &body.calldata) {
        Ok(result) => {
            let resp = ApiResponse::success(result, trace_id);
            Ok(HttpResponse::Ok().json(resp))
        }
        Err(e) => {
            let resp = ApiResponse::<()>::error(ErrorDto { code: "EVM_ERROR".into(), message: e.to_string(), field: None }, 400);
            Ok(HttpResponse::BadRequest().json(resp))
        }
    }
}

/// GET /evm/contracts — list all deployed contract addresses.
#[get("/evm/contracts")]
async fn evm_list_contracts(evm: web::Data<EvmState>) -> ApiResult<HttpResponse> {
    let trace_id = uuid::Uuid::new_v4().to_string();
    let exec = evm.executor.lock().unwrap();
    let contracts = exec.list_contracts();
    let resp = ApiResponse::success(contracts, trace_id);
    Ok(HttpResponse::Ok().json(resp))
}
