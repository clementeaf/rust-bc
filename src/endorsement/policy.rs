//! Endorsement policy definitions

/// Composable endorsement policy
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum EndorsementPolicy {
    /// At least one of the listed orgs must sign
    AnyOf(Vec<String>),
    /// All of the listed orgs must sign
    AllOf(Vec<String>),
    /// At least `n` of the listed orgs must sign
    NOutOf { n: usize, orgs: Vec<String> },
    /// Both sub-policies must be satisfied
    And(Box<EndorsementPolicy>, Box<EndorsementPolicy>),
    /// At least one sub-policy must be satisfied
    Or(Box<EndorsementPolicy>, Box<EndorsementPolicy>),
    /// At least `min_count` signers whose OU is in `ou_ids` must sign.
    ///
    /// Evaluation requires an external lookup of signer → OU mapping;
    /// the simple `evaluate(&[&str])` method treats `ou_ids` as org IDs
    /// for compatibility. Full OU-aware evaluation is done via `evaluate_with_ous`.
    OuBased { ou_ids: Vec<String>, min_count: usize },
}

impl EndorsementPolicy {
    /// Evaluate the policy against a set of signer org IDs.
    pub fn evaluate(&self, signer_orgs: &[&str]) -> bool {
        match self {
            EndorsementPolicy::AnyOf(orgs) => orgs.iter().any(|o| signer_orgs.contains(&o.as_str())),
            EndorsementPolicy::AllOf(orgs) => orgs.iter().all(|o| signer_orgs.contains(&o.as_str())),
            EndorsementPolicy::NOutOf { n, orgs } => {
                let count = orgs.iter().filter(|o| signer_orgs.contains(&o.as_str())).count();
                count >= *n
            }
            EndorsementPolicy::And(a, b) => a.evaluate(signer_orgs) && b.evaluate(signer_orgs),
            EndorsementPolicy::Or(a, b) => a.evaluate(signer_orgs) || b.evaluate(signer_orgs),
            EndorsementPolicy::OuBased { ou_ids, min_count } => {
                // Fallback: treat ou_ids as org IDs when no OU mapping is available.
                let count = ou_ids.iter().filter(|o| signer_orgs.contains(&o.as_str())).count();
                count >= *min_count
            }
        }
    }

