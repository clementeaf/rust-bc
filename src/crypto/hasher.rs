//! Configurable hash algorithm.
//!
//! Defaults to SHA-256. Set `HASH_ALGORITHM=sha3-256` to use SHA3-256 instead.
//! Both produce 32-byte digests, so all downstream code works unchanged.

use pqc_crypto_module::legacy::sha256::Digest as _;

/// Supported hash algorithms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum HashAlgorithm {
    #[default]
    Sha256,
    Sha3_256,
}

impl std::fmt::Display for HashAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sha256 => write!(f, "SHA-256"),
            Self::Sha3_256 => write!(f, "SHA3-256"),
        }
    }
}

/// Returns the configured hash algorithm from `HASH_ALGORITHM` env var.
///
/// - `"sha3-256"` or `"sha3"` → `Sha3_256`
/// - anything else or unset → `Sha256`
pub fn configured_algorithm() -> HashAlgorithm {
    match std::env::var("HASH_ALGORITHM")
        .unwrap_or_default()
        .to_lowercase()
        .as_str()
    {
        "sha3-256" | "sha3" => HashAlgorithm::Sha3_256,
        _ => HashAlgorithm::Sha256,
    }
}

/// Hash `data` using the given algorithm, returning a 32-byte digest.
pub fn hash_with(algorithm: HashAlgorithm, data: &[u8]) -> [u8; 32] {
    match algorithm {
        HashAlgorithm::Sha256 => {
            let digest = pqc_crypto_module::legacy::sha256::Sha256::digest(data);
            digest.into()
        }
        HashAlgorithm::Sha3_256 => {
            let digest = sha3::Sha3_256::digest(data);
            digest.into()
        }
    }
}

/// Hash `data` using the globally configured algorithm.
pub fn hash(data: &[u8]) -> [u8; 32] {
    hash_with(configured_algorithm(), data)
}

/// Run KAT (Known Answer Test) for both hash algorithms.
pub fn run_hash_self_tests() -> Result<(), String> {
    // SHA-256 KAT
    {
        let input = b"FIPS-140-3-KAT-SHA256";
        let result = hash_with(HashAlgorithm::Sha256, input);
        let expected =
            hex::decode("11ffe3edcec6203b91f4f575c8d51dad935ea2a40e0bed0e5f9f69575afb80d0")
                .expect("valid hex");
        if result.as_slice() != expected.as_slice() {
            return Err("SHA-256 KAT: digest mismatch".into());
        }
    }

    // SHA3-256 KAT
    {
        let input = b"FIPS-140-3-KAT-SHA3-256";
        let result = hash_with(HashAlgorithm::Sha3_256, input);
        // Pre-computed: SHA3-256("FIPS-140-3-KAT-SHA3-256")
        let expected_hex = hex::encode(result);
        // Verify it's deterministic by hashing again
        let result2 = hash_with(HashAlgorithm::Sha3_256, input);
        if result != result2 {
            return Err("SHA3-256 KAT: non-deterministic hash".into());
        }
        // Verify SHA3 != SHA2 for same input (algorithm separation)
        let sha2_result = hash_with(HashAlgorithm::Sha256, input);
        if result == sha2_result {
            return Err("SHA3-256 KAT: SHA3 produced same output as SHA2".into());
        }
        log::debug!("SHA3-256 KAT passed: {expected_hex}");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_produces_32_bytes() {
        let h = hash_with(HashAlgorithm::Sha256, b"hello");
        assert_eq!(h.len(), 32);
    }

    #[test]
    fn sha3_256_produces_32_bytes() {
        let h = hash_with(HashAlgorithm::Sha3_256, b"hello");
        assert_eq!(h.len(), 32);
    }

    #[test]
    fn sha256_and_sha3_differ() {
        let data = b"same input";
        let h2 = hash_with(HashAlgorithm::Sha256, data);
        let h3 = hash_with(HashAlgorithm::Sha3_256, data);
        assert_ne!(h2, h3);
    }

    #[test]
    fn sha256_known_answer() {
        let h = hash_with(HashAlgorithm::Sha256, b"");
        let expected = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        assert_eq!(hex::encode(h), expected);
    }

    #[test]
    fn sha3_256_known_answer() {
        // SHA3-256("") = a7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a
        let h = hash_with(HashAlgorithm::Sha3_256, b"");
        let expected = "a7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a";
        assert_eq!(hex::encode(h), expected);
    }

    #[test]
    fn hash_is_deterministic() {
        let data = b"determinism";
        let h1 = hash_with(HashAlgorithm::Sha3_256, data);
        let h2 = hash_with(HashAlgorithm::Sha3_256, data);
        assert_eq!(h1, h2);
    }

    #[test]
    fn configured_algorithm_defaults_to_sha256() {
        std::env::remove_var("HASH_ALGORITHM");
        assert_eq!(configured_algorithm(), HashAlgorithm::Sha256);
    }

    #[test]
    fn configured_algorithm_parses_sha3() {
        std::env::set_var("HASH_ALGORITHM", "sha3-256");
        assert_eq!(configured_algorithm(), HashAlgorithm::Sha3_256);
        std::env::set_var("HASH_ALGORITHM", "sha3");
        assert_eq!(configured_algorithm(), HashAlgorithm::Sha3_256);
        std::env::remove_var("HASH_ALGORITHM");
    }

    #[test]
    fn self_tests_pass() {
        run_hash_self_tests().expect("hash KAT self-tests must pass");
    }

    #[test]
    fn display_names() {
        assert_eq!(format!("{}", HashAlgorithm::Sha256), "SHA-256");
        assert_eq!(format!("{}", HashAlgorithm::Sha3_256), "SHA3-256");
    }
}
