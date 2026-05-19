//! Approved hash functions: SHA3-256 (FIPS 202) and HMAC-SHA3-256 (SP 800-185).

use hmac::{Hmac, Mac};
use sha3::Digest;

use crate::approved_mode::require_approved;
use crate::errors::CryptoError;
use crate::types::Hash256;

type HmacSha3_256 = Hmac<sha3::Sha3_256>;

/// Compute SHA3-256 hash. Requires approved mode.
pub fn sha3_256(data: &[u8]) -> Result<Hash256, CryptoError> {
    require_approved()?;
    Ok(sha3_256_raw(data))
}

/// Compute HMAC-SHA3-256 (SP 800-185 aligned).
///
/// Used for server-side blind indexing in vault recovery.
/// Does not require approved mode since it is a utility hash, not a signing operation.
pub fn hmac_sha3_256(secret: &[u8], data: &[u8]) -> Result<Hash256, CryptoError> {
    let mut mac =
        HmacSha3_256::new_from_slice(secret).map_err(|_| CryptoError::InvalidKeyLength)?;
    mac.update(data);
    let result = mac.finalize().into_bytes();
    Ok(Hash256(result.into()))
}

/// Internal SHA3-256 without approved-mode check (for self-tests).
pub(crate) fn sha3_256_raw(data: &[u8]) -> Hash256 {
    let digest = sha3::Sha3_256::digest(data);
    Hash256(digest.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha3_256_known_answer() {
        let h = sha3_256_raw(b"");
        assert_eq!(
            h.to_hex(),
            "a7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a"
        );
    }

    #[test]
    fn sha3_256_deterministic() {
        let a = sha3_256_raw(b"test");
        let b = sha3_256_raw(b"test");
        assert_eq!(a, b);
    }

    #[test]
    fn hmac_sha3_256_deterministic() {
        let a = hmac_sha3_256(b"secret", b"data").unwrap();
        let b = hmac_sha3_256(b"secret", b"data").unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn hmac_sha3_256_different_keys_differ() {
        let a = hmac_sha3_256(b"key1", b"data").unwrap();
        let b = hmac_sha3_256(b"key2", b"data").unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn hmac_sha3_256_different_data_differ() {
        let a = hmac_sha3_256(b"secret", b"data1").unwrap();
        let b = hmac_sha3_256(b"secret", b"data2").unwrap();
        assert_ne!(a, b);
    }
}
