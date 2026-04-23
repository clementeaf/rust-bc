//! EVM stress tests and property-based fuzzing.
//!
//! Gap 1: Concurrent load (multi-threaded access to shared executor)
//! Gap 2: Randomized fuzzing (proptest on bytecode, calldata, addresses)

use proptest::prelude::*;
use rust_bc::evm_compat::executor::EvmExecutor;
use std::sync::{Arc, Mutex};
use std::thread;

// Known-good contract: returns 0x42
const RETURN_42: &str = "600a600c600039600a6000f3604260005260206000f3";

// ── Gap 1: Stress / Load tests ──────────────────────────────────────────────

#[test]
fn stress_concurrent_deploys_10_threads() {
    let exec = Arc::new(Mutex::new(EvmExecutor::new()));
    let mut handles = vec![];

    for _ in 0..10 {
        let exec = Arc::clone(&exec);
        handles.push(thread::spawn(move || {
            let mut e = exec.lock().unwrap();
            let result = e.deploy(RETURN_42);
            assert!(result.is_ok(), "deploy failed: {:?}", result.err());
        }));
    }

    for h in handles {
        h.join().expect("thread panicked");
    }

    let e = exec.lock().unwrap();
    assert_eq!(e.list_contracts().len(), 10);
}

#[test]
fn stress_concurrent_calls_10_threads_50_each() {
    let exec = Arc::new(Mutex::new(EvmExecutor::new()));

    // Deploy one contract first
    let addr = {
        let mut e = exec.lock().unwrap();
        e.deploy(RETURN_42).unwrap().address
    };

    let mut handles = vec![];
    for _ in 0..10 {
        let exec = Arc::clone(&exec);
        let addr = addr.clone();
        handles.push(thread::spawn(move || {
            for i in 0..50 {
                let mut e = exec.lock().unwrap();
                let result = e.call(&addr, "");
                assert!(result.is_ok(), "call {i} failed: {:?}", result.err());
            }
        }));
    }

    for h in handles {
        h.join().expect("thread panicked");
    }
}

#[test]
fn stress_mixed_deploy_and_call_concurrent() {
    let exec = Arc::new(Mutex::new(EvmExecutor::new()));

    // Deploy initial contract
    let addr = {
        let mut e = exec.lock().unwrap();
        e.deploy(RETURN_42).unwrap().address
    };

    let mut handles = vec![];

    // 5 threads deploying
    for _ in 0..5 {
        let exec = Arc::clone(&exec);
        handles.push(thread::spawn(move || {
            for _ in 0..10 {
                let mut e = exec.lock().unwrap();
                let _ = e.deploy(RETURN_42);
            }
        }));
    }

    // 5 threads calling
    for _ in 0..5 {
        let exec = Arc::clone(&exec);
        let addr = addr.clone();
        handles.push(thread::spawn(move || {
            for _ in 0..20 {
                let mut e = exec.lock().unwrap();
                let _ = e.call(&addr, "");
            }
        }));
    }

    for h in handles {
        h.join().expect("thread panicked");
    }

    let e = exec.lock().unwrap();
    // 1 initial + 50 from threads
    assert_eq!(e.list_contracts().len(), 51);
}

#[test]
fn stress_sequential_500_operations() {
    let mut exec = EvmExecutor::new();
    let mut addrs = vec![];

    // 100 deploys
    for _ in 0..100 {
        let d = exec.deploy(RETURN_42).unwrap();
        addrs.push(d.address);
    }
    assert_eq!(exec.list_contracts().len(), 100);

    // 400 calls spread across contracts
    for i in 0..400 {
        let addr = &addrs[i % addrs.len()];
        let result = exec.call(addr, "");
        assert!(result.is_ok(), "call {i} failed: {:?}", result.err());
    }
}

// ── Gap 2: Property-based fuzzing ───────────────────────────────────────────

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    /// Random bytecode never panics the executor.
    #[test]
    fn fuzz_evm_deploy_random_bytecode(data in prop::collection::vec(any::<u8>(), 0..2048)) {
        let mut exec = EvmExecutor::new();
        let hex_str = hex::encode(&data);
        let _ = exec.deploy(&hex_str);
        // Must not panic — any result (Ok or Err) is acceptable
    }

    /// Random calldata never panics on a valid contract.
    #[test]
    fn fuzz_evm_call_random_calldata(data in prop::collection::vec(any::<u8>(), 0..4096)) {
        let mut exec = EvmExecutor::new();
        let deployed = exec.deploy(RETURN_42).unwrap();
        let hex_str = hex::encode(&data);
        let _ = exec.call(&deployed.address, &hex_str);
        // Must not panic
    }

    /// Random address strings never panic.
    #[test]
    fn fuzz_evm_call_random_address(addr_bytes in prop::collection::vec(any::<u8>(), 20..=20)) {
        let mut exec = EvmExecutor::new();
        let addr = format!("0x{}", hex::encode(&addr_bytes));
        let _ = exec.call(&addr, "");
        // Must not panic
    }

    /// Garbage strings as address never panic.
    #[test]
    fn fuzz_evm_call_garbage_address(s in "\\PC{0,100}") {
        let mut exec = EvmExecutor::new();
        let _ = exec.call(&s, "");
        // Must not panic
    }

    /// Random bytecode deploy + immediate call never panics.
    #[test]
    fn fuzz_evm_deploy_then_call(
        bytecode in prop::collection::vec(any::<u8>(), 1..512),
        calldata in prop::collection::vec(any::<u8>(), 0..256),
    ) {
        let mut exec = EvmExecutor::new();
        let bc_hex = hex::encode(&bytecode);
        if let Ok(deployed) = exec.deploy(&bc_hex) {
            let cd_hex = hex::encode(&calldata);
            let _ = exec.call(&deployed.address, &cd_hex);
        }
        // Must not panic regardless of input
    }

    /// Static call with random calldata never mutates state.
    #[test]
    fn fuzz_evm_static_call_no_panic(data in prop::collection::vec(any::<u8>(), 0..1024)) {
        let mut exec = EvmExecutor::new();
        let deployed = exec.deploy(RETURN_42).unwrap();
        let hex_str = hex::encode(&data);
        let _ = exec.static_call(&deployed.address, &hex_str);
        // Must not panic
    }
}
