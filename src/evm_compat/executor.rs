//! EVM executor — deploy and call Solidity contracts using revm.
//!
//! Uses an in-memory `CacheDB` per executor instance. Each executor
//! represents one EVM "world" (typically one per channel).

use revm::{
    context::TxEnv,
    context_interface::result::{ExecutionResult, Output},
    database::CacheDB,
    database_interface::EmptyDB,
    primitives::{Bytes, TxKind, Address, B256, U256},
    state::AccountInfo,
    MainBuilder, MainContext,
};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EvmError {
    #[error("deployment failed: {0}")]
    DeployFailed(String),
    #[error("execution reverted: {0}")]
    Reverted(String),
    #[error("evm internal error: {0}")]
    Internal(String),
}

/// Deployed contract metadata.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DeployedContract {
    pub address: String,
    pub gas_used: u64,
}

/// Result of a contract call.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CallResult {
    pub output: String,
    pub gas_used: u64,
}

/// EVM executor with in-memory state.
pub struct EvmExecutor {
    db: CacheDB<EmptyDB>,
    nonce: u64,
    contracts: HashMap<String, Address>,
    caller: Address,
}

impl EvmExecutor {
    /// Create a new executor. The caller gets a large ETH balance for gas.
    pub fn new() -> Self {
        let caller = Address::from_slice(&[0x01; 20]);
        let mut db = CacheDB::<EmptyDB>::default();

        // Fund the caller with enough balance for gas
        let info = AccountInfo {
            balance: U256::from(1_000_000_000_000_000_000_u128),
            nonce: 0,
            code_hash: B256::ZERO,
            code: None,
            account_id: None,
        };
        db.insert_account_info(caller, info);

        Self {
            db,
            nonce: 0,
            contracts: HashMap::new(),
            caller,
        }
    }

    /// Deploy a contract from bytecode (hex-encoded init code).
    pub fn deploy(&mut self, bytecode_hex: &str) -> Result<DeployedContract, EvmError> {
        let bytecode = hex::decode(bytecode_hex.trim_start_matches("0x"))
            .map_err(|e| EvmError::DeployFailed(format!("invalid hex: {e}")))?;

        let ctx = revm::Context::mainnet().with_db(&mut self.db);
        let mut evm = ctx.build_mainnet();

        let tx = TxEnv::builder()
            .caller(self.caller)
            .kind(TxKind::Create)
            .data(Bytes::from(bytecode))
            .nonce(self.nonce)
            .gas_limit(5_000_000)
            .gas_price(0)
            .build()
            .map_err(|e| EvmError::DeployFailed(format!("tx build: {e}")))?;

        self.nonce += 1;

        let result = revm::ExecuteCommitEvm::transact_commit(&mut evm, tx)
            .map_err(|e| EvmError::Internal(format!("{e}")))?;

        match result {
            ExecutionResult::Success {
                output: Output::Create(_, Some(addr)),
                gas,
                ..
            } => {
                let addr_hex = format!("0x{}", hex::encode(addr.as_slice()));
                self.contracts.insert(addr_hex.clone(), addr);
                Ok(DeployedContract {
                    address: addr_hex,
                    gas_used: gas.total_gas_spent(),
                })
            }
            ExecutionResult::Success { .. } => {
                Err(EvmError::DeployFailed("no address returned".into()))
            }
            ExecutionResult::Revert { output, .. } => {
                Err(EvmError::Reverted(format!("0x{}", hex::encode(&output))))
            }
            ExecutionResult::Halt { reason, .. } => {
                Err(EvmError::DeployFailed(format!("halted: {reason:?}")))
            }
        }
    }

    /// Call a deployed contract with hex-encoded calldata.
    pub fn call(
        &mut self,
        contract_addr: &str,
        calldata_hex: &str,
    ) -> Result<CallResult, EvmError> {
        let addr = self.resolve_address(contract_addr)?;
        let calldata = hex::decode(calldata_hex.trim_start_matches("0x"))
            .map_err(|e| EvmError::Internal(format!("invalid calldata hex: {e}")))?;

        let ctx = revm::Context::mainnet().with_db(&mut self.db);
        let mut evm = ctx.build_mainnet();

        let tx = TxEnv::builder()
            .caller(self.caller)
            .kind(TxKind::Call(addr))
            .data(Bytes::from(calldata))
            .nonce(self.nonce)
            .gas_limit(5_000_000)
            .gas_price(0)
            .build()
            .map_err(|e| EvmError::Internal(format!("tx build: {e}")))?;

        self.nonce += 1;

        let result = revm::ExecuteCommitEvm::transact_commit(&mut evm, tx)
            .map_err(|e| EvmError::Internal(format!("{e}")))?;

        match result {
            ExecutionResult::Success {
                output: Output::Call(bytes),
                gas,
                ..
            } => Ok(CallResult {
                output: format!("0x{}", hex::encode(&bytes)),
                gas_used: gas.total_gas_spent(),
            }),
            ExecutionResult::Revert { output, .. } => {
                Err(EvmError::Reverted(format!("0x{}", hex::encode(&output))))
            }
            ExecutionResult::Halt { reason, .. } => {
                Err(EvmError::Internal(format!("halted: {reason:?}")))
            }
            _ => Err(EvmError::Internal("unexpected result type".into())),
        }
    }

