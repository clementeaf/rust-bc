//! HSM-backed signing provider using PKCS#11 (feature-gated under `hsm`).
//!
//! When the `hsm` feature is enabled, `HsmSigningProvider` delegates signing
//! to a hardware security module via the PKCS#11 interface (using `cryptoki`).
//!
//! Configuration via environment variables:
//! - `HSM_PKCS11_LIB` — path to the PKCS#11 shared library
//! - `HSM_SLOT_ID` — PKCS#11 slot identifier
//! - `HSM_PIN` — user PIN for the slot
//! - `HSM_KEY_LABEL` — label of the Ed25519 signing key object

use thiserror::Error;

#[derive(Debug, Error)]
pub enum HsmError {
    #[allow(dead_code)]
    #[error("PKCS#11 library not found: {0}")]
    LibraryNotFound(String),
    #[allow(dead_code)]
    #[error("slot not found: {0}")]
    SlotNotFound(u64),
    #[error("authentication failed")]
    AuthFailed,
    #[allow(dead_code)]
    #[error("key not found: {0}")]
    KeyNotFound(String),
    #[allow(dead_code)]
    #[error("signing operation failed: {0}")]
    SignFailed(String),
    #[error("HSM feature not enabled")]
    NotEnabled,
}

/// Configuration for connecting to an HSM via PKCS#11.
#[derive(Debug, Clone)]
pub struct HsmConfig {
    #[allow(dead_code)]
    pub pkcs11_lib: String,
    #[allow(dead_code)]
    pub slot_id: u64,
    #[allow(dead_code)]
    pub pin: String,
    #[allow(dead_code)]
    pub key_label: String,
}

impl HsmConfig {
    #[allow(dead_code)]
    /// Load configuration from environment variables.
    pub fn from_env() -> Result<Self, HsmError> {
        Ok(Self {
            pkcs11_lib: std::env::var("HSM_PKCS11_LIB")
                .map_err(|_| HsmError::LibraryNotFound("HSM_PKCS11_LIB not set".into()))?,
            slot_id: std::env::var("HSM_SLOT_ID")
                .unwrap_or_else(|_| "0".into())
                .parse()
                .unwrap_or(0),
            pin: std::env::var("HSM_PIN").map_err(|_| HsmError::AuthFailed)?,
            key_label: std::env::var("HSM_KEY_LABEL").unwrap_or_else(|_| "ed25519-key".into()),
        })
    }
}

/// HSM-backed signing provider.
///
/// When the `hsm` feature is not enabled, construction always returns
/// `Err(HsmError::NotEnabled)`. When enabled, it delegates to PKCS#11.
pub struct HsmSigningProvider {
    /// Cached public key bytes (read from HSM during construction).
    public_key: [u8; 32],
    /// Placeholder for the PKCS#11 context — real impl under `#[cfg(feature = "hsm")]`.
    _config: HsmConfig,
}

impl HsmSigningProvider {
    #[allow(dead_code)]
    /// Connect to an HSM and locate the signing key.
    ///
    /// This is a no-op stub when compiled without the `hsm` feature.
    #[cfg(not(feature = "hsm"))]
    pub fn new(
        _pkcs11_lib: &str,
        _slot_id: u64,
        _pin: &str,
        _key_label: &str,
    ) -> Result<Self, HsmError> {
        Err(HsmError::NotEnabled)
    }

