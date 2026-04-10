use serde::{Deserialize, Serialize};

use crate::endorsement::types::Endorsement;
use crate::transaction::proposal::TransactionProposal;
use crate::transaction::rwset::ReadWriteSet;

/// A transaction proposal that has been endorsed by one or more organizations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndorsedTransaction {
    pub proposal: TransactionProposal,
    pub endorsements: Vec<Endorsement>,
    pub rwset: ReadWriteSet,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::traits::Transaction;
    use crate::transaction::rwset::{KVRead, KVWrite};

    fn sample_tx() -> Transaction {
        Transaction {
            id: "tx-1".to_string(),
            block_height: 0,
            timestamp: 0,
            input_did: "did:example:alice".to_string(),
            output_recipient: "did:example:bob".to_string(),
            amount: 0,
            state: "pending".to_string(),
        }
    }

    fn sample_rwset() -> ReadWriteSet {
        ReadWriteSet {
            reads: vec![KVRead {
                key: "k".to_string(),
                version: 1,
            }],
            writes: vec![KVWrite {
                key: "k".to_string(),
                value: vec![1],
            }],
        }
    }

    fn sample_endorsement(org: &str) -> Endorsement {
        Endorsement {
            signer_did: format!("did:example:{org}"),
            org_id: org.to_string(),
            signature: vec![0u8; 64],
            payload_hash: [0u8; 32],
            timestamp: 0,
        }
    }

    #[test]
    fn creates_endorsed_transaction_with_two_endorsements() {
        let proposal = TransactionProposal {
            tx: sample_tx(),
            creator_did: "did:example:alice".to_string(),
            creator_signature: vec![0u8; 64],
            rwset: sample_rwset(),
        };
        let endorsed = EndorsedTransaction {
            proposal,
            endorsements: vec![sample_endorsement("Org1"), sample_endorsement("Org2")],
            rwset: sample_rwset(),
        };
        assert_eq!(endorsed.endorsements.len(), 2);
        assert_eq!(endorsed.endorsements[0].org_id, "Org1");
        assert_eq!(endorsed.endorsements[1].org_id, "Org2");
        assert!(!endorsed.rwset.is_empty());
    }
}
