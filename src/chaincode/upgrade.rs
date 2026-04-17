//! Chaincode upgrade lifecycle — multi-org approval before activation.
//!
//! Fabric-compatible upgrade flow:
//! 1. New version installed on peers
//! 2. Each org approves the new definition
//! 3. When enough orgs approve (per upgrade policy), definition is committed
//! 4. Old version deprecated, new version active
//!
//! This prevents unilateral upgrades — all orgs in the endorsement policy
//! must agree before a chaincode version change takes effect.

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

/// An upgrade proposal for a chaincode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpgradeProposal {
    pub chaincode_id: String,
    pub current_version: String,
    pub new_version: String,
    /// SHA-256 hash of the new Wasm package.
    pub package_hash: [u8; 32],
    /// Orgs that have approved this upgrade.
    pub approvals: HashSet<String>,
    /// Orgs required to approve (from endorsement/upgrade policy).
    pub required_orgs: HashSet<String>,
    /// Block height when proposed.
    pub proposed_at: u64,
    /// Whether the upgrade has been committed.
    pub committed: bool,
}

impl UpgradeProposal {
    /// Whether enough orgs have approved.
    pub fn is_ready(&self) -> bool {
        self.required_orgs
            .iter()
            .all(|org| self.approvals.contains(org))
    }

    /// Number of approvals still needed.
    pub fn pending_approvals(&self) -> Vec<&str> {
        self.required_orgs
            .iter()
            .filter(|org| !self.approvals.contains(*org))
            .map(|s| s.as_str())
            .collect()
    }

    /// Approval progress as fraction (0.0 to 1.0).
    pub fn progress(&self) -> f64 {
        if self.required_orgs.is_empty() {
            return 1.0;
        }
        let approved = self
            .required_orgs
            .iter()
            .filter(|org| self.approvals.contains(*org))
            .count();
        approved as f64 / self.required_orgs.len() as f64
    }
}

/// Errors from the upgrade system.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UpgradeError {
    #[error("no pending upgrade for chaincode '{0}'")]
    NotFound(String),
    #[error("upgrade already pending for '{0}' version '{1}'")]
    AlreadyPending(String, String),
    #[error("org '{0}' not in required approvers")]
    UnauthorizedOrg(String),
    #[error("org '{0}' already approved")]
    AlreadyApproved(String),
    #[error("not enough approvals: {have}/{need}")]
    InsufficientApprovals { have: usize, need: usize },
    #[error("upgrade already committed")]
    AlreadyCommitted,
    #[error("version mismatch: expected '{expected}', got '{got}'")]
    VersionMismatch { expected: String, got: String },
}

/// Manages pending chaincode upgrades.
pub struct UpgradeManager {
    /// chaincode_id → pending proposal.
    pending: Mutex<HashMap<String, UpgradeProposal>>,
    /// chaincode_id → list of completed upgrade records.
    history: Mutex<HashMap<String, Vec<UpgradeProposal>>>,
}

impl UpgradeManager {
    pub fn new() -> Self {
        Self {
            pending: Mutex::new(HashMap::new()),
            history: Mutex::new(HashMap::new()),
        }
    }

    /// Propose an upgrade for a chaincode.
    pub fn propose(
        &self,
        chaincode_id: &str,
        current_version: &str,
        new_version: &str,
        package_hash: [u8; 32],
        required_orgs: Vec<String>,
        block_height: u64,
    ) -> Result<(), UpgradeError> {
        let mut pending = self.pending.lock().unwrap();
        if let Some(existing) = pending.get(chaincode_id) {
            if !existing.committed {
                return Err(UpgradeError::AlreadyPending(
                    chaincode_id.into(),
                    existing.new_version.clone(),
                ));
            }
        }

        let proposal = UpgradeProposal {
            chaincode_id: chaincode_id.into(),
            current_version: current_version.into(),
            new_version: new_version.into(),
            package_hash,
            approvals: HashSet::new(),
            required_orgs: required_orgs.into_iter().collect(),
            proposed_at: block_height,
            committed: false,
        };

        pending.insert(chaincode_id.to_string(), proposal);
        Ok(())
    }

