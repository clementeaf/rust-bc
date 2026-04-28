//! Solidity ABI encoding/decoding — enables Ethereum tooling (ethers.js, web3.js)
//! to interact with rust-bc Wasm chaincode via familiar function signatures.
//!
//! Supports the core ABI types: uint256, address, bytes, string, bool.

use pqc_crypto_module::legacy::sha256::{Digest, Sha256};

/// ABI-encoded value types (subset of Solidity ABI).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AbiValue {
    Uint256([u8; 32]),
    Address([u8; 20]),
    Bool(bool),
    Bytes(Vec<u8>),
    String(String),
}

impl AbiValue {
    /// Create a Uint256 from a u64.
    pub fn from_u64(val: u64) -> Self {
        let mut buf = [0u8; 32];
        buf[24..32].copy_from_slice(&val.to_be_bytes());
        AbiValue::Uint256(buf)
    }

    /// Extract u64 from Uint256 (truncates upper bytes).
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            AbiValue::Uint256(buf) => {
                let mut bytes = [0u8; 8];
                bytes.copy_from_slice(&buf[24..32]);
                Some(u64::from_be_bytes(bytes))
            }
            _ => None,
        }
    }

    /// Create an Address from a 20-byte array.
    pub fn from_address(addr: [u8; 20]) -> Self {
        AbiValue::Address(addr)
    }

    /// Create an Address from a hex string (with or without 0x prefix).
    pub fn address_from_hex(hex: &str) -> Result<Self, AbiError> {
        let hex = hex.strip_prefix("0x").unwrap_or(hex);
        if hex.len() != 40 {
            return Err(AbiError::InvalidAddress(format!(
                "expected 40 hex chars, got {}",
                hex.len()
            )));
        }
        let bytes = hex::decode(hex).map_err(|e| AbiError::InvalidAddress(e.to_string()))?;
        let mut addr = [0u8; 20];
        addr.copy_from_slice(&bytes);
        Ok(AbiValue::Address(addr))
    }
}

/// Compute the 4-byte function selector: `keccak256(signature)[0..4]`.
///
/// Uses SHA-256 (not keccak) since rust-bc doesn't depend on keccak.
/// This is a rust-bc-specific convention; for true Ethereum compat,
/// swap to keccak256 when needed.
pub fn function_selector(signature: &str) -> [u8; 4] {
    let hash = Sha256::digest(signature.as_bytes());
    let mut selector = [0u8; 4];
    selector.copy_from_slice(&hash[..4]);
    selector
}

/// Encode a function call: `selector || encoded_args`.
pub fn encode_call(signature: &str, args: &[AbiValue]) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&function_selector(signature));

    // Head section: fixed-size values or offsets for dynamic types.
    let mut head = Vec::new();
    let mut tail = Vec::new();
    let head_size = args.len() * 32;

    for arg in args {
        match arg {
            AbiValue::Uint256(val) => {
                head.extend_from_slice(val);
            }
            AbiValue::Address(addr) => {
                let mut padded = [0u8; 32];
                padded[12..32].copy_from_slice(addr);
                head.extend_from_slice(&padded);
            }
            AbiValue::Bool(val) => {
                let mut padded = [0u8; 32];
                if *val {
                    padded[31] = 1;
                }
                head.extend_from_slice(&padded);
            }
            AbiValue::Bytes(_) | AbiValue::String(_) => {
                // Dynamic type: head contains offset, tail contains length + data.
                let offset = head_size + tail.len();
                let mut offset_bytes = [0u8; 32];
                offset_bytes[24..32].copy_from_slice(&(offset as u64).to_be_bytes());
                head.extend_from_slice(&offset_bytes);

                let raw = match arg {
                    AbiValue::String(s) => s.as_bytes().to_vec(),
                    AbiValue::Bytes(b) => b.clone(),
                    _ => unreachable!(),
                };

                // Length prefix.
                let mut len_bytes = [0u8; 32];
                len_bytes[24..32].copy_from_slice(&(raw.len() as u64).to_be_bytes());
                tail.extend_from_slice(&len_bytes);

                // Data padded to 32-byte boundary.
                tail.extend_from_slice(&raw);
                let padding = (32 - (raw.len() % 32)) % 32;
                tail.extend(std::iter::repeat_n(0u8, padding));
            }
        }
    }

    data.extend_from_slice(&head);
    data.extend_from_slice(&tail);
    data
}