    /// Evaluate the policy with an explicit signer-to-OU mapping.
    ///
    /// `signer_ous` maps each signer org ID to its OU ID.
    pub fn evaluate_with_ous(&self, signer_orgs: &[&str], signer_ous: &std::collections::HashMap<String, String>) -> bool {
        match self {
            EndorsementPolicy::OuBased { ou_ids, min_count } => {
                let count = signer_orgs
                    .iter()
                    .filter(|org| signer_ous.get(**org).map(|ou| ou_ids.contains(ou)).unwrap_or(false))
                    .count();
                count >= *min_count
            }
            // For non-OU policies, delegate to the simple evaluate.
            _ => self.evaluate(signer_orgs),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper
    fn orgs<'a>(ids: &[&'a str]) -> Vec<&'a str> {
        ids.to_vec()
    }

    #[test]
    fn serde_roundtrip_any_of() {
        let p = EndorsementPolicy::AnyOf(vec!["org1".into(), "org2".into()]);
        let json = serde_json::to_string(&p).unwrap();
        assert_eq!(serde_json::from_str::<EndorsementPolicy>(&json).unwrap(), p);
    }

    #[test]
    fn serde_roundtrip_all_of() {
        let p = EndorsementPolicy::AllOf(vec!["org1".into()]);
        let json = serde_json::to_string(&p).unwrap();
        assert_eq!(serde_json::from_str::<EndorsementPolicy>(&json).unwrap(), p);
    }

    #[test]
    fn serde_roundtrip_n_out_of() {
        let p = EndorsementPolicy::NOutOf { n: 2, orgs: vec!["org1".into(), "org2".into(), "org3".into()] };
        let json = serde_json::to_string(&p).unwrap();
        assert_eq!(serde_json::from_str::<EndorsementPolicy>(&json).unwrap(), p);
    }

    #[test]
    fn serde_roundtrip_and() {
        let p = EndorsementPolicy::And(
            Box::new(EndorsementPolicy::AnyOf(vec!["org1".into()])),
            Box::new(EndorsementPolicy::AnyOf(vec!["org2".into()])),
        );
        let json = serde_json::to_string(&p).unwrap();
        assert_eq!(serde_json::from_str::<EndorsementPolicy>(&json).unwrap(), p);
    }

    #[test]
    fn serde_roundtrip_or() {
        let p = EndorsementPolicy::Or(
            Box::new(EndorsementPolicy::AnyOf(vec!["org1".into()])),
            Box::new(EndorsementPolicy::AnyOf(vec!["org2".into()])),
        );
        let json = serde_json::to_string(&p).unwrap();
        assert_eq!(serde_json::from_str::<EndorsementPolicy>(&json).unwrap(), p);
    }

    // evaluate tests
    #[test]
    fn any_of_no_match() {
        let p = EndorsementPolicy::AnyOf(vec!["org1".into()]);
        assert!(!p.evaluate(&orgs(&["org2"])));
    }

    #[test]
    fn any_of_one_match() {
        let p = EndorsementPolicy::AnyOf(vec!["org1".into(), "org2".into()]);
        assert!(p.evaluate(&orgs(&["org2"])));
    }

    #[test]
    fn all_of_partial() {
        let p = EndorsementPolicy::AllOf(vec!["org1".into(), "org2".into()]);
        assert!(!p.evaluate(&orgs(&["org1"])));
    }

    #[test]
    fn all_of_complete() {
        let p = EndorsementPolicy::AllOf(vec!["org1".into(), "org2".into()]);
        assert!(p.evaluate(&orgs(&["org1", "org2"])));
    }

    #[test]
    fn n_out_of_below_n() {
        let p = EndorsementPolicy::NOutOf { n: 2, orgs: vec!["org1".into(), "org2".into(), "org3".into()] };
        assert!(!p.evaluate(&orgs(&["org1"])));
    }

    #[test]
    fn n_out_of_exact_n() {
        let p = EndorsementPolicy::NOutOf { n: 2, orgs: vec!["org1".into(), "org2".into(), "org3".into()] };
        assert!(p.evaluate(&orgs(&["org1", "org2"])));
    }

    #[test]
    fn n_out_of_above_n() {
        let p = EndorsementPolicy::NOutOf { n: 2, orgs: vec!["org1".into(), "org2".into(), "org3".into()] };
        assert!(p.evaluate(&orgs(&["org1", "org2", "org3"])));
    }

    #[test]
    fn and_true_false() {
        let p = EndorsementPolicy::And(
            Box::new(EndorsementPolicy::AnyOf(vec!["org1".into()])),
            Box::new(EndorsementPolicy::AnyOf(vec!["org2".into()])),
        );
        assert!(!p.evaluate(&orgs(&["org1"])));
    }

    #[test]
    fn and_true_true() {
        let p = EndorsementPolicy::And(
            Box::new(EndorsementPolicy::AnyOf(vec!["org1".into()])),
            Box::new(EndorsementPolicy::AnyOf(vec!["org2".into()])),
        );
        assert!(p.evaluate(&orgs(&["org1", "org2"])));
    }

    #[test]
    fn or_false_false() {
        let p = EndorsementPolicy::Or(
            Box::new(EndorsementPolicy::AnyOf(vec!["org1".into()])),
            Box::new(EndorsementPolicy::AnyOf(vec!["org2".into()])),
        );
        assert!(!p.evaluate(&orgs(&["org3"])));
    }

    #[test]
    fn or_false_true() {
        let p = EndorsementPolicy::Or(
            Box::new(EndorsementPolicy::AnyOf(vec!["org1".into()])),
            Box::new(EndorsementPolicy::AnyOf(vec!["org2".into()])),
        );
        assert!(p.evaluate(&orgs(&["org2"])));
    }

    #[test]
    fn ou_based_serde_roundtrip() {
        let p = EndorsementPolicy::OuBased {
            ou_ids: vec!["manufacturing".into()],
            min_count: 2,
        };
        let json = serde_json::to_string(&p).unwrap();
        assert_eq!(serde_json::from_str::<EndorsementPolicy>(&json).unwrap(), p);
    }

    #[test]
    fn ou_based_evaluate_with_ous_pass() {
        use std::collections::HashMap;
        let p = EndorsementPolicy::OuBased {
            ou_ids: vec!["manufacturing".into()],
            min_count: 2,
        };
        let mut ous = HashMap::new();
        ous.insert("org1".to_string(), "manufacturing".to_string());
        ous.insert("org2".to_string(), "manufacturing".to_string());
        ous.insert("org3".to_string(), "engineering".to_string());

        assert!(p.evaluate_with_ous(&["org1", "org2", "org3"], &ous));
    }

    #[test]
    fn ou_based_evaluate_with_ous_fail() {
        use std::collections::HashMap;
        let p = EndorsementPolicy::OuBased {
            ou_ids: vec!["manufacturing".into()],
            min_count: 2,
        };
        let mut ous = HashMap::new();
        ous.insert("org1".to_string(), "manufacturing".to_string());
        ous.insert("org2".to_string(), "engineering".to_string());

        assert!(!p.evaluate_with_ous(&["org1", "org2"], &ous));
    }
}