    #[cfg(feature = "hsm")]
    pub fn new(
        pkcs11_lib: &str,
        slot_id: u64,
        pin: &str,
        key_label: &str,
    ) -> Result<Self, HsmError> {
        use cryptoki::context::{CInitializeArgs, Pkcs11};

        let ctx = Pkcs11::new(pkcs11_lib).map_err(|e| HsmError::LibraryNotFound(e.to_string()))?;
        ctx.initialize(CInitializeArgs::OsThreads)
            .map_err(|e| HsmError::LibraryNotFound(e.to_string()))?;

        let slots = ctx
            .get_slots_with_token()
            .map_err(|e| HsmError::SlotNotFound(slot_id))?;
        let slot = slots
            .into_iter()
            .find(|s| s.id() == slot_id)
            .ok_or(HsmError::SlotNotFound(slot_id))?;

        // Open session and login.
        let session = ctx
            .open_rw_session(slot)
            .map_err(|e| HsmError::AuthFailed)?;
        session
            .login(cryptoki::session::UserType::User, Some(pin))
            .map_err(|_| HsmError::AuthFailed)?;

        // For now, return a placeholder public key — full PKCS#11 key lookup
        // would require CKA_LABEL search and CKM_EDDSA mechanism support.
        Ok(Self {
            public_key: [0u8; 32],
            _config: HsmConfig {
                pkcs11_lib: pkcs11_lib.to_string(),
                slot_id,
                pin: pin.to_string(),
                key_label: key_label.to_string(),
            },
        })
    }
}

impl super::signing::SigningProvider for HsmSigningProvider {
    fn algorithm(&self) -> super::signing::SigningAlgorithm {
        super::signing::SigningAlgorithm::Ed25519
    }

    #[cfg(not(feature = "hsm"))]
    fn sign(&self, _data: &[u8]) -> Result<Vec<u8>, super::signing::SigningError> {
        Err(super::signing::SigningError::SignFailed(
            "HSM signing not available in this build".into(),
        ))
    }

    #[cfg(feature = "hsm")]
    fn sign(&self, data: &[u8]) -> Result<Vec<u8>, super::signing::SigningError> {
        // In a full implementation, this would:
        // 1. Open a PKCS#11 session to the HSM
        // 2. Find the key by label (CKA_LABEL)
        // 3. Call C_Sign with CKM_EDDSA mechanism
        // 4. Return the 64-byte Ed25519 signature
        //
        // For now, delegate to the software fallback using the cached public key
        // as a placeholder. Real HSM integration requires hardware testing.
        let _ = data;
        Err(super::signing::SigningError::SignFailed(
            "HSM sign: key lookup not yet implemented — requires hardware testing".into(),
        ))
    }

    fn public_key(&self) -> Vec<u8> {
        self.public_key.to_vec()
    }

    #[cfg(not(feature = "hsm"))]
    fn verify(&self, _data: &[u8], _sig: &[u8]) -> Result<bool, super::signing::SigningError> {
        Err(super::signing::SigningError::VerifyFailed(
            "HSM verification not available in this build".into(),
        ))
    }

    #[cfg(feature = "hsm")]
    fn verify(&self, data: &[u8], sig: &[u8]) -> Result<bool, super::signing::SigningError> {
        use ed25519_dalek::{Signature, VerifyingKey};
        let sig_bytes: [u8; 64] = sig.try_into().map_err(|_| {
            super::signing::SigningError::VerifyFailed("Ed25519 signature must be 64 bytes".into())
        })?;
        let vk = VerifyingKey::from_bytes(&self.public_key)
            .map_err(|e| super::signing::SigningError::VerifyFailed(e.to_string()))?;
        let signature = Signature::from_bytes(&sig_bytes);
        Ok(vk.verify_strict(data, &signature).is_ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hsm_config_fields() {
        let cfg = HsmConfig {
            pkcs11_lib: "/usr/lib/softhsm/libsofthsm2.so".into(),
            slot_id: 0,
            pin: "1234".into(),
            key_label: "mykey".into(),
        };
        assert_eq!(cfg.slot_id, 0);
        assert_eq!(cfg.key_label, "mykey");
    }

    #[test]
    fn hsm_provider_not_enabled_without_feature() {
        // Without the hsm feature, construction should fail with NotEnabled.
        #[cfg(not(feature = "hsm"))]
        {
            let result = HsmSigningProvider::new("/lib.so", 0, "pin", "label");
            assert!(matches!(result, Err(HsmError::NotEnabled)));
        }
    }
}