/// Decode a u64 from the first 32 bytes of ABI-encoded return data.
pub fn decode_u64(data: &[u8]) -> Result<u64, AbiError> {
    if data.len() < 32 {
        return Err(AbiError::InsufficientData {
            expected: 32,
            got: data.len(),
        });
    }
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&data[24..32]);
    Ok(u64::from_be_bytes(bytes))
}

/// Decode a bool from 32 bytes of ABI-encoded data.
pub fn decode_bool(data: &[u8]) -> Result<bool, AbiError> {
    if data.len() < 32 {
        return Err(AbiError::InsufficientData {
            expected: 32,
            got: data.len(),
        });
    }
    Ok(data[31] != 0)
}

/// Derive an Ethereum-compatible address from a rust-bc DID.
///
/// `address = SHA-256(did)[12..32]` (take last 20 bytes).
pub fn did_to_address(did: &str) -> [u8; 20] {
    let hash = Sha256::digest(did.as_bytes());
    let mut addr = [0u8; 20];
    addr.copy_from_slice(&hash[12..32]);
    addr
}

/// ABI errors.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AbiError {
    #[error("insufficient data: expected {expected} bytes, got {got}")]
    InsufficientData { expected: usize, got: usize },
    #[error("invalid address: {0}")]
    InvalidAddress(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uint256_from_u64_roundtrip() {
        let val = AbiValue::from_u64(42);
        assert_eq!(val.as_u64(), Some(42));
    }

    #[test]
    fn uint256_from_u64_large() {
        let val = AbiValue::from_u64(u64::MAX);
        assert_eq!(val.as_u64(), Some(u64::MAX));
    }

    #[test]
    fn uint256_from_u64_zero() {
        let val = AbiValue::from_u64(0);
        assert_eq!(val.as_u64(), Some(0));
    }

    #[test]
    fn address_from_hex_valid() {
        let addr =
            AbiValue::address_from_hex("0x0000000000000000000000000000000000000001").unwrap();
        match addr {
            AbiValue::Address(bytes) => assert_eq!(bytes[19], 1),
            _ => panic!("expected Address"),
        }
    }

    #[test]
    fn address_from_hex_no_prefix() {
        let addr = AbiValue::address_from_hex("0000000000000000000000000000000000000002").unwrap();
        match addr {
            AbiValue::Address(bytes) => assert_eq!(bytes[19], 2),
            _ => panic!("expected Address"),
        }
    }

    #[test]
    fn address_from_hex_invalid_length() {
        let err = AbiValue::address_from_hex("0xABCD").unwrap_err();
        assert!(matches!(err, AbiError::InvalidAddress(_)));
    }

    #[test]
    fn function_selector_deterministic() {
        let s1 = function_selector("transfer(address,uint256)");
        let s2 = function_selector("transfer(address,uint256)");
        assert_eq!(s1, s2);
    }

    #[test]
    fn function_selector_differs_by_signature() {
        let s1 = function_selector("transfer(address,uint256)");
        let s2 = function_selector("approve(address,uint256)");
        assert_ne!(s1, s2);
    }

    #[test]
    fn function_selector_4_bytes() {
        let s = function_selector("balanceOf(address)");
        assert_eq!(s.len(), 4);
    }

    #[test]
    fn encode_call_uint256() {
        let data = encode_call("setValue(uint256)", &[AbiValue::from_u64(100)]);
        // 4 bytes selector + 32 bytes arg = 36 bytes.
        assert_eq!(data.len(), 36);
        // Decode the arg back.
        let decoded = decode_u64(&data[4..]).unwrap();
        assert_eq!(decoded, 100);
    }

    #[test]
    fn encode_call_bool() {
        let data = encode_call("setFlag(bool)", &[AbiValue::Bool(true)]);
        assert_eq!(data.len(), 36);
        assert!(decode_bool(&data[4..]).unwrap());
    }

    #[test]
    fn decode_u64_insufficient_data() {
        let err = decode_u64(&[0u8; 10]).unwrap_err();
        assert!(matches!(err, AbiError::InsufficientData { .. }));
    }

    #[test]
    fn did_to_address_deterministic() {
        let a1 = did_to_address("did:bc:alice");
        let a2 = did_to_address("did:bc:alice");
        assert_eq!(a1, a2);
    }

    #[test]
    fn did_to_address_differs() {
        let a1 = did_to_address("did:bc:alice");
        let a2 = did_to_address("did:bc:bob");
        assert_ne!(a1, a2);
    }

    #[test]
    fn did_to_address_20_bytes() {
        let addr = did_to_address("did:bc:test");
        assert_eq!(addr.len(), 20);
    }
}
