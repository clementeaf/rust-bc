use std::sync::Arc;

use wasmtime::{Caller, Config, Engine, Linker, Module, Store, StoreLimits, StoreLimitsBuilder};

use crate::chaincode::ChaincodeError;
use crate::events::{BlockEvent, EventBus};
use crate::storage::world_state::WorldState;

/// Compiles and holds a Wasm chaincode module ready for execution.
///
/// Each invocation creates a fresh [`Store`] so that fuel and memory limits
/// are enforced independently per call.
pub struct WasmExecutor {
    pub(crate) engine: Engine,
    pub(crate) module: Module,
    pub(crate) fuel_limit: u64,
    pub(crate) memory_limit: Option<usize>,
    pub(crate) event_bus: Option<EventBus>,
    pub(crate) chaincode_id: String,
    pub(crate) channel_id: String,
}

/// Host data injected into the `Store` for every invocation.
struct HostState {
    world_state: Arc<dyn WorldState>,
    limits: StoreLimits,
    event_bus: Option<EventBus>,
    chaincode_id: String,
    channel_id: String,
}

impl WasmExecutor {
    /// Compile `wasm_bytes` and prepare the executor with a CPU fuel cap.
    ///
    /// Returns `Err(ChaincodeError::Execution(_))` if the bytes are not valid
    /// Wasm or if the engine cannot be configured.
    pub fn new(wasm_bytes: &[u8], fuel_limit: u64) -> Result<Self, ChaincodeError> {
        let mut config = Config::new();
        config.consume_fuel(true);

        let engine = Engine::new(&config)
            .map_err(|e| ChaincodeError::Execution(e.to_string()))?;

        let module = Module::new(&engine, wasm_bytes)
            .map_err(|e| ChaincodeError::Execution(e.to_string()))?;

        Ok(Self { engine, module, fuel_limit, memory_limit: None, event_bus: None, chaincode_id: String::new(), channel_id: String::new() })
    }

    /// Attach an [`EventBus`] so chaincode can emit [`BlockEvent::ChaincodeEvent`]s
    /// via the `set_event` host function.
    ///
    /// `chaincode_id` is embedded in every event emitted during execution.
    pub fn with_event_bus(mut self, bus: EventBus, chaincode_id: impl Into<String>) -> Self {
        self.event_bus = Some(bus);
        self.chaincode_id = chaincode_id.into();
        self
    }

    /// Set the maximum Wasm linear memory this executor will allow (in bytes).
    ///
    /// If a module tries to instantiate or grow memory beyond this limit,
    /// the operation fails — instantiation returns an error, and `memory.grow`
    /// returns -1 at runtime.
    pub fn with_memory_limit(mut self, max_bytes: usize) -> Self {
        self.memory_limit = Some(max_bytes);
        self
    }

