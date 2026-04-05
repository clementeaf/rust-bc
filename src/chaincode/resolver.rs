use std::sync::Arc;

use super::{ChaincodeError, ChaincodePackageStore};

/// Resolves a chaincode ID to its Wasm bytes.
///
/// Used for chaincode-to-chaincode invocation: the calling chaincode
/// references a target by ID, and the resolver fetches the binary.
pub trait ChaincodeResolver: Send + Sync {
    fn resolve(&self, chaincode_id: &str) -> Result<Vec<u8>, ChaincodeError>;
}

/// Resolver backed by a [`ChaincodePackageStore`].
///
/// Always resolves to the `"latest"` version of the chaincode.
pub struct StoreBackedResolver {
    store: Arc<dyn ChaincodePackageStore>,
}

impl StoreBackedResolver {
    pub fn new(store: Arc<dyn ChaincodePackageStore>) -> Self {
        Self { store }
    }
}

impl ChaincodeResolver for StoreBackedResolver {
    fn resolve(&self, chaincode_id: &str) -> Result<Vec<u8>, ChaincodeError> {
        self.store
            .get_package(chaincode_id, "latest")?
            .ok_or_else(|| ChaincodeError::NotFound(format!("chaincode '{}' not found", chaincode_id)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chaincode::MemoryChaincodePackageStore;

    #[test]
    fn resolve_existing_chaincode() {
        let store = Arc::new(MemoryChaincodePackageStore::new());
        store.store_package("mycc", "latest", b"(module)").unwrap();

        let resolver = StoreBackedResolver::new(store);
        let bytes = resolver.resolve("mycc").unwrap();
        assert_eq!(bytes, b"(module)");
    }

    #[test]
    fn resolve_missing_chaincode_returns_error() {
        let store = Arc::new(MemoryChaincodePackageStore::new());
        let resolver = StoreBackedResolver::new(store);
        let err = resolver.resolve("nonexistent").unwrap_err();
        assert!(matches!(err, ChaincodeError::NotFound(_)));
    }
}
