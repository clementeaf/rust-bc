use std::collections::HashMap;

use crate::chaincode::external::ChaincodeRuntime;
use crate::chaincode::ChaincodeStatus;
use crate::endorsement::EndorsementPolicy;

// ── ChaincodeDefinition ───────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChaincodeDefinition {
    pub chaincode_id: String,
    pub version: String,
    pub status: ChaincodeStatus,
    pub endorsement_policy: EndorsementPolicy,
    /// org_id → has approved
    pub approvals: HashMap<String, bool>,
    /// Runtime mode: in-process Wasm (default) or external HTTP service.
    #[serde(default)]
    pub runtime: ChaincodeRuntime,
}

impl ChaincodeDefinition {
    pub fn new(
        chaincode_id: impl Into<String>,
        version: impl Into<String>,
        endorsement_policy: EndorsementPolicy,
    ) -> Self {
        Self {
            chaincode_id: chaincode_id.into(),
            version: version.into(),
            status: ChaincodeStatus::Installed,
            endorsement_policy,
            approvals: HashMap::new(),
            runtime: ChaincodeRuntime::default(),
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_definition_starts_as_installed() {
        let def = ChaincodeDefinition::new(
            "my_cc",
            "1.0",
            EndorsementPolicy::AnyOf(vec!["org1".to_string()]),
        );
        assert_eq!(def.chaincode_id, "my_cc");
        assert_eq!(def.version, "1.0");
        assert_eq!(def.status, ChaincodeStatus::Installed);
        assert!(def.approvals.is_empty());
    }

    #[test]
    fn approvals_map_tracks_org_decisions() {
        let mut def = ChaincodeDefinition::new(
            "cc1",
            "2.0",
            EndorsementPolicy::AllOf(vec!["org1".to_string(), "org2".to_string()]),
        );
        def.approvals.insert("org1".to_string(), true);
        def.approvals.insert("org2".to_string(), false);

        assert!(def.approvals["org1"]);
        assert!(!def.approvals["org2"]);
    }

    #[test]
    fn status_can_be_advanced_via_transition() {
        let mut def = ChaincodeDefinition::new(
            "cc2",
            "1.0",
            EndorsementPolicy::AnyOf(vec!["org1".to_string()]),
        );
        def.status = def
            .status
            .transition_to(&ChaincodeStatus::Approved)
            .unwrap();
        assert_eq!(def.status, ChaincodeStatus::Approved);

        def.status = def
            .status
            .transition_to(&ChaincodeStatus::Committed)
            .unwrap();
        assert_eq!(def.status, ChaincodeStatus::Committed);
    }
}
