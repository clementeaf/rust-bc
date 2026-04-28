//! Approved hash function: SHA3-256 (FIPS 202).

use sha3::Digest;

use crate::approved_mode::require_approved;
use crate::errors::CryptoError;
use crate::types::Hash256;

/// Compute SHA3-256 hash. Requires approved mode.
pub fn sha3_256(data: &[u8]) -> Result<Hash256, CryptoError> {
    require_approved()?;
    Ok(sha3_256_raw(data))
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
}
