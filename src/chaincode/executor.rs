use std::sync::Arc;

use wasmtime::{Caller, Config, Engine, Linker, Module, Store, StoreLimits, StoreLimitsBuilder};

use crate::acl::provider::AclProvider;
use crate::chaincode::resolver::ChaincodeResolver;
use crate::chaincode::ChaincodeError;
use crate::endorsement::key_policy::KeyEndorsementStore;
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
    pub(crate) key_endorsement_store: Option<Arc<dyn KeyEndorsementStore>>,
    pub(crate) chaincode_resolver: Option<Arc<dyn ChaincodeResolver>>,
    pub(crate) acl_provider: Option<Arc<dyn AclProvider>>,
    pub(crate) invocation_depth: u32,
}

/// Maximum nesting depth for chaincode-to-chaincode invocations.
pub const MAX_CHAINCODE_DEPTH: u32 = 8;

/// Host data injected into the `Store` for every invocation.
struct HostState {
    world_state: Arc<dyn WorldState>,
    limits: StoreLimits,
    event_bus: Option<EventBus>,
    chaincode_id: String,
    channel_id: String,
    key_endorsement_store: Option<Arc<dyn KeyEndorsementStore>>,
    chaincode_resolver: Option<Arc<dyn ChaincodeResolver>>,
    acl_provider: Option<Arc<dyn AclProvider>>,
    invocation_depth: u32,
    fuel_limit: u64,
}