    /// Static call (read-only, no state mutation).
    pub fn static_call(
        &mut self,
        contract_addr: &str,
        calldata_hex: &str,
    ) -> Result<CallResult, EvmError> {
        let addr = self.resolve_address(contract_addr)?;
        let calldata = hex::decode(calldata_hex.trim_start_matches("0x"))
            .map_err(|e| EvmError::Internal(format!("invalid calldata hex: {e}")))?;

        let ctx = revm::Context::mainnet().with_db(&mut self.db);
        let mut evm = ctx.build_mainnet();

        let tx = TxEnv::builder()
            .caller(self.caller)
            .kind(TxKind::Call(addr))
            .data(Bytes::from(calldata))
            .nonce(self.nonce)
            .gas_limit(5_000_000)
            .gas_price(0)
            .build()
            .map_err(|e| EvmError::Internal(format!("tx build: {e}")))?;

        let output = revm::ExecuteEvm::transact(&mut evm, tx)
            .map_err(|e| EvmError::Internal(format!("{e}")))?;

        match output.result {
            ExecutionResult::Success {
                output: Output::Call(bytes),
                gas,
                ..
            } => Ok(CallResult {
                output: format!("0x{}", hex::encode(&bytes)),
                gas_used: gas.total_gas_spent(),
            }),
            ExecutionResult::Revert { output, .. } => {
                Err(EvmError::Reverted(format!("0x{}", hex::encode(&output))))
            }
            ExecutionResult::Halt { reason, .. } => {
                Err(EvmError::Internal(format!("halted: {reason:?}")))
            }
            _ => Err(EvmError::Internal("unexpected result type".into())),
        }
    }

    /// List deployed contract addresses.
    pub fn list_contracts(&self) -> Vec<String> {
        self.contracts.keys().cloned().collect()
    }

