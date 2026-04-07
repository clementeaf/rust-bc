//! Unified chaincode invocation trait.
//!
//! `ChaincodeInvoker` abstracts over Wasm and external chaincode execution,
//! allowing the Gateway to invoke chaincodes without caring about the runtime.

use std::sync::Arc;

use crate::chaincode::ChaincodeError;
use crate::storage::world_state::WorldState;

/// Unified interface for invoking a chaincode regardless of its runtime.
pub trait ChaincodeInvoker: Send + Sync {
    fn invoke(
        &self,
        state: Arc<dyn WorldState>,
        func_name: &str,
    ) -> Result<Vec<u8>, ChaincodeError>;
}

/// Wraps a `WasmExecutor` as a `ChaincodeInvoker`.
pub struct WasmInvoker {
    executor: crate::chaincode::executor::WasmExecutor,
}

impl WasmInvoker {
    pub fn new(executor: crate::chaincode::executor::WasmExecutor) -> Self {
        Self { executor }
    }
}

impl ChaincodeInvoker for WasmInvoker {
    fn invoke(
        &self,
        state: Arc<dyn WorldState>,
        func_name: &str,
    ) -> Result<Vec<u8>, ChaincodeError> {
        self.executor.invoke(state, func_name)
    }
}

/// Wraps an `ExternalChaincodeClient` as a `ChaincodeInvoker`.
///
/// Since external invocations are async but the trait is sync, this
/// invoker uses `block_in_place` + `Handle::block_on` to bridge without
/// blocking other async worker threads.
pub struct ExternalInvoker {
    client: crate::chaincode::external::ExternalChaincodeClient,
}

impl ExternalInvoker {
    pub fn new(client: crate::chaincode::external::ExternalChaincodeClient) -> Self {
        Self { client }
    }
}

impl ChaincodeInvoker for ExternalInvoker {
    fn invoke(
        &self,
        _state: Arc<dyn WorldState>,
        func_name: &str,
    ) -> Result<Vec<u8>, ChaincodeError> {
        // External chaincode receives state context as a string identifier;
        // the actual state operations happen on the external service side.
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(self.client.invoke(func_name, &[], ""))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::world_state::MemoryWorldState;

    // WasmInvoker test uses a minimal WAT that returns empty.
    const NOOP_WAT: &[u8] = br#"
(module
  (import "env" "put_state" (func $put (param i32 i32 i32 i32) (result i32)))
  (import "env" "get_state" (func $get (param i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (func (export "run") (result i64) (i64.const 0))
)
"#;

    #[test]
    fn wasm_invoker_via_trait_object() {
        let executor = crate::chaincode::executor::WasmExecutor::new(NOOP_WAT, 1_000_000).unwrap();
        let invoker: Box<dyn ChaincodeInvoker> = Box::new(WasmInvoker::new(executor));
        let state = Arc::new(MemoryWorldState::new());
        let result = invoker.invoke(state, "run").unwrap();
        assert!(result.is_empty());
    }
}
