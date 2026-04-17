//! Precompile interface — maps Ethereum-style precompile addresses to
//! rust-bc native operations (hashing, signature verification, etc.).
//!
//! Precompiles are called by address (0x01-0x09 in Ethereum convention).
//! Each precompile takes raw input bytes and returns output bytes.

use sha2::{Digest, Sha256};

/// Precompile address range.
pub const PRECOMPILE_START: u8 = 0x01;
pub const PRECOMPILE_END: u8 = 0x09;

/// Result of executing a precompile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrecompileResult {
    pub output: Vec<u8>,
    pub gas_used: u64,
}

/// Precompile execution errors.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PrecompileError {
    #[error("unknown precompile address: 0x{0:02x}")]
    UnknownAddress(u8),
    #[error("invalid input for precompile 0x{addr:02x}: {reason}")]
    InvalidInput { addr: u8, reason: String },
}

/// Execute a precompile by address.
pub fn execute(address: u8, input: &[u8]) -> Result<PrecompileResult, PrecompileError> {
    match address {
        0x01 => precompile_ecrecover(input),
        0x02 => precompile_sha256(input),
        0x03 => precompile_ripemd160(input),
        0x04 => precompile_identity(input),
        0x05 => precompile_modexp(input),
        0x20 => precompile_sha256_bc(input), // rust-bc extension
        _ => Err(PrecompileError::UnknownAddress(address)),
    }
}

/// Check if an address is a precompile.
pub fn is_precompile(address: u8) -> bool {
    matches!(address, 0x01..=0x05 | 0x20)
}

// --- Precompile implementations ---

/// 0x01: ecrecover — stub (returns empty, real impl needs secp256k1).
fn precompile_ecrecover(input: &[u8]) -> Result<PrecompileResult, PrecompileError> {
    if input.len() < 128 {
        return Err(PrecompileError::InvalidInput {
            addr: 0x01,
            reason: format!("need 128 bytes, got {}", input.len()),
        });
    }
    // Stub: ecrecover not implemented (rust-bc uses Ed25519/ML-DSA-65).
    Ok(PrecompileResult {
        output: vec![0u8; 32],
        gas_used: 3000,
    })
}

/// 0x02: SHA-256 hash.
fn precompile_sha256(input: &[u8]) -> Result<PrecompileResult, PrecompileError> {
    let hash = Sha256::digest(input);
    Ok(PrecompileResult {
        output: hash.to_vec(),
        gas_used: 60 + (input.len() as u64 / 32) * 12,
    })
}

/// 0x03: RIPEMD-160 — stub (returns SHA-256 truncated to 20 bytes).
fn precompile_ripemd160(input: &[u8]) -> Result<PrecompileResult, PrecompileError> {
    let hash = Sha256::digest(input);
    let mut output = vec![0u8; 32];
    output[12..32].copy_from_slice(&hash[..20]);
    Ok(PrecompileResult {
        output,
        gas_used: 600 + (input.len() as u64 / 32) * 120,
    })
}

/// 0x04: Identity — returns input unchanged.
fn precompile_identity(input: &[u8]) -> Result<PrecompileResult, PrecompileError> {
    Ok(PrecompileResult {
        output: input.to_vec(),
        gas_used: 15 + (input.len() as u64 / 32) * 3,
    })
}

/// 0x05: Modular exponentiation — stub (returns zeros).
fn precompile_modexp(input: &[u8]) -> Result<PrecompileResult, PrecompileError> {
    // Full modexp is complex; stub returns 32 zero bytes.
    let _ = input;
    Ok(PrecompileResult {
        output: vec![0u8; 32],
        gas_used: 200,
    })
}

/// 0x20: rust-bc SHA-256 extension (same as 0x02 but at non-Ethereum address).
fn precompile_sha256_bc(input: &[u8]) -> Result<PrecompileResult, PrecompileError> {
    precompile_sha256(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_precompile() {
        let result = execute(0x02, b"hello").unwrap();
        let expected = Sha256::digest(b"hello");
        assert_eq!(result.output, expected.to_vec());
        assert!(result.gas_used > 0);
    }

    #[test]
    fn sha256_empty_input() {
        let result = execute(0x02, b"").unwrap();
        assert_eq!(result.output.len(), 32);
    }

    #[test]
    fn identity_precompile() {
        let result = execute(0x04, b"passthrough").unwrap();
        assert_eq!(result.output, b"passthrough");
    }

    #[test]
    fn identity_empty() {
        let result = execute(0x04, b"").unwrap();
        assert!(result.output.is_empty());
    }

    #[test]
    fn ecrecover_stub_needs_128_bytes() {
        let err = execute(0x01, b"too short").unwrap_err();
        assert!(matches!(
            err,
            PrecompileError::InvalidInput { addr: 0x01, .. }
        ));
    }

    #[test]
    fn ecrecover_stub_128_bytes_ok() {
        let result = execute(0x01, &[0u8; 128]).unwrap();
        assert_eq!(result.output.len(), 32);
    }

    #[test]
    fn ripemd160_stub() {
        let result = execute(0x03, b"test").unwrap();
        assert_eq!(result.output.len(), 32);
        // First 12 bytes are zero (EVM convention).
        assert!(result.output[..12].iter().all(|&b| b == 0));
    }

    #[test]
    fn modexp_stub() {
        let result = execute(0x05, b"").unwrap();
        assert_eq!(result.output, vec![0u8; 32]);
    }

    #[test]
    fn bc_sha256_extension() {
        let standard = execute(0x02, b"data").unwrap();
        let extension = execute(0x20, b"data").unwrap();
        assert_eq!(standard.output, extension.output);
    }

    #[test]
    fn unknown_precompile() {
        let err = execute(0xFF, b"").unwrap_err();
        assert!(matches!(err, PrecompileError::UnknownAddress(0xFF)));
    }

    #[test]
    fn is_precompile_check() {
        assert!(is_precompile(0x01));
        assert!(is_precompile(0x04));
        assert!(is_precompile(0x20));
        assert!(!is_precompile(0x06));
        assert!(!is_precompile(0xFF));
    }

    #[test]
    fn gas_scales_with_input() {
        let small = execute(0x02, &[0u8; 32]).unwrap();
        let large = execute(0x02, &[0u8; 1024]).unwrap();
        assert!(large.gas_used > small.gas_used);
    }
}