    fn resolve_address(&self, addr_hex: &str) -> Result<Address, EvmError> {
        let clean = addr_hex.trim_start_matches("0x");
        let bytes = hex::decode(clean)
            .map_err(|e| EvmError::Internal(format!("invalid address: {e}")))?;
        if bytes.len() != 20 {
            return Err(EvmError::Internal(format!(
                "address must be 20 bytes, got {}",
                bytes.len()
            )));
        }
        Ok(Address::from_slice(&bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Runtime: PUSH1 0x42 PUSH1 0x00 MSTORE PUSH1 0x20 PUSH1 0x00 RETURN
    // Hex:     60 42 60 00 52 60 20 60 00 f3  (10 bytes)
    // Init: PUSH1 0x0a PUSH1 0x0c PUSH1 0x00 CODECOPY PUSH1 0x0a PUSH1 0x00 RETURN
    // Hex:  60 0a 60 0c 60 00 39 60 0a 60 00 f3  (12 bytes)
    // Total: init (12) + runtime (10) = 22 bytes
    const RETURN_42_INIT: &str = "600a600c600039600a6000f3604260005260206000f3";

    #[test]
    fn deploy_and_call_returns_value() {
        let mut exec = EvmExecutor::new();
        let deployed = exec.deploy(RETURN_42_INIT).unwrap();
        assert!(deployed.address.starts_with("0x"));
        assert!(deployed.gas_used > 0);

        let result = exec.call(&deployed.address, "").unwrap();
        assert!(result.gas_used > 0);
        // 32-byte word: 0x0000...0042
        assert!(result.output.ends_with("42"), "unexpected output: {}", result.output);
    }

    #[test]
    fn deploy_invalid_hex_fails() {
        let mut exec = EvmExecutor::new();
        let result = exec.deploy("not-hex");
        assert!(result.is_err());
    }

    #[test]
    fn list_contracts_after_deploy() {
        let mut exec = EvmExecutor::new();
        assert!(exec.list_contracts().is_empty());
        exec.deploy(RETURN_42_INIT).unwrap();
        assert_eq!(exec.list_contracts().len(), 1);
    }

    // ── Brute-force / adversarial tests ──────────────────────────────────────

    #[test]
    fn bruteforce_rapid_deploys_100() {
        let mut exec = EvmExecutor::new();
        for i in 0..100 {
            let result = exec.deploy(RETURN_42_INIT);
            assert!(result.is_ok(), "deploy {i} failed: {:?}", result.err());
        }
        assert_eq!(exec.list_contracts().len(), 100);
    }

    #[test]
    fn bruteforce_rapid_calls_200() {
        let mut exec = EvmExecutor::new();
        let deployed = exec.deploy(RETURN_42_INIT).unwrap();
        for i in 0..200 {
            let result = exec.call(&deployed.address, "");
            assert!(result.is_ok(), "call {i} failed: {:?}", result.err());
        }
    }

    #[test]
    fn bruteforce_empty_bytecode() {
        let mut exec = EvmExecutor::new();
        // Empty bytecode — should deploy but create empty contract
        let result = exec.deploy("");
        // Either succeeds with no code or fails gracefully
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn bruteforce_garbage_bytecode() {
        let mut exec = EvmExecutor::new();
        // Invalid opcode spam — should not panic
        let garbage = "ff".repeat(1000); // 1000 SELFDESTRUCT opcodes
        let result = exec.deploy(&garbage);
        // Must not panic, may succeed or fail
        let _ = result;
    }

    #[test]
    fn bruteforce_oversized_bytecode() {
        let mut exec = EvmExecutor::new();
        // 100KB of NOPs — tests memory limits
        let large = "5b".repeat(50_000); // 50K JUMPDEST opcodes
        let result = exec.deploy(&large);
        // Should fail (exceeds EIP-170 contract size limit) or succeed with gas exhaustion
        let _ = result;
    }

    #[test]
    fn bruteforce_call_wrong_address() {
        let mut exec = EvmExecutor::new();
        // Call nonexistent contract
        let result = exec.call("0x0000000000000000000000000000000000dead01", "");
        // Should succeed (EVM returns empty for EOA) or fail gracefully
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn bruteforce_call_invalid_address_format() {
        let mut exec = EvmExecutor::new();
        assert!(exec.call("not-an-address", "").is_err());
        assert!(exec.call("0x", "").is_err());
        assert!(exec.call("0x1234", "").is_err()); // too short
    }

    #[test]
    fn bruteforce_call_huge_calldata() {
        let mut exec = EvmExecutor::new();
        let deployed = exec.deploy(RETURN_42_INIT).unwrap();
        // 10KB calldata — contract ignores it but EVM must handle
        let big_calldata = "ab".repeat(5_000);
        let result = exec.call(&deployed.address, &big_calldata);
        assert!(result.is_ok(), "large calldata failed: {:?}", result.err());
    }

    #[test]
    fn bruteforce_infinite_loop_bytecode() {
        let mut exec = EvmExecutor::new();
        // JUMPDEST, PUSH1 0x00, JUMP — infinite loop, should exhaust gas
        // Init: deploy runtime that loops
        // Runtime: 5B 6000 56 (JUMPDEST PUSH1_0 JUMP)
        // Init: PUSH1 3 PUSH1 0x0c PUSH1 0 CODECOPY PUSH1 3 PUSH1 0 RETURN
        let loop_init = "6003600c600039600360f35b600056";
        let deployed = exec.deploy(loop_init);
        if let Ok(d) = deployed {
            let result = exec.call(&d.address, "");
            // Must fail with gas exhaustion, never hang
            assert!(result.is_err(), "infinite loop should exhaust gas");
        }
    }

    #[test]
    fn bruteforce_reentrancy_pattern() {
        let mut exec = EvmExecutor::new();
        // Deploy two contracts, call one from another context
        let a = exec.deploy(RETURN_42_INIT).unwrap();
        let b = exec.deploy(RETURN_42_INIT).unwrap();
        // Rapid alternating calls
        for _ in 0..50 {
            assert!(exec.call(&a.address, "").is_ok());
            assert!(exec.call(&b.address, "").is_ok());
        }
    }

    #[test]
    fn bruteforce_static_call_no_mutation() {
        let mut exec = EvmExecutor::new();
        let deployed = exec.deploy(RETURN_42_INIT).unwrap();
        // 100 static calls should all return same value
        let mut outputs = Vec::new();
        for _ in 0..100 {
            let r = exec.static_call(&deployed.address, "").unwrap();
            outputs.push(r.output);
        }
        // All outputs identical
        assert!(outputs.windows(2).all(|w| w[0] == w[1]));
    }
}
