//! ACL access-check logic.

use thiserror::Error;

use crate::acl::AclProvider;
use crate::endorsement::policy_store::PolicyStore;

/// Errors returned by [`check_access`].
#[derive(Debug, Error, PartialEq, Eq)]
pub enum AclError {
    /// No ACL entry exists for the resource; deny by default.
    #[error("no ACL defined for resource '{0}'")]
    NotDefined(String),
    /// The policy named in the ACL entry was not found in the policy store.
    #[error("policy '{0}' not found")]
    PolicyNotFound(String),
    /// The caller's orgs do not satisfy the required policy.
    #[error("access denied: caller orgs do not satisfy policy '{0}'")]
    Denied(String),
}

/// Check whether `caller_orgs` may access `resource`.
///
/// Resolution order:
/// 1. Look up the ACL entry for `resource` — deny if absent.
/// 2. Fetch the [`EndorsementPolicy`] named by `policy_ref` — deny if not found.
/// 3. Evaluate the policy against `caller_orgs` — deny if not satisfied.
pub fn check_access(
    acl_provider: &dyn AclProvider,
    policy_store: &dyn PolicyStore,
    resource: &str,
    caller_orgs: &[&str],
) -> Result<(), AclError> {
    let entry = acl_provider
        .get_acl(resource)
        .map_err(|_| AclError::NotDefined(resource.to_string()))?
        .ok_or_else(|| AclError::NotDefined(resource.to_string()))?;

    let policy = policy_store
        .get_policy(&entry.policy_ref)
        .map_err(|_| AclError::PolicyNotFound(entry.policy_ref.clone()))?;

    if policy.evaluate(caller_orgs) {
        Ok(())
    } else {
        Err(AclError::Denied(entry.policy_ref))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::acl::provider::MemoryAclProvider;
    use crate::endorsement::policy::EndorsementPolicy;
    use crate::endorsement::policy_store::MemoryPolicyStore;

    fn setup() -> (MemoryAclProvider, MemoryPolicyStore) {
        let acl = MemoryAclProvider::new();
        let ps = MemoryPolicyStore::new();

        // Policy that requires org1 OR org2
        let policy = EndorsementPolicy::AnyOf(vec!["org1".to_string(), "org2".to_string()]);
        ps.set_policy("OrgPolicy", &policy).unwrap();
        acl.set_acl("peer/ChaincodeInvoke", "OrgPolicy").unwrap();

        (acl, ps)
    }

    #[test]
    fn access_allowed() {
        let (acl, ps) = setup();
        assert!(check_access(&acl, &ps, "peer/ChaincodeInvoke", &["org1"]).is_ok());
    }

    #[test]
    fn access_denied_policy_not_satisfied() {
        let (acl, ps) = setup();
        let err = check_access(&acl, &ps, "peer/ChaincodeInvoke", &["org3"]).unwrap_err();
        assert!(matches!(err, AclError::Denied(_)));
    }

    #[test]
    fn access_denied_no_acl_defined() {
        let (acl, ps) = setup();
        let err = check_access(&acl, &ps, "peer/Unknown", &["org1"]).unwrap_err();
        assert!(matches!(err, AclError::NotDefined(_)));
    }
}
