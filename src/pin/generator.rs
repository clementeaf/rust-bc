//! Cryptographically secure numeric PIN generator with Argon2 hashing.

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand::Rng;
use thiserror::Error;

/// Minimum allowed PIN length.
pub const MIN_PIN_LENGTH: u8 = 4;

/// Maximum allowed PIN length.
pub const MAX_PIN_LENGTH: u8 = 6;

#[derive(Debug, Error)]
pub enum PinError {
    #[error("PIN length must be between {MIN_PIN_LENGTH} and {MAX_PIN_LENGTH}, got {0}")]
    InvalidLength(u8),

    #[error("hashing failed: {0}")]
    HashError(String),

    #[error("verification failed")]
    VerifyFailed,
}

/// A validated numeric PIN.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pin {
    value: String,
}

impl Pin {
    /// Generate a cryptographically secure numeric PIN of the given length.
    ///
    /// Length must be between [`MIN_PIN_LENGTH`] and [`MAX_PIN_LENGTH`] inclusive.
    pub fn generate(length: u8) -> Result<Self, PinError> {
        if !(MIN_PIN_LENGTH..=MAX_PIN_LENGTH).contains(&length) {
            return Err(PinError::InvalidLength(length));
        }

        let mut rng = rand::thread_rng();
        let upper_bound = 10u32.pow(u32::from(length));
        let raw: u32 = rng.gen_range(0..upper_bound);

        Ok(Self {
            value: format!("{:0>width$}", raw, width = usize::from(length)),
        })
    }

    /// Hash the PIN using Argon2id. Returns a PHC-format string safe for storage.
    pub fn hash(&self) -> Result<String, PinError> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        argon2
            .hash_password(self.value.as_bytes(), &salt)
            .map(|h| h.to_string())
            .map_err(|e| PinError::HashError(e.to_string()))
    }

    /// Verify a plaintext PIN against a stored Argon2 hash.
    pub fn verify(plain: &str, hash: &str) -> Result<(), PinError> {
        let parsed = PasswordHash::new(hash).map_err(|e| PinError::HashError(e.to_string()))?;
        Argon2::default()
            .verify_password(plain.as_bytes(), &parsed)
            .map_err(|_| PinError::VerifyFailed)
    }

    /// Returns the PIN as a string slice.
    pub fn as_str(&self) -> &str {
        &self.value
    }

    /// Returns the number of digits.
    pub fn len(&self) -> usize {
        self.value.len()
    }

    /// A PIN is never empty (minimum 4 digits), but clippy requires this.
    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_pin_of_requested_length() {
        for len in MIN_PIN_LENGTH..=MAX_PIN_LENGTH {
            let pin = Pin::generate(len).unwrap();
            assert_eq!(pin.len(), usize::from(len));
        }
    }

    #[test]
    fn pin_contains_only_digits() {
        for _ in 0..100 {
            let pin = Pin::generate(6).unwrap();
            assert!(pin.as_str().chars().all(|c| c.is_ascii_digit()));
        }
    }

    #[test]
    fn rejects_length_below_minimum() {
        let err = Pin::generate(3).unwrap_err();
        assert!(matches!(err, PinError::InvalidLength(3)));
    }

    #[test]
    fn rejects_length_above_maximum() {
        let err = Pin::generate(7).unwrap_err();
        assert!(matches!(err, PinError::InvalidLength(7)));
    }

    #[test]
    fn preserves_leading_zeros() {
        // Generate many 4-digit PINs; at least some should start with '0'
        // if the generator correctly zero-pads.
        let has_leading_zero = (0..1000)
            .map(|_| Pin::generate(4).unwrap())
            .any(|p| p.as_str().starts_with('0'));
        assert!(
            has_leading_zero,
            "no leading-zero PIN found in 1000 attempts"
        );
    }

    #[test]
    fn pins_are_not_all_identical() {
        let pins: Vec<String> = (0..10)
            .map(|_| Pin::generate(6).unwrap().as_str().to_string())
            .collect();
        let first = &pins[0];
        assert!(
            pins.iter().any(|p| p != first),
            "all 10 PINs were identical — RNG broken"
        );
    }

    #[test]
    fn hash_produces_phc_string() {
        let pin = Pin::generate(4).unwrap();
        let hash = pin.hash().unwrap();
        assert!(
            hash.starts_with("$argon2"),
            "expected PHC format, got: {hash}"
        );
    }

    #[test]
    fn verify_correct_pin_succeeds() {
        let pin = Pin::generate(6).unwrap();
        let hash = pin.hash().unwrap();
        assert!(Pin::verify(pin.as_str(), &hash).is_ok());
    }

    #[test]
    fn verify_wrong_pin_fails() {
        let pin = Pin::generate(4).unwrap();
        let hash = pin.hash().unwrap();
        let err = Pin::verify("0000", &hash);
        // Might pass if pin happens to be "0000", but overwhelmingly unlikely
        // We test the mechanism, not the specific value
        if pin.as_str() != "0000" {
            assert!(matches!(err, Err(PinError::VerifyFailed)));
        }
    }

    #[test]
    fn same_pin_produces_different_hashes() {
        let pin = Pin::generate(4).unwrap();
        let h1 = pin.hash().unwrap();
        let h2 = pin.hash().unwrap();
        assert_ne!(
            h1, h2,
            "same PIN should produce different hashes (unique salt)"
        );
    }
}