    /// Invoke `func_name` in the Wasm module against `state`.
    ///
    /// ## Function ABI
    ///
    /// The exported Wasm function must have signature `() -> i64`.
    /// The `i64` return encodes the output slice in the module's `memory` export:
    /// ```text
    /// high 32 bits = byte offset (ptr)
    /// low  32 bits = byte length (len)
    /// ```
    /// The host reads `memory[ptr..ptr+len]` and returns it.
    ///
    /// ## Host imports (module `"env"`)
    ///
    /// ```text
    /// put_state(key_ptr: i32, key_len: i32, val_ptr: i32, val_len: i32) -> i32
    ///   Writes val into world state under key.
    ///   Returns 0 on success, -1 on error.
    ///
    /// get_state(key_ptr: i32, key_len: i32, out_ptr: i32, out_cap: i32) -> i32
    ///   Reads key from world state, copies up to out_cap bytes into memory[out_ptr].
    ///   Returns the number of bytes written, or -1 if the key is absent.
    /// ```
    pub fn invoke(
        &self,
        state: Arc<dyn WorldState>,
        func_name: &str,
    ) -> Result<Vec<u8>, ChaincodeError> {
        let limits = match self.memory_limit {
            Some(max) => StoreLimitsBuilder::new().memory_size(max).build(),
            None => StoreLimitsBuilder::new().build(),
        };

        let mut store = Store::new(
            &self.engine,
            HostState {
                world_state: state,
                limits,
                event_bus: self.event_bus.clone(),
                chaincode_id: self.chaincode_id.clone(),
                channel_id: self.channel_id.clone(),
            },
        );

        store.limiter(|s| &mut s.limits);

        store
            .set_fuel(self.fuel_limit)
            .map_err(|e| ChaincodeError::Execution(e.to_string()))?;

        let mut linker = Linker::<HostState>::new(&self.engine);

        // ── put_state ────────────────────────────────────────────────────────
        linker
            .func_wrap(
                "env",
                "put_state",
                |mut caller: Caller<'_, HostState>,
                 key_ptr: i32,
                 key_len: i32,
                 val_ptr: i32,
                 val_len: i32|
                 -> i32 {
                    let mem = match caller
                        .get_export("memory")
                        .and_then(|e| e.into_memory())
                    {
                        Some(m) => m,
                        None => return -1,
                    };

                    // Copy key and val before releasing the immutable borrow.
                    let (key, val) = {
                        let data = mem.data(&caller);
                        let key = match read_str(data, key_ptr, key_len) {
                            Some(k) => k.to_string(),
                            None => return -1,
                        };
                        let val = match read_bytes(data, val_ptr, val_len) {
                            Some(v) => v.to_vec(),
                            None => return -1,
                        };
                        (key, val)
                    };

                    match caller.data().world_state.put(&key, &val) {
                        Ok(_) => 0,
                        Err(_) => -1,
                    }
                },
            )
            .map_err(|e| ChaincodeError::Execution(e.to_string()))?;

        // ── get_state ────────────────────────────────────────────────────────
        linker
            .func_wrap(
                "env",
                "get_state",
                |mut caller: Caller<'_, HostState>,
                 key_ptr: i32,
                 key_len: i32,
                 out_ptr: i32,
                 out_cap: i32|
                 -> i32 {
                    let mem = match caller
                        .get_export("memory")
                        .and_then(|e| e.into_memory())
                    {
                        Some(m) => m,
                        None => return -1,
                    };

                    // Copy key before releasing the immutable borrow.
                    let key = {
                        let data = mem.data(&caller);
                        match read_str(data, key_ptr, key_len) {
                            Some(k) => k.to_string(),
                            None => return -1,
                        }
                    };

                    let value = match caller.data().world_state.get(&key) {
                        Ok(Some(v)) => v.data,
                        Ok(None) => return -1,
                        Err(_) => return -1,
                    };

                    let n = value.len().min(out_cap as usize);
                    let out = out_ptr as usize;
                    mem.data_mut(&mut caller)[out..out + n].copy_from_slice(&value[..n]);
                    n as i32
                },
            )
            .map_err(|e| ChaincodeError::Execution(e.to_string()))?;

        // ── set_event ────────────────────────────────────────────────────────
        linker
            .func_wrap(
                "env",
                "set_event",
                |mut caller: Caller<'_, HostState>,
                 name_ptr: i32,
                 name_len: i32,
                 payload_ptr: i32,
                 payload_len: i32|
                 -> i32 {
                    let mem = match caller
                        .get_export("memory")
                        .and_then(|e| e.into_memory())
                    {
                        Some(m) => m,
                        None => return -1,
                    };

                    let (event_name, payload) = {
                        let data = mem.data(&caller);
                        let name = match read_str(data, name_ptr, name_len) {
                            Some(n) => n.to_string(),
                            None => return -1,
                        };
                        let payload = match read_bytes(data, payload_ptr, payload_len) {
                            Some(p) => p.to_vec(),
                            None => return -1,
                        };
                        (name, payload)
                    };

                    if let Some(bus) = &caller.data().event_bus {
                        let chaincode_id = caller.data().chaincode_id.clone();
                        let channel_id = caller.data().channel_id.clone();
                        bus.publish(BlockEvent::ChaincodeEvent { channel_id, chaincode_id, event_name, payload });
                    }
                    0
                },
            )
            .map_err(|e| ChaincodeError::Execution(e.to_string()))?;

        // ── instantiate and call ─────────────────────────────────────────────
        let instance = linker
            .instantiate(&mut store, &self.module)
            .map_err(|e| ChaincodeError::Execution(e.to_string()))?;

        let func = instance
            .get_typed_func::<(), i64>(&mut store, func_name)
            .map_err(|e| ChaincodeError::Execution(e.to_string()))?;

        let ret = func
            .call(&mut store, ())
            .map_err(|e| ChaincodeError::Execution(e.to_string()))?;

        let ptr = (ret >> 32) as usize;
        let len = (ret & 0xFFFF_FFFF) as usize;

        let mem = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| ChaincodeError::Execution("no memory export".to_string()))?;