    /// Approve a pending upgrade on behalf of an org.
    pub fn approve(
        &self,
        chaincode_id: &str,
        org_id: &str,
    ) -> Result<UpgradeProposal, UpgradeError> {
        let mut pending = self.pending.lock().unwrap();
        let proposal = pending
            .get_mut(chaincode_id)
            .ok_or_else(|| UpgradeError::NotFound(chaincode_id.into()))?;

        if proposal.committed {
            return Err(UpgradeError::AlreadyCommitted);
        }

        if !proposal.required_orgs.contains(org_id) {
            return Err(UpgradeError::UnauthorizedOrg(org_id.into()));
        }

        if proposal.approvals.contains(org_id) {
            return Err(UpgradeError::AlreadyApproved(org_id.into()));
        }

        proposal.approvals.insert(org_id.to_string());
        Ok(proposal.clone())
    }

    /// Commit the upgrade if all required orgs have approved.
    pub fn commit(&self, chaincode_id: &str) -> Result<UpgradeProposal, UpgradeError> {
        let mut pending = self.pending.lock().unwrap();
        let proposal = pending
            .get_mut(chaincode_id)
            .ok_or_else(|| UpgradeError::NotFound(chaincode_id.into()))?;

        if proposal.committed {
            return Err(UpgradeError::AlreadyCommitted);
        }

        if !proposal.is_ready() {
            let have = proposal.approvals.len();
            let need = proposal.required_orgs.len();
            return Err(UpgradeError::InsufficientApprovals { have, need });
        }

        proposal.committed = true;
        let result = proposal.clone();

        // Move to history.
        self.history
            .lock()
            .unwrap()
            .entry(chaincode_id.to_string())
            .or_default()
            .push(result.clone());

        Ok(result)
    }

    /// Get the pending proposal for a chaincode.
    pub fn get_pending(&self, chaincode_id: &str) -> Option<UpgradeProposal> {
        self.pending
            .lock()
            .unwrap()
            .get(chaincode_id)
            .filter(|p| !p.committed)
            .cloned()
    }

    /// Get upgrade history for a chaincode.
    pub fn get_history(&self, chaincode_id: &str) -> Vec<UpgradeProposal> {
        self.history
            .lock()
            .unwrap()
            .get(chaincode_id)
            .cloned()
            .unwrap_or_default()
    }
}

