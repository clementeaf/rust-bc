//! # Chaincode SDK
//!
//! Write smart contracts in Rust that compile to WebAssembly and run on the
//! rust-bc blockchain.
//!
//! ## Quick start
//!
//! ```rust,ignore
//! use chaincode_sdk::*;
//!
//! #[no_mangle]
//! pub extern "C" fn create_asset() -> i32 {
//!     let key = "asset1";
//!     let value = r#"{"owner":"alice","value":100}"#;
//!     put_state(key, value.as_bytes());
//!     0
//! }
//!
//! #[no_mangle]
//! pub extern "C" fn read_asset() -> i32 {
//!     match get_state("asset1") {
//!         Some(data) => {
//!             set_response(&data);
//!             0
//!         }
//!         None => -1,
//!     }
//! }
//! ```
//!
//! ## Build
//!
//! ```bash
//! cargo build --target wasm32-unknown-unknown --release
//! ```
//!
//! ## Deploy
//!
//! ```bash
//! curl -X POST https://localhost:8080/api/v1/chaincode/install \
//!   -F "chaincode_id=mycc" \
//!   -F "version=1.0" \
//!   -F "wasm=@target/wasm32-unknown-unknown/release/my_chaincode.wasm"
//! ```

// ── Host function imports (provided by the blockchain runtime) ──────────────

extern "C" {
    /// Write a key-value pair to the world state.
    /// Returns 0 on success, -1 on error.
    fn put_state(key_ptr: *const u8, key_len: i32, val_ptr: *const u8, val_len: i32) -> i32;

    /// Read a value from the world state.
    /// Writes the value to `out_ptr` (up to `out_cap` bytes).
    /// Returns the number of bytes written, or -1 if the key does not exist.
    fn get_state(key_ptr: *const u8, key_len: i32, out_ptr: *mut u8, out_cap: i32) -> i32;

    /// Emit a chaincode event.
    /// Returns 0 on success.
    fn set_event(
        name_ptr: *const u8,
        name_len: i32,
        payload_ptr: *const u8,
        payload_len: i32,
    ) -> i32;

    /// Override the endorsement policy for a specific state key.
    /// `policy_json` is a JSON-serialized endorsement policy.
    /// Returns 0 on success.
    fn set_key_endorsement_policy(
        key_ptr: *const u8,
        key_len: i32,
        policy_ptr: *const u8,
        policy_len: i32,
    ) -> i32;

    /// Get the version history for a key.
    /// Writes JSON-serialized history entries to `out_ptr`.
    /// Returns bytes written, or -1 on error.
    fn get_history_for_key(
        key_ptr: *const u8,
        key_len: i32,
        out_ptr: *mut u8,
        out_cap: i32,
    ) -> i32;

    /// Invoke another chaincode.
    /// Writes the result to `out_ptr`.
    /// Returns bytes written, or -1 on error.
    fn invoke_chaincode(
        cc_id_ptr: *const u8,
        cc_id_len: i32,
        func_ptr: *const u8,
        func_len: i32,
        out_ptr: *mut u8,
        out_cap: i32,
    ) -> i32;
}

// ── Public API (what chaincode developers use) ──────────────────────────────

/// Maximum buffer size for reading state values and history.
const MAX_BUFFER: usize = 64 * 1024; // 64 KB

/// Write a key-value pair to the world state.
///
/// ```rust,ignore
/// use chaincode_sdk::state_put;
/// state_put("asset:1", b"hello");
/// ```
pub fn state_put(key: &str, value: &[u8]) -> Result<(), ChaincodeError> {
    let result =
        unsafe { put_state(key.as_ptr(), key.len() as i32, value.as_ptr(), value.len() as i32) };
    if result == 0 {
        Ok(())
    } else {
        Err(ChaincodeError::StatePutFailed(key.to_string()))
    }
}

/// Read a value from the world state.
///
/// Returns `None` if the key does not exist.
///
/// ```rust,ignore
/// use chaincode_sdk::state_get;
/// if let Some(data) = state_get("asset:1") {
///     // use data
/// }
/// ```
pub fn state_get(key: &str) -> Option<Vec<u8>> {
    let mut buf = vec![0u8; MAX_BUFFER];
    let n = unsafe {
        get_state(
            key.as_ptr(),
            key.len() as i32,
            buf.as_mut_ptr(),
            buf.len() as i32,
        )
    };
    if n < 0 {
        None
    } else {
        buf.truncate(n as usize);
        Some(buf)
    }
}

/// Read a value from the world state and deserialize it as JSON.
///
/// ```rust,ignore
/// use chaincode_sdk::state_get_json;
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct Asset { owner: String, value: u64 }
///
/// let asset: Asset = state_get_json("asset:1").unwrap();
/// ```
pub fn state_get_json<T: serde::de::DeserializeOwned>(key: &str) -> Option<T> {
    let bytes = state_get(key)?;
    serde_json::from_slice(&bytes).ok()
}

