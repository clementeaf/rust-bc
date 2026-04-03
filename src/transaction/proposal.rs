use serde::{Deserialize, Serialize};

use crate::endorsement::types::Endorsement;
use crate::storage::traits::Transaction;
use crate::transaction::rwset::ReadWriteSet;

/// A transaction proposal submitted by a client for endorsement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionProposal {
    pub tx: Transaction,
    pub creator_did: String,
    #[serde(with = "sig_bytes")]
    pub creator_signature: [u8; 64],
    pub rwset: ReadWriteSet,
}

/// An endorser's response to a transaction proposal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalResponse {
    pub rwset: ReadWriteSet,
    pub endorsement: Endorsement,
}

mod sig_bytes {
    use serde::{Deserializer, Serializer};

    pub fn serialize<S: Serializer>(bytes: &[u8; 64], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_bytes(bytes)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 64], D::Error> {
        let v: Vec<u8> = serde::Deserialize::deserialize(d)?;
        v.try_into().map_err(|_| serde::de::Error::custom("expected 64 bytes"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
            reads: vec![KVRead { key: "k".to_string(), version: 1 }],
            writes: vec![KVWrite { key: "k".to_string(), value: vec![1] }],
        }
    }

    fn sample_endorsement() -> Endorsement {
        Endorsement {
            signer_did: "did:example:org1".to_string(),
            org_id: "Org1".to_string(),
            signature: [0u8; 64],
            payload_hash: [0u8; 32],
            timestamp: 0,
        }
    }

    #[test]
    fn creates_transaction_proposal() {
        let proposal = TransactionProposal {
            tx: sample_tx(),
            creator_did: "did:example:alice".to_string(),
            creator_signature: [0u8; 64],
            rwset: sample_rwset(),
        };
        assert_eq!(proposal.creator_did, "did:example:alice");
        assert!(!proposal.rwset.is_empty());
    }

    #[test]
    fn creates_proposal_response() {
        let response = ProposalResponse {
            rwset: sample_rwset(),
            endorsement: sample_endorsement(),
        };
        assert_eq!(response.endorsement.org_id, "Org1");
        assert!(!response.rwset.is_empty());
    }
}