impl Default for UpgradeManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn orgs(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("org{i}")).collect()
    }

    fn hash(id: u8) -> [u8; 32] {
        let mut h = [0u8; 32];
        h[0] = id;
        h
    }

    // --- propose ---

    #[test]
    fn propose_upgrade() {
        let mgr = UpgradeManager::new();
        mgr.propose("mycc", "1.0", "2.0", hash(1), orgs(3), 100)
            .unwrap();
        let p = mgr.get_pending("mycc").unwrap();
        assert_eq!(p.new_version, "2.0");
        assert!(!p.committed);
        assert!(p.approvals.is_empty());
    }

    #[test]
    fn propose_duplicate_fails() {
        let mgr = UpgradeManager::new();
        mgr.propose("mycc", "1.0", "2.0", hash(1), orgs(2), 100)
            .unwrap();
        let err = mgr
            .propose("mycc", "1.0", "3.0", hash(2), orgs(2), 101)
            .unwrap_err();
        assert!(matches!(err, UpgradeError::AlreadyPending(_, _)));
    }

    // --- approve ---

    #[test]
    fn approve_by_required_org() {
        let mgr = UpgradeManager::new();
        mgr.propose("mycc", "1.0", "2.0", hash(1), orgs(3), 100)
            .unwrap();
        let p = mgr.approve("mycc", "org0").unwrap();
        assert!(p.approvals.contains("org0"));
    }

    #[test]
    fn approve_by_unauthorized_org_fails() {
        let mgr = UpgradeManager::new();
        mgr.propose("mycc", "1.0", "2.0", hash(1), orgs(2), 100)
            .unwrap();
        let err = mgr.approve("mycc", "intruder").unwrap_err();
        assert!(matches!(err, UpgradeError::UnauthorizedOrg(_)));
    }

    #[test]
    fn approve_duplicate_fails() {
        let mgr = UpgradeManager::new();
        mgr.propose("mycc", "1.0", "2.0", hash(1), orgs(2), 100)
            .unwrap();
        mgr.approve("mycc", "org0").unwrap();
        let err = mgr.approve("mycc", "org0").unwrap_err();
        assert!(matches!(err, UpgradeError::AlreadyApproved(_)));
    }

    // --- commit ---

    #[test]
    fn commit_with_all_approvals() {
        let mgr = UpgradeManager::new();
        mgr.propose("mycc", "1.0", "2.0", hash(1), orgs(2), 100)
            .unwrap();
        mgr.approve("mycc", "org0").unwrap();
        mgr.approve("mycc", "org1").unwrap();

        let result = mgr.commit("mycc").unwrap();
        assert!(result.committed);
        assert!(result.is_ready());
    }

    #[test]
    fn commit_insufficient_approvals_fails() {
        let mgr = UpgradeManager::new();
        mgr.propose("mycc", "1.0", "2.0", hash(1), orgs(3), 100)
            .unwrap();
        mgr.approve("mycc", "org0").unwrap();

        let err = mgr.commit("mycc").unwrap_err();
        assert!(matches!(
            err,
            UpgradeError::InsufficientApprovals { have: 1, need: 3 }
        ));
    }

    #[test]
    fn commit_already_committed_fails() {
        let mgr = UpgradeManager::new();
        mgr.propose("mycc", "1.0", "2.0", hash(1), orgs(1), 100)
            .unwrap();
        mgr.approve("mycc", "org0").unwrap();
        mgr.commit("mycc").unwrap();

        let err = mgr.commit("mycc").unwrap_err();
        assert!(matches!(err, UpgradeError::AlreadyCommitted));
    }

    // --- progress ---

    #[test]
    fn progress_tracks_approvals() {
        let mgr = UpgradeManager::new();
        mgr.propose("mycc", "1.0", "2.0", hash(1), orgs(4), 100)
            .unwrap();

        let p = mgr.get_pending("mycc").unwrap();
        assert!((p.progress() - 0.0).abs() < f64::EPSILON);

        mgr.approve("mycc", "org0").unwrap();
        mgr.approve("mycc", "org1").unwrap();
        let p = mgr.get_pending("mycc").unwrap();
        assert!((p.progress() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn pending_approvals_lists_remaining() {
        let mgr = UpgradeManager::new();
        mgr.propose("mycc", "1.0", "2.0", hash(1), orgs(3), 100)
            .unwrap();
        mgr.approve("mycc", "org0").unwrap();

        let p = mgr.get_pending("mycc").unwrap();
        let mut pending = p.pending_approvals();
        pending.sort();
        assert_eq!(pending, vec!["org1", "org2"]);
    }

    // --- history ---

    #[test]
    fn history_records_committed_upgrades() {
        let mgr = UpgradeManager::new();
        mgr.propose("mycc", "1.0", "2.0", hash(1), orgs(1), 100)
            .unwrap();
        mgr.approve("mycc", "org0").unwrap();
        mgr.commit("mycc").unwrap();

        let history = mgr.get_history("mycc");
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].new_version, "2.0");
    }

    #[test]
    fn no_pending_after_commit() {
        let mgr = UpgradeManager::new();
        mgr.propose("mycc", "1.0", "2.0", hash(1), orgs(1), 100)
            .unwrap();
        mgr.approve("mycc", "org0").unwrap();
        mgr.commit("mycc").unwrap();

        assert!(mgr.get_pending("mycc").is_none());
    }

    // --- full lifecycle ---

    #[test]
    fn full_upgrade_lifecycle() {
        let mgr = UpgradeManager::new();

        // v1 → v2: 3 orgs must approve.
        mgr.propose("mycc", "1.0", "2.0", hash(1), orgs(3), 100)
            .unwrap();
        assert_eq!(mgr.get_pending("mycc").unwrap().progress(), 0.0);

        mgr.approve("mycc", "org0").unwrap();
        mgr.approve("mycc", "org1").unwrap();
        assert!(!mgr.get_pending("mycc").unwrap().is_ready());

        mgr.approve("mycc", "org2").unwrap();
        assert!(mgr.get_pending("mycc").unwrap().is_ready());

        let committed = mgr.commit("mycc").unwrap();
        assert!(committed.committed);
        assert_eq!(committed.new_version, "2.0");

        // Can propose next upgrade now.
        mgr.propose("mycc", "2.0", "3.0", hash(2), orgs(3), 200)
            .unwrap();
        assert_eq!(mgr.get_pending("mycc").unwrap().new_version, "3.0");
        assert_eq!(mgr.get_history("mycc").len(), 1);
    }
}