/// Write a value to the world state as JSON.
///
/// ```rust,ignore
/// use chaincode_sdk::state_put_json;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct Asset { owner: String, value: u64 }
///
/// let asset = Asset { owner: "alice".into(), value: 100 };
/// state_put_json("asset:1", &asset).unwrap();
/// ```
pub fn state_put_json<T: serde::Serialize>(key: &str, value: &T) -> Result<(), ChaincodeError> {
    let json = serde_json::to_vec(value).map_err(|e| ChaincodeError::SerializationFailed(e.to_string()))?;
    state_put(key, &json)
}

/// Emit a chaincode event.
///
/// Events are delivered to clients subscribed via WebSocket.
///
/// ```rust,ignore
/// use chaincode_sdk::emit_event;
/// emit_event("transfer", b"from:alice,to:bob,amount:100");
/// ```
pub fn emit_event(name: &str, payload: &[u8]) -> Result<(), ChaincodeError> {
    let result = unsafe {
        set_event(
            name.as_ptr(),
            name.len() as i32,
            payload.as_ptr(),
            payload.len() as i32,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(ChaincodeError::EventFailed(name.to_string()))
    }
}

/// Override the endorsement policy for a specific state key.
///
/// ```rust,ignore
/// use chaincode_sdk::set_key_policy;
/// set_key_policy("sensitive:key", r#"{"AllOf":["org1","org2"]}"#).unwrap();
/// ```
pub fn set_key_policy(key: &str, policy_json: &str) -> Result<(), ChaincodeError> {
    let result = unsafe {
        set_key_endorsement_policy(
            key.as_ptr(),
            key.len() as i32,
            policy_json.as_ptr(),
            policy_json.len() as i32,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(ChaincodeError::PolicyFailed(key.to_string()))
    }
}

/// Get the version history of a key.
///
/// Returns a JSON string containing the history entries.
///
/// ```rust,ignore
/// use chaincode_sdk::history_for_key;
/// let history = history_for_key("asset:1").unwrap();
/// ```
pub fn history_for_key(key: &str) -> Option<Vec<u8>> {
    let mut buf = vec![0u8; MAX_BUFFER];
    let n = unsafe {
        get_history_for_key(
            key.as_ptr(),
            key.len() as i32,
            buf.as_mut_ptr(),
            buf.len() as i32,
        )
    };
    if n < 0 {
        None
    } else {
        buf.truncate(n as usize);
        Some(buf)
    }
}

/// Invoke another chaincode by ID and function name.
///
/// Returns the raw response bytes, or `None` on failure.
///
/// ```rust,ignore
/// use chaincode_sdk::invoke;
/// let result = invoke("other_cc", "queryBalance").unwrap();
/// ```
pub fn invoke(chaincode_id: &str, function: &str) -> Option<Vec<u8>> {
    let mut buf = vec![0u8; MAX_BUFFER];
    let n = unsafe {
        invoke_chaincode(
            chaincode_id.as_ptr(),
            chaincode_id.len() as i32,
            function.as_ptr(),
            function.len() as i32,
            buf.as_mut_ptr(),
            buf.len() as i32,
        )
    };
    if n < 0 {
        None
    } else {
        buf.truncate(n as usize);
        Some(buf)
    }
}

// ── Response helper ─────────────────────────────────────────────────────────

use core::sync::atomic::{AtomicUsize, Ordering};

/// Response buffer and metadata. Wasm is single-threaded so a global is safe.
static mut RESPONSE_BUF: [u8; MAX_BUFFER] = [0u8; MAX_BUFFER];
static RESPONSE_LEN: AtomicUsize = AtomicUsize::new(0);

/// Set the response data that will be returned to the caller.
///
/// The runtime reads this buffer after the chaincode function returns.
pub fn set_response(data: &[u8]) {
    let len = data.len().min(MAX_BUFFER);
    // SAFETY: Wasm execution is single-threaded; no concurrent access.
    unsafe {
        RESPONSE_BUF[..len].copy_from_slice(&data[..len]);
    }
    RESPONSE_LEN.store(len, Ordering::Relaxed);
}

/// Called by the runtime to read the response buffer.
#[no_mangle]
pub extern "C" fn __chaincode_response_ptr() -> *const u8 {
    // SAFETY: Wasm execution is single-threaded; returns raw pointer without creating a reference.
    core::ptr::addr_of!(RESPONSE_BUF) as *const u8
}

/// Called by the runtime to read the response buffer length.
#[no_mangle]
pub extern "C" fn __chaincode_response_len() -> i32 {
    RESPONSE_LEN.load(Ordering::Relaxed) as i32
}

// ── Error types ─────────────────────────────────────────────────────────────

/// Errors returned by chaincode SDK operations.
#[derive(Debug)]
pub enum ChaincodeError {
    StatePutFailed(String),
    SerializationFailed(String),
    EventFailed(String),
    PolicyFailed(String),
}

impl core::fmt::Display for ChaincodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::StatePutFailed(k) => write!(f, "failed to put state key '{k}'"),
            Self::SerializationFailed(e) => write!(f, "serialization failed: {e}"),
            Self::EventFailed(n) => write!(f, "failed to emit event '{n}'"),
            Self::PolicyFailed(k) => write!(f, "failed to set policy for key '{k}'"),
        }
    }
}