impl WasmExecutor {
    /// Compile `wasm_bytes` and prepare the executor with a CPU fuel cap.
    ///
    /// Returns `Err(ChaincodeError::Execution(_))` if the bytes are not valid
    /// Wasm or if the engine cannot be configured.
    pub fn new(wasm_bytes: &[u8], fuel_limit: u64) -> Result<Self, ChaincodeError> {
        let mut config = Config::new();
        config.consume_fuel(true);

        let engine = Engine::new(&config).map_err(|e| ChaincodeError::Execution(e.to_string()))?;

        let module = Module::new(&engine, wasm_bytes)
            .map_err(|e| ChaincodeError::Execution(e.to_string()))?;

        Ok(Self {
            engine,
            module,
            fuel_limit,
            memory_limit: None,
            event_bus: None,
            chaincode_id: String::new(),
            channel_id: String::new(),
            key_endorsement_store: None,
            chaincode_resolver: None,
            acl_provider: None,
            invocation_depth: 0,
        })
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

    /// Attach a [`KeyEndorsementStore`] so chaincode can call
    /// `set_key_endorsement_policy(key, policy_json)` to override the
    /// endorsement policy for individual state keys.
    pub fn with_key_endorsement_store(mut self, store: Arc<dyn KeyEndorsementStore>) -> Self {
        self.key_endorsement_store = Some(store);
        self
    }

    /// Set the maximum Wasm linear memory this executor will allow (in bytes).
    ///
    /// If a module tries to instantiate or grow memory beyond this limit,
    /// the operation fails — instantiation returns an error, and `memory.grow`
    /// returns -1 at runtime.
    /// Attach a [`ChaincodeResolver`] so this executor can invoke other
    /// chaincodes via the `invoke_chaincode` host function.
    pub fn with_chaincode_resolver(mut self, resolver: Arc<dyn ChaincodeResolver>) -> Self {
        self.chaincode_resolver = Some(resolver);
        self
    }

    /// Attach an [`AclProvider`] for cross-chaincode invocation ACL checks.
    pub fn with_acl_provider(mut self, acl: Arc<dyn AclProvider>) -> Self {
        self.acl_provider = Some(acl);
        self
    }

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
                key_endorsement_store: self.key_endorsement_store.clone(),
                chaincode_resolver: self.chaincode_resolver.clone(),
                acl_provider: self.acl_provider.clone(),
                invocation_depth: self.invocation_depth,
                fuel_limit: self.fuel_limit,
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
                    let mem = match caller.get_export("memory").and_then(|e| e.into_memory()) {
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
                    let mem = match caller.get_export("memory").and_then(|e| e.into_memory()) {
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
                    let mem = match caller.get_export("memory").and_then(|e| e.into_memory()) {
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
                        bus.publish(BlockEvent::ChaincodeEvent {
                            channel_id,
                            chaincode_id,
                            event_name,
                            payload,
                        });
                    }
                    0
                },
            )
            .map_err(|e| ChaincodeError::Execution(e.to_string()))?;

        // ── set_key_endorsement_policy ────────────────────────────────────────
        linker
            .func_wrap(
                "env",
                "set_key_endorsement_policy",
                |mut caller: Caller<'_, HostState>,
                 key_ptr: i32,
                 key_len: i32,
                 policy_ptr: i32,
                 policy_len: i32|
                 -> i32 {
                    let mem = match caller.get_export("memory").and_then(|e| e.into_memory()) {
                        Some(m) => m,
                        None => return -1,
                    };

                    let (key, policy_json) = {
                        let data = mem.data(&caller);
                        let key = match read_str(data, key_ptr, key_len) {
                            Some(k) => k.to_string(),
                            None => return -1,
                        };
                        let json = match read_str(data, policy_ptr, policy_len) {
                            Some(j) => j.to_string(),
                            None => return -1,
                        };
                        (key, json)
                    };

                    let store = match &caller.data().key_endorsement_store {
                        Some(s) => Arc::clone(s),
                        None => return -1,
                    };

                    let policy = match serde_json::from_str(&policy_json) {
                        Ok(p) => p,
                        Err(_) => return -1,
                    };

                    match store.set_key_policy(&key, &policy) {
                        Ok(_) => 0,
                        Err(_) => -1,
                    }
                },
            )
            .map_err(|e| ChaincodeError::Execution(e.to_string()))?;

        // ── get_history_for_key ──────────────────────────────────────────────
        //
        // ABI: (key_ptr, key_len, out_ptr, out_cap) -> i32
        // Writes JSON-serialized `Vec<HistoryEntry>` to guest memory.
        // Returns bytes written, or -1 on error.
        linker
            .func_wrap(
                "env",
                "get_history_for_key",
                |mut caller: Caller<'_, HostState>,
                 key_ptr: i32,
                 key_len: i32,
                 out_ptr: i32,
                 out_cap: i32|
                 -> i32 {
                    let mem = match caller.get_export("memory").and_then(|e| e.into_memory()) {
                        Some(m) => m,
                        None => return -1,
                    };

                    let key = {
                        let data = mem.data(&caller);
                        match read_str(data, key_ptr, key_len) {
                            Some(k) => k.to_string(),
                            None => return -1,
                        }
                    };

                    let entries = match caller.data().world_state.get_history(&key) {
                        Ok(e) => e,
                        Err(_) => return -1,
                    };

                    let json = match serde_json::to_vec(&entries) {
                        Ok(j) => j,
                        Err(_) => return -1,
                    };

                    let n = json.len().min(out_cap as usize);
                    let out = out_ptr as usize;
                    mem.data_mut(&mut caller)[out..out + n].copy_from_slice(&json[..n]);
                    n as i32
                },
            )
            .map_err(|e| ChaincodeError::Execution(e.to_string()))?;

        // ── invoke_chaincode ────────────────────────────────────────────────
        //
        // ABI: (cc_id_ptr, cc_id_len, func_ptr, func_len, out_ptr, out_cap) -> i32
        // Resolves the target chaincode, creates a temporary WasmExecutor,
        // invokes `func`, copies the result to guest memory.
        // Returns bytes written, or -1 on error.
        linker
            .func_wrap(
                "env",
                "invoke_chaincode",
                |mut caller: Caller<'_, HostState>,
                 cc_id_ptr: i32,
                 cc_id_len: i32,
                 func_ptr: i32,
                 func_len: i32,
                 out_ptr: i32,
                 out_cap: i32|
                 -> i32 {
                    let mem = match caller.get_export("memory").and_then(|e| e.into_memory()) {
                        Some(m) => m,
                        None => return -1,
                    };

                    let (cc_id, func_name) = {
                        let data = mem.data(&caller);
                        let cc = match read_str(data, cc_id_ptr, cc_id_len) {
                            Some(s) => s.to_string(),
                            None => return -1,
                        };
                        let f = match read_str(data, func_ptr, func_len) {
                            Some(s) => s.to_string(),
                            None => return -1,
                        };
                        (cc, f)
                    };

                    let host = caller.data();
                    let depth = host.invocation_depth;
                    if depth >= MAX_CHAINCODE_DEPTH {
                        return -1;
                    }

                    // ACL check: if an AclProvider is set, verify ChaincodeInvoke permission.
                    if let Some(acl) = &host.acl_provider {
                        let resource = format!("chaincode/{}/invoke", cc_id);
                        match acl.get_acl(&resource) {
                            Ok(None) => return -1, // No ACL entry → denied
                            Err(_) => return -1,
                            Ok(Some(_)) => {} // ACL entry exists → allowed
                        }
                    }

                    let resolver = match &host.chaincode_resolver {
                        Some(r) => Arc::clone(r),
                        None => return -1,
                    };

                    let wasm_bytes = match resolver.resolve(&cc_id) {
                        Ok(b) => b,
                        Err(_) => return -1,
                    };

                    let fuel = host.fuel_limit;
                    let world_state = Arc::clone(&host.world_state);
                    let cc_resolver = Some(Arc::clone(&resolver));

                    let child = match WasmExecutor::new(&wasm_bytes, fuel) {
                        Ok(mut ex) => {
                            ex.chaincode_resolver = cc_resolver;
                            ex.invocation_depth = depth + 1;
                            ex
                        }
                        Err(_) => return -1,
                    };

                    let result = match child.invoke(world_state, &func_name) {
                        Ok(r) => r,
                        Err(_) => return -1,
                    };

                    let n = result.len().min(out_cap as usize);
                    let out = out_ptr as usize;
                    mem.data_mut(&mut caller)[out..out + n].copy_from_slice(&result[..n]);
                    n as i32
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

    /// Execute chaincode in simulation mode — writes are buffered locally and
    /// the `base_state` is never modified.
    ///
    /// Returns `(result_bytes, ReadWriteSet)` so callers can build an endorsed
    /// transaction proposal without committing state changes.
    pub fn simulate(
        &self,
        state: Arc<dyn WorldState>,
        func_name: &str,
    ) -> Result<(Vec<u8>, crate::transaction::rwset::ReadWriteSet), ChaincodeError> {
        use crate::chaincode::simulation::SimulationWorldState;
        let sim = Arc::new(SimulationWorldState::new(state));
        let result = self.invoke(Arc::clone(&sim) as Arc<dyn WorldState>, func_name)?;
        let rwset = sim.to_rwset();
        Ok((result, rwset))
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
        ex.invoke(Arc::clone(&state) as Arc<dyn WorldState>, "run")
            .unwrap();
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

    // ── simulate tests ────────────────────────────────────────────────────────

    /// WAT chaincode that:
    ///   1. put_state("a", "1")   → produces a KVWrite for "a"
    ///   2. get_state("b", ...)   → produces a KVRead  for "b" (absent → returns -1)
    ///
    /// Memory layout:
    ///   offset 0 : "a"  (1 byte)
    ///   offset 4 : "1"  (1 byte)
    ///   offset 8 : "b"  (1 byte)
    ///   offset 16: output buffer (64 bytes)
    const SIMULATE_WAT: &[u8] = br#"
(module
  (import "env" "put_state" (func $put_state (param i32 i32 i32 i32) (result i32)))
  (import "env" "get_state" (func $get_state (param i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 0) "a")
  (data (i32.const 4) "1")
  (data (i32.const 8) "b")
  (func (export "run") (result i64)
    ;; put_state("a", "1")
    (drop (call $put_state (i32.const 0) (i32.const 1) (i32.const 4) (i32.const 1)))
    ;; get_state("b", out=16, cap=64) - key absent, returns -1
    (drop (call $get_state (i32.const 8) (i32.const 1) (i32.const 16) (i32.const 64)))
    ;; return empty result
    (i64.const 0)
  )
)
"#;

    /// WAT chaincode that calls set_key_endorsement_policy("asset:1", <policy_json>)
    /// and returns 0.
    ///
    /// Memory layout:
    ///   offset 0  : "asset:1"  (key, 7 bytes)
    ///   offset 16 : policy JSON (see data section below)
    const SET_KEY_ENDORSEMENT_WAT: &[u8] = br#"
(module
  (import "env" "set_key_endorsement_policy"
    (func $set_kep (param i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 0)  "asset:1")
  (data (i32.const 16) "{\"AnyOf\":[\"org1\"]}")
  (func (export "run") (result i64)
    ;; set_key_endorsement_policy(key=0,klen=7, policy=16,plen=18)
    (drop (call $set_kep (i32.const 0) (i32.const 7) (i32.const 16) (i32.const 18)))
    (i64.const 0)
  )
)
"#;

    /// Same WAT but with malformed JSON so the host function returns -1.
    const SET_KEY_ENDORSEMENT_BAD_JSON_WAT: &[u8] = br#"
(module
  (import "env" "set_key_endorsement_policy"
    (func $set_kep (param i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 0)  "asset:1")
  (data (i32.const 16) "not-valid-json!!!")
  (func (export "run") (result i64)
    (local $rc i32)
    (local.set $rc
      (call $set_kep (i32.const 0) (i32.const 7) (i32.const 16) (i32.const 17)))
    ;; store return code at offset 64 so we can read it back
    (i32.store8 (i32.const 64) (local.get $rc))
    ;; return (64<<32)|1
    (i64.or
      (i64.shl (i64.const 64) (i64.const 32))
      (i64.const 1)
    )
  )
)
"#;

    #[test]
    fn set_key_endorsement_policy_stores_policy() {
        use crate::endorsement::key_policy::MemoryKeyEndorsementStore;
        use crate::endorsement::policy::EndorsementPolicy;

        let kep_store = Arc::new(MemoryKeyEndorsementStore::new());
        let ex = WasmExecutor::new(SET_KEY_ENDORSEMENT_WAT, 10_000_000)
            .unwrap()
            .with_key_endorsement_store(Arc::clone(&kep_store) as Arc<dyn KeyEndorsementStore>);

        ex.invoke(make_state(), "run").unwrap();

        let policy = kep_store.get_key_policy("asset:1").unwrap();
        assert_eq!(
            policy,
            Some(EndorsementPolicy::AnyOf(vec!["org1".to_string()]))
        );
    }

    #[test]
    fn set_key_endorsement_policy_returns_minus_one_on_bad_json() {
        use crate::endorsement::key_policy::MemoryKeyEndorsementStore;

        let kep_store = Arc::new(MemoryKeyEndorsementStore::new());
        let ex = WasmExecutor::new(SET_KEY_ENDORSEMENT_BAD_JSON_WAT, 10_000_000)
            .unwrap()
            .with_key_endorsement_store(Arc::clone(&kep_store) as Arc<dyn KeyEndorsementStore>);

        // invoke succeeds (chaincode itself does not abort)
        let result = ex.invoke(make_state(), "run").unwrap();
        // the byte stored at offset 64 is the return code (cast to u8, -1i32 wraps to 0xFF)
        assert_eq!(
            result[0], 0xFF_u8,
            "expected -1 (0xFF) return code from host"
        );

        // no policy must have been stored
        assert!(kep_store.get_key_policy("asset:1").unwrap().is_none());
    }

    #[test]
    fn set_key_endorsement_policy_without_store_returns_minus_one() {
        // No store attached → host function must return -1 gracefully.
        // We reuse SET_KEY_ENDORSEMENT_BAD_JSON_WAT which captures the return code.
        // But we use the good JSON WAT and attach no store instead.
        const WAT: &[u8] = br#"
(module
  (import "env" "set_key_endorsement_policy"
    (func $set_kep (param i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 0)  "asset:1")
  (data (i32.const 16) "{\"AnyOf\":[\"org1\"]}")
  (func (export "run") (result i64)
    (local $rc i32)
    (local.set $rc
      (call $set_kep (i32.const 0) (i32.const 7) (i32.const 16) (i32.const 18)))
    (i32.store8 (i32.const 64) (local.get $rc))
    (i64.or
      (i64.shl (i64.const 64) (i64.const 32))
      (i64.const 1)
    )
  )
)
"#;
        let ex = WasmExecutor::new(WAT, 10_000_000).unwrap();
        // no with_key_endorsement_store call → store is None
        let result = ex.invoke(make_state(), "run").unwrap();
        assert_eq!(result[0], 0xFF_u8, "expected -1 when no store is attached");
    }

    #[test]
    fn simulate_produces_rwset_without_modifying_base_state() {
        let base = Arc::new(MemoryWorldState::new());
        let ex = WasmExecutor::new(SIMULATE_WAT, 10_000_000).unwrap();

        let (_result, rwset) = ex
            .simulate(Arc::clone(&base) as Arc<dyn WorldState>, "run")
            .unwrap();

        // base state must not contain "a"
        assert!(base.get("a").unwrap().is_none());

        // rwset must have write("a")
        let write_keys: Vec<&str> = rwset.writes.iter().map(|w| w.key.as_str()).collect();
        assert!(write_keys.contains(&"a"), "expected write for 'a'");

        // rwset must have read("b")
        let read_keys: Vec<&str> = rwset.reads.iter().map(|r| r.key.as_str()).collect();
        assert!(read_keys.contains(&"b"), "expected read for 'b'");
    }

    #[test]
    fn get_history_via_world_state_trait() {
        let base = Arc::new(MemoryWorldState::new());

        // Write 3 versions through the WorldState trait.
        base.put("color", b"red").unwrap();
        base.put("color", b"blue").unwrap();
        base.put("color", b"green").unwrap();

        let history = base.get_history("color").unwrap();
        assert_eq!(history.len(), 3);
        assert_eq!(history[0].version, 1);
        assert_eq!(history[0].data, b"red");
        assert_eq!(history[1].data, b"blue");
        assert_eq!(history[2].data, b"green");
        assert!(!history[2].is_delete);
    }

    #[test]
    fn invoke_chaincode_cross_call_shares_world_state() {
        use crate::chaincode::resolver::StoreBackedResolver;
        use crate::chaincode::{ChaincodePackageStore, MemoryChaincodePackageStore};

        // Chaincode B: puts "x" = "1" and returns empty.
        let cc_b_wat: &[u8] = br#"
(module
  (import "env" "put_state"
    (func $put (param i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 0) "x")
  (data (i32.const 8) "1")
  (func (export "run") (result i64)
    (drop (call $put (i32.const 0) (i32.const 1) (i32.const 8) (i32.const 1)))
    (i64.const 0)
  )
)
"#;

        // Chaincode A: invokes chaincode B "run", then gets "x" and returns it.
        let cc_a_wat: &[u8] = br#"
(module
  (import "env" "invoke_chaincode"
    (func $invoke (param i32 i32 i32 i32 i32 i32) (result i32)))
  (import "env" "get_state"
    (func $get (param i32 i32 i32 i32) (result i32)))
  (import "env" "put_state"
    (func $put (param i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  ;; "ccB" at offset 0
  (data (i32.const 0) "ccB")
  ;; "run" at offset 8
  (data (i32.const 8) "run")
  ;; "x" at offset 16
  (data (i32.const 16) "x")
  (func (export "run") (result i64)
    (local $n i32)
    ;; invoke_chaincode("ccB", "run", out=128, cap=64)
    (drop (call $invoke (i32.const 0) (i32.const 3) (i32.const 8) (i32.const 3) (i32.const 128) (i32.const 64)))
    ;; get_state("x", out=200, cap=64)
    (local.set $n (call $get (i32.const 16) (i32.const 1) (i32.const 200) (i32.const 64)))
    ;; return (200<<32)|n
    (i64.or
      (i64.shl (i64.const 200) (i64.const 32))
      (i64.extend_i32_u (local.get $n))
    )
  )
)
"#;

        let pkg_store = Arc::new(MemoryChaincodePackageStore::new());
        pkg_store.store_package("ccB", "latest", cc_b_wat).unwrap();

        let resolver = Arc::new(StoreBackedResolver::new(pkg_store));
        let state = Arc::new(MemoryWorldState::new());

        let ex = WasmExecutor::new(cc_a_wat, 10_000_000)
            .unwrap()
            .with_chaincode_resolver(resolver);

        let result = ex.invoke(state.clone(), "run").unwrap();
        assert_eq!(result, b"1", "chaincode A should see the value put by B");
    }

    #[test]
    fn invoke_chaincode_acl_allowed() {
        use crate::acl::provider::{AclProvider, MemoryAclProvider};
        use crate::chaincode::resolver::StoreBackedResolver;
        use crate::chaincode::{ChaincodePackageStore, MemoryChaincodePackageStore};

        // Chaincode B: puts "y" = "ok"
        let cc_b_wat: &[u8] = br#"
(module
  (import "env" "put_state" (func $put (param i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 0) "y")
  (data (i32.const 8) "ok")
  (func (export "run") (result i64)
    (drop (call $put (i32.const 0) (i32.const 1) (i32.const 8) (i32.const 2)))
    (i64.const 0)
  )
)
"#;

        // Chaincode A: invokes ccB
        let cc_a_wat: &[u8] = br#"
(module
  (import "env" "invoke_chaincode" (func $invoke (param i32 i32 i32 i32 i32 i32) (result i32)))
  (import "env" "get_state" (func $get (param i32 i32 i32 i32) (result i32)))
  (import "env" "put_state" (func $put (param i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 0) "ccB")
  (data (i32.const 8) "run")
  (data (i32.const 16) "y")
  (func (export "run") (result i64)
    (local $n i32)
    (drop (call $invoke (i32.const 0) (i32.const 3) (i32.const 8) (i32.const 3) (i32.const 128) (i32.const 64)))
    (local.set $n (call $get (i32.const 16) (i32.const 1) (i32.const 200) (i32.const 64)))
    (i64.or (i64.shl (i64.const 200) (i64.const 32)) (i64.extend_i32_u (local.get $n)))
  )
)
"#;

        let pkg = Arc::new(MemoryChaincodePackageStore::new());
        pkg.store_package("ccB", "latest", cc_b_wat).unwrap();
        let resolver = Arc::new(StoreBackedResolver::new(pkg));

        let acl = Arc::new(MemoryAclProvider::new());
        acl.set_acl("chaincode/ccB/invoke", "allow_all").unwrap();

        let state = Arc::new(MemoryWorldState::new());
        let ex = WasmExecutor::new(cc_a_wat, 10_000_000)
            .unwrap()
            .with_chaincode_resolver(resolver)
            .with_acl_provider(acl);

        let result = ex.invoke(state, "run").unwrap();
        assert_eq!(result, b"ok");
    }

    #[test]
    fn invoke_chaincode_acl_denied() {
        use crate::acl::provider::MemoryAclProvider;
        use crate::chaincode::resolver::StoreBackedResolver;
        use crate::chaincode::{ChaincodePackageStore, MemoryChaincodePackageStore};

        let cc_b_wat: &[u8] = br#"
(module
  (import "env" "put_state" (func $put (param i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (func (export "run") (result i64) (i64.const 0))
)
"#;

        // A tries to invoke ccB
        let cc_a_wat: &[u8] = br#"
(module
  (import "env" "invoke_chaincode" (func $invoke (param i32 i32 i32 i32 i32 i32) (result i32)))
  (import "env" "put_state" (func $put (param i32 i32 i32 i32) (result i32)))
  (import "env" "get_state" (func $get (param i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 0) "ccB")
  (data (i32.const 8) "run")
  (func (export "run") (result i64)
    (local $rc i32)
    ;; invoke_chaincode should return -1 (denied)
    (local.set $rc (call $invoke (i32.const 0) (i32.const 3) (i32.const 8) (i32.const 3) (i32.const 128) (i32.const 64)))
    ;; store rc at offset 200 and return it
    (i32.store8 (i32.const 200) (local.get $rc))
    (i64.or (i64.shl (i64.const 200) (i64.const 32)) (i64.const 1))
  )
)
"#;

        let pkg = Arc::new(MemoryChaincodePackageStore::new());
        pkg.store_package("ccB", "latest", cc_b_wat).unwrap();
        let resolver = Arc::new(StoreBackedResolver::new(pkg));

        // ACL provider with NO entry for ccB → denied
        let acl = Arc::new(MemoryAclProvider::new());

        let state = Arc::new(MemoryWorldState::new());
        let ex = WasmExecutor::new(cc_a_wat, 10_000_000)
            .unwrap()
            .with_chaincode_resolver(resolver)
            .with_acl_provider(acl);

        let result = ex.invoke(state, "run").unwrap();
        // -1 as i32 stored as u8 = 255
        assert_eq!(
            result,
            &[255u8],
            "invoke_chaincode should return -1 when ACL denies"
        );
    }

    #[test]
    fn invoke_chaincode_depth_limit() {
        use crate::chaincode::resolver::StoreBackedResolver;
        use crate::chaincode::{ChaincodePackageStore, MemoryChaincodePackageStore};

        // Self-recursive chaincode: invokes itself.
        let recursive_wat: &[u8] = br#"
(module
  (import "env" "invoke_chaincode" (func $invoke (param i32 i32 i32 i32 i32 i32) (result i32)))
  (import "env" "put_state" (func $put (param i32 i32 i32 i32) (result i32)))
  (import "env" "get_state" (func $get (param i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 0) "self")
  (data (i32.const 8) "run")
  (func (export "run") (result i64)
    (local $rc i32)
    (local.set $rc (call $invoke (i32.const 0) (i32.const 4) (i32.const 8) (i32.const 3) (i32.const 128) (i32.const 64)))
    ;; store rc at 200
    (i32.store8 (i32.const 200) (local.get $rc))
    (i64.or (i64.shl (i64.const 200) (i64.const 32)) (i64.const 1))
  )
)
"#;

        let pkg = Arc::new(MemoryChaincodePackageStore::new());
        pkg.store_package("self", "latest", recursive_wat).unwrap();
        let resolver = Arc::new(StoreBackedResolver::new(pkg));

        let state = Arc::new(MemoryWorldState::new());

        // Start at MAX_CHAINCODE_DEPTH → invoke_chaincode immediately returns -1
        let mut ex = WasmExecutor::new(recursive_wat, 100_000_000)
            .unwrap()
            .with_chaincode_resolver(resolver);
        ex.invocation_depth = MAX_CHAINCODE_DEPTH; // 8

        let result = ex.invoke(state, "run").unwrap();
        // invoke_chaincode returns -1, stored as u8 = 255
        assert_eq!(result, &[255u8], "should fail at depth limit");
    }
}