        Ok(mem.data(&store)[ptr..ptr + len].to_vec())
    }
}

// ── Private helpers ────────────────────────────────────────────────────────────

/// Borrow `data[ptr..ptr+len]` as a UTF-8 `&str`, or `None` on out-of-bounds
/// or invalid UTF-8.
fn read_str(data: &[u8], ptr: i32, len: i32) -> Option<&str> {
    let start = ptr as usize;
    let end = start.checked_add(len as usize)?;
    std::str::from_utf8(data.get(start..end)?).ok()
}

/// Borrow `data[ptr..ptr+len]` as `&[u8]`, or `None` on out-of-bounds.
fn read_bytes(data: &[u8], ptr: i32, len: i32) -> Option<&[u8]> {
    let start = ptr as usize;
    let end = start.checked_add(len as usize)?;
    data.get(start..end)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::world_state::MemoryWorldState;

    /// Minimal valid Wasm: `(module)` in binary format.
    const EMPTY_MODULE: &[u8] = &[
        0x00, 0x61, 0x73, 0x6d, // magic
        0x01, 0x00, 0x00, 0x00, // version
    ];

    /// WAT chaincode that calls put_state("x","1") then get_state("x") and
    /// returns the result via the (ptr<<32|len) ABI.
    ///
    /// Memory layout:
    ///   offset 0 : "x"  (key, 1 byte)
    ///   offset 4 : "1"  (value to put, 1 byte)
    ///   offset 8 : output area for get_state (64 bytes capacity)
    const CHAINCODE_WAT: &[u8] = br#"
(module
  (import "env" "put_state" (func $put_state (param i32 i32 i32 i32) (result i32)))
  (import "env" "get_state" (func $get_state (param i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 0) "x")
  (data (i32.const 4) "1")
  (func (export "run") (result i64)
    (local $n i32)
    ;; put_state(key=0,klen=1, val=4,vlen=1)
    (drop (call $put_state (i32.const 0) (i32.const 1) (i32.const 4) (i32.const 1)))
    ;; get_state(key=0,klen=1, out=8,cap=64) -> n
    (local.set $n (call $get_state (i32.const 0) (i32.const 1) (i32.const 8) (i32.const 64)))
    ;; return (8 << 32) | n
    (i64.or
      (i64.shl (i64.const 8) (i64.const 32))
      (i64.extend_i32_u (local.get $n))
    )
  )
)
"#;

    /// WAT module that starts with 2 pages (128 KB) of memory and has no
    /// host imports — used to test the memory limit at instantiation time.
    const TWO_PAGE_MODULE_WAT: &[u8] = br#"
(module
  (memory (export "memory") 2)
  (func (export "run") (result i64)
    (i64.const 0)
  )
)
"#;

    /// WAT chaincode that calls set_event("Transfer", [1,2,3]) and returns 0.
    ///
    /// Memory layout:
    ///   offset 0  : "Transfer" (event name, 8 bytes)
    ///   offset 16 : 0x01 0x02 0x03 (payload, 3 bytes)
    const SET_EVENT_WAT: &[u8] = br#"
(module
  (import "env" "set_event" (func $set_event (param i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 0)  "Transfer")
  (data (i32.const 16) "\01\02\03")
  (func (export "run") (result i64)
    (drop (call $set_event (i32.const 0) (i32.const 8) (i32.const 16) (i32.const 3)))
    (i64.const 0)
  )
)
"#;

    fn make_state() -> Arc<dyn WorldState> {
        Arc::new(MemoryWorldState::new())
    }

    // ── constructor tests ─────────────────────────────────────────────────────

    #[test]
    fn new_with_valid_wasm_succeeds() {
        assert!(WasmExecutor::new(EMPTY_MODULE, 1_000_000).is_ok());
    }

    #[test]
    fn new_with_valid_wasm_stores_fuel_limit() {
        let ex = WasmExecutor::new(EMPTY_MODULE, 42_000).unwrap();
        assert_eq!(ex.fuel_limit, 42_000);
    }

    #[test]
    fn new_with_invalid_wasm_returns_execution_error() {
        let result = WasmExecutor::new(b"this is not wasm", 1_000_000);
        match result {
            Err(ChaincodeError::Execution(_)) => {}
            Err(other) => panic!("expected Execution variant, got {other:?}"),
            Ok(_) => panic!("expected error, got Ok"),
        }
    }

    // ── host function tests ───────────────────────────────────────────────────

    #[test]
    fn invoke_put_then_get_returns_value() {
        let ex = WasmExecutor::new(CHAINCODE_WAT, 10_000_000).unwrap();
        let result = ex.invoke(make_state(), "run").unwrap();
        assert_eq!(result, b"1");
    }

    #[test]
    fn invoke_writes_to_world_state() {
        let state = Arc::new(MemoryWorldState::new());
        let ex = WasmExecutor::new(CHAINCODE_WAT, 10_000_000).unwrap();
        ex.invoke(Arc::clone(&state) as Arc<dyn WorldState>, "run").unwrap();
        let stored = state.get("x").unwrap().unwrap();
        assert_eq!(stored.data, b"1");
    }

    // ── memory limit tests ────────────────────────────────────────────────────

    #[test]
    fn invoke_exceeding_memory_limit_returns_error() {
        // TWO_PAGE_MODULE requests 2 pages (131_072 bytes); limit is 1 page.
        let ex = WasmExecutor::new(TWO_PAGE_MODULE_WAT, 10_000_000)
            .unwrap()
            .with_memory_limit(65_536); // 1 page = 64 KB

        let result = ex.invoke(make_state(), "run");
        match result {
            Err(ChaincodeError::Execution(_)) => {}
            Err(other) => panic!("expected Execution variant, got {other:?}"),
            Ok(_) => panic!("expected memory-limit error, got Ok"),
        }
    }

    #[test]
    fn invoke_within_memory_limit_succeeds() {
        // TWO_PAGE_MODULE requests 2 pages; limit is 3 pages → should pass.
        let ex = WasmExecutor::new(TWO_PAGE_MODULE_WAT, 10_000_000)
            .unwrap()
            .with_memory_limit(3 * 65_536);

        assert!(ex.invoke(make_state(), "run").is_ok());
    }

    // ── set_event tests ───────────────────────────────────────────────────────

    #[test]
    fn set_event_emits_chaincode_event_to_bus() {
        use crate::events::{BlockEvent, EventBus};

        let bus = EventBus::new();
        let mut rx = bus.subscribe();

        let ex = WasmExecutor::new(SET_EVENT_WAT, 10_000_000)
            .unwrap()
            .with_event_bus(bus, "mycc");

        ex.invoke(make_state(), "run").unwrap();

        let event = rx.try_recv().expect("event should be in channel");
        assert_eq!(
            event,
            BlockEvent::ChaincodeEvent {
                channel_id: "".to_string(),
                chaincode_id: "mycc".to_string(),
                event_name: "Transfer".to_string(),
                payload: vec![1, 2, 3],
            }
        );
    }

    #[test]
    fn set_event_without_bus_is_noop_and_succeeds() {
        // No event_bus attached — set_event should still return 0 without panicking.
        let ex = WasmExecutor::new(SET_EVENT_WAT, 10_000_000).unwrap();
        assert!(ex.invoke(make_state(), "run").is_ok());
    }
}
