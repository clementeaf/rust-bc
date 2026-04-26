//! DID-to-PIN association store.

use std::collections::HashMap;
use std::sync::RwLock;

use super::generator::{Pin, PinError};

/// Persistent mapping from DID to hashed PIN.
pub trait PinStore: Send + Sync {
    /// Store a hashed PIN for a DID. Overwrites any previous PIN.
    fn set(&self, did: &str, pin_hash: &str) -> Result<(), PinError>;

    /// Retrieve the stored hash for a DID, if any.
    fn get_hash(&self, did: &str) -> Result<Option<String>, PinError>;

    /// Verify a plaintext PIN against the stored hash for a DID.
    fn verify(&self, did: &str, plain_pin: &str) -> Result<(), PinError>;

    /// Remove the PIN for a DID. Returns true if a PIN was removed.
    fn remove(&self, did: &str) -> Result<bool, PinError>;
}

/// In-memory implementation backed by a `HashMap`.
pub struct MemoryPinStore {
    data: RwLock<HashMap<String, String>>,
}

impl MemoryPinStore {
    pub fn new() -> Self {
        Self {
            data: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for MemoryPinStore {
    fn default() -> Self {
        Self::new()
    }
}

impl PinStore for MemoryPinStore {
    fn set(&self, did: &str, pin_hash: &str) -> Result<(), PinError> {
        let mut map = self
            .data
            .write()
            .map_err(|e| PinError::HashError(format!("lock poisoned: {e}")))?;
        map.insert(did.to_string(), pin_hash.to_string());
        Ok(())
    }

    fn get_hash(&self, did: &str) -> Result<Option<String>, PinError> {
        let map = self
            .data
            .read()
            .map_err(|e| PinError::HashError(format!("lock poisoned: {e}")))?;
        Ok(map.get(did).cloned())
    }

    fn verify(&self, did: &str, plain_pin: &str) -> Result<(), PinError> {
        let hash = self
            .get_hash(did)?
            .ok_or_else(|| PinError::HashError(format!("no PIN registered for DID: {did}")))?;
        Pin::verify(plain_pin, &hash)
    }

    fn remove(&self, did: &str) -> Result<bool, PinError> {
        let mut map = self
            .data
            .write()
            .map_err(|e| PinError::HashError(format!("lock poisoned: {e}")))?;
        Ok(map.remove(did).is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_and_verify_pin() {
        let store = MemoryPinStore::new();
        let pin = Pin::generate(4).unwrap();
        let hash = pin.hash().unwrap();

        store.set("did:cerulean:alice", &hash).unwrap();
        assert!(store.verify("did:cerulean:alice", pin.as_str()).is_ok());
    }

    #[test]
    fn verify_wrong_pin_fails() {
        let store = MemoryPinStore::new();
        let pin = Pin::generate(6).unwrap();
        let hash = pin.hash().unwrap();

        store.set("did:cerulean:bob", &hash).unwrap();
        let result = store.verify("did:cerulean:bob", "000000");
        if pin.as_str() != "000000" {
            assert!(result.is_err());
        }
    }

    #[test]
    fn verify_unknown_did_fails() {
        let store = MemoryPinStore::new();
        let result = store.verify("did:cerulean:unknown", "1234");
        assert!(result.is_err());
    }

    #[test]
    fn overwrite_pin() {
        let store = MemoryPinStore::new();
        let pin1 = Pin::generate(4).unwrap();
        let pin2 = Pin::generate(4).unwrap();

        store
            .set("did:cerulean:carol", &pin1.hash().unwrap())
            .unwrap();
        store
            .set("did:cerulean:carol", &pin2.hash().unwrap())
            .unwrap();

        assert!(store.verify("did:cerulean:carol", pin2.as_str()).is_ok());
    }

    #[test]
    fn remove_pin() {
        let store = MemoryPinStore::new();
        let pin = Pin::generate(4).unwrap();
        store
            .set("did:cerulean:dave", &pin.hash().unwrap())
            .unwrap();

        assert!(store.remove("did:cerulean:dave").unwrap());
        assert!(!store.remove("did:cerulean:dave").unwrap());
        assert!(store.verify("did:cerulean:dave", pin.as_str()).is_err());
    }

    #[test]
    fn get_hash_returns_none_for_unknown() {
        let store = MemoryPinStore::new();
        assert!(store.get_hash("did:cerulean:nobody").unwrap().is_none());
    }
}
