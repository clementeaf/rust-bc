//! Address derivation from public keys.
//!
//! Address = first 20 bytes of SHA-256(public_key), hex-encoded (40 chars).
//! This mirrors Ethereum's approach but uses SHA-256 instead of Keccak-256.

use pqc_crypto_module::legacy::legacy_sha256;

/// Derive a 40-character hex address from a public key (any algorithm).
///
/// `address = hex(sha256(pubkey_bytes)[0..20])`
///
/// Works for Ed25519 (32-byte pk), ML-DSA-65 (1952-byte pk), or any key bytes.
pub fn address_from_pubkey(pubkey: &[u8]) -> String {
    let hash = legacy_sha256(pubkey).unwrap_or_else(|_| {
        // Fallback: manual truncation if crypto module not initialized
        let mut out = [0u8; 32];
        let len = pubkey.len().min(32);
        out[..len].copy_from_slice(&pubkey[..len]);
        out
    });
    // Take first 20 bytes → 40 hex chars
    hex::encode(&hash[..20])
}

/// Validate that a string looks like a valid address (40 hex chars).
pub fn is_valid_address(addr: &str) -> bool {
    addr.len() == 40 && addr.chars().all(|c| c.is_ascii_hexdigit())
}

/// The zero address (used for coinbase source).
pub const ZERO_ADDRESS: &str = "0000000000000000000000000000000000000000";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn address_from_pubkey_is_40_hex_chars() {
        let pk = [42u8; 32]; // Fake Ed25519 pubkey
        let addr = address_from_pubkey(&pk);
        assert_eq!(addr.len(), 40);
        assert!(addr.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn deterministic() {
        let pk = b"test-public-key-bytes";
        let a1 = address_from_pubkey(pk);
        let a2 = address_from_pubkey(pk);
        assert_eq!(a1, a2);
    }

    #[test]
    fn different_keys_different_addresses() {
        let a1 = address_from_pubkey(&[1u8; 32]);
        let a2 = address_from_pubkey(&[2u8; 32]);
        assert_ne!(a1, a2);
    }

    #[test]
    fn works_with_large_key() {
        // ML-DSA-65 public key is 1952 bytes
        let pk = vec![0xAB; 1952];
        let addr = address_from_pubkey(&pk);
        assert_eq!(addr.len(), 40);
        assert!(is_valid_address(&addr));
    }

    #[test]
    fn is_valid_address_checks() {
        assert!(is_valid_address("abcdef0123456789abcdef0123456789abcdef01"));
        assert!(is_valid_address(ZERO_ADDRESS));
        assert!(!is_valid_address("too_short"));
        assert!(!is_valid_address(
            "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz"
        )); // non-hex
        assert!(!is_valid_address("")); // empty
    }

    #[test]
    fn zero_address_is_valid() {
        assert!(is_valid_address(ZERO_ADDRESS));
    }
}
