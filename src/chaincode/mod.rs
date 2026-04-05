pub mod definition;
pub mod executor;
pub mod external;
pub mod invoker;
pub mod resolver;
pub mod simulation;

use std::collections::HashMap;
use std::sync::Mutex;

use thiserror::Error;

use crate::chaincode::definition::ChaincodeDefinition;

// ── ChaincodeStatus ───────────────────────────────────────────────────────────

/// Lifecycle states of a chaincode on the network.
///
/// Valid transition path:
///   `Installed` → `Approved` → `Committed`
///
/// `Deprecated` is a terminal state reachable from `Committed`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChaincodeStatus {
    Installed,
    Approved,
    Committed,
    Deprecated,
}

impl ChaincodeStatus {
    /// Attempt to advance to `next`. Returns `Err` for invalid transitions.
    pub fn transition_to(&self, next: &ChaincodeStatus) -> Result<ChaincodeStatus, ChaincodeError> {
        let valid = matches!(
            (self, next),
            (ChaincodeStatus::Installed, ChaincodeStatus::Approved)
                | (ChaincodeStatus::Approved, ChaincodeStatus::Committed)
                | (ChaincodeStatus::Committed, ChaincodeStatus::Deprecated)
        );
        if valid {
            Ok(next.clone())
        } else {
            Err(ChaincodeError::InvalidTransition {
                from: format!("{self:?}"),
                to: format!("{next:?}"),
            })
        }
    }
}

// ── ChaincodePackageStore ─────────────────────────────────────────────────────

/// Persistence for raw Wasm chaincode packages.
pub trait ChaincodePackageStore: Send + Sync {
    fn store_package(&self, chaincode_id: &str, version: &str, wasm_bytes: &[u8]) -> Result<(), ChaincodeError>;
    fn get_package(&self, chaincode_id: &str, version: &str) -> Result<Option<Vec<u8>>, ChaincodeError>;
}

/// In-memory implementation for testing.
pub struct MemoryChaincodePackageStore {
    packages: Mutex<HashMap<String, Vec<u8>>>,
}

impl MemoryChaincodePackageStore {
    pub fn new() -> Self {
        Self { packages: Mutex::new(HashMap::new()) }
    }

    fn key(chaincode_id: &str, version: &str) -> String {
        format!("{chaincode_id}:{version}")
    }
}

impl Default for MemoryChaincodePackageStore {
    fn default() -> Self {
        Self::new()
    }
}

impl ChaincodePackageStore for MemoryChaincodePackageStore {
    fn store_package(&self, chaincode_id: &str, version: &str, wasm_bytes: &[u8]) -> Result<(), ChaincodeError> {
        self.packages.lock().unwrap().insert(Self::key(chaincode_id, version), wasm_bytes.to_vec());
        Ok(())
    }

    fn get_package(&self, chaincode_id: &str, version: &str) -> Result<Option<Vec<u8>>, ChaincodeError> {
        Ok(self.packages.lock().unwrap().get(&Self::key(chaincode_id, version)).cloned())
    }
}

// ── ChaincodeDefinitionStore ──────────────────────────────────────────────────

/// Persistence for chaincode lifecycle definitions.
pub trait ChaincodeDefinitionStore: Send + Sync {
    fn upsert_definition(&self, def: ChaincodeDefinition) -> Result<(), ChaincodeError>;
    fn get_definition(&self, chaincode_id: &str, version: &str) -> Result<Option<ChaincodeDefinition>, ChaincodeError>;
}

/// In-memory implementation for testing.
pub struct MemoryChaincodeDefinitionStore {
    defs: Mutex<HashMap<String, ChaincodeDefinition>>,
}

impl MemoryChaincodeDefinitionStore {
    pub fn new() -> Self {
        Self { defs: Mutex::new(HashMap::new()) }
    }

    fn key(chaincode_id: &str, version: &str) -> String {
        format!("{chaincode_id}:{version}")
    }
}

impl Default for MemoryChaincodeDefinitionStore {
    fn default() -> Self {
        Self::new()
    }
}

impl ChaincodeDefinitionStore for MemoryChaincodeDefinitionStore {
    fn upsert_definition(&self, def: ChaincodeDefinition) -> Result<(), ChaincodeError> {
        let key = Self::key(&def.chaincode_id, &def.version);
        self.defs.lock().unwrap().insert(key, def);
        Ok(())
    }

    fn get_definition(&self, chaincode_id: &str, version: &str) -> Result<Option<ChaincodeDefinition>, ChaincodeError> {
        Ok(self.defs.lock().unwrap().get(&Self::key(chaincode_id, version)).cloned())
    }
}

impl<T: ChaincodeDefinitionStore> ChaincodeDefinitionStore for std::sync::Arc<T> {
    fn upsert_definition(&self, def: ChaincodeDefinition) -> Result<(), ChaincodeError> {
        (**self).upsert_definition(def)
    }

    fn get_definition(&self, chaincode_id: &str, version: &str) -> Result<Option<ChaincodeDefinition>, ChaincodeError> {
        (**self).get_definition(chaincode_id, version)
    }
}

// ── ChaincodeError ────────────────────────────────────────────────────────────

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ChaincodeError {
    #[error("invalid lifecycle transition: {from} → {to}")]
    InvalidTransition { from: String, to: String },
    #[error("storage error: {0}")]
    Storage(String),
    #[error("wasm execution error: {0}")]
    Execution(String),
    #[error("not found: {0}")]
    NotFound(String),
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installed_to_approved_is_valid() {
        let result = ChaincodeStatus::Installed.transition_to(&ChaincodeStatus::Approved);
        assert_eq!(result, Ok(ChaincodeStatus::Approved));
    }

    #[test]
    fn approved_to_committed_is_valid() {
        let result = ChaincodeStatus::Approved.transition_to(&ChaincodeStatus::Committed);
        assert_eq!(result, Ok(ChaincodeStatus::Committed));
    }

    #[test]
    fn committed_to_deprecated_is_valid() {
        let result = ChaincodeStatus::Committed.transition_to(&ChaincodeStatus::Deprecated);
        assert_eq!(result, Ok(ChaincodeStatus::Deprecated));
    }

    #[test]
    fn installed_to_committed_is_invalid() {
        let result = ChaincodeStatus::Installed.transition_to(&ChaincodeStatus::Committed);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Installed"));
        assert!(err.to_string().contains("Committed"));
    }

    #[test]
    fn installed_to_deprecated_is_invalid() {
        let result = ChaincodeStatus::Installed.transition_to(&ChaincodeStatus::Deprecated);
        assert!(result.is_err());
    }

    #[test]
    fn approved_to_installed_is_invalid() {
        let result = ChaincodeStatus::Approved.transition_to(&ChaincodeStatus::Installed);
        assert!(result.is_err());
    }

    #[test]
    fn committed_to_installed_is_invalid() {
        let result = ChaincodeStatus::Committed.transition_to(&ChaincodeStatus::Installed);
        assert!(result.is_err());
    }

    #[test]
    fn deprecated_has_no_valid_transitions() {
        for next in [
            ChaincodeStatus::Installed,
            ChaincodeStatus::Approved,
            ChaincodeStatus::Committed,
        ] {
            let result = ChaincodeStatus::Deprecated.transition_to(&next);
            assert!(result.is_err(), "expected error transitioning from Deprecated to {next:?}");
        }
    }
}
