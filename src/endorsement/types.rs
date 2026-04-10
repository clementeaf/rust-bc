//! Endorsement data types

mod vec_hex {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(bytes: &Vec<u8>, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&hex::encode(bytes))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let hex_str = String::deserialize(d)?;
        hex::decode(&hex_str).map_err(serde::de::Error::custom)
    }
}

mod hash_hex {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(hash: &[u8; 32], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&hex::encode(hash))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 32], D::Error> {
        let hex_str = String::deserialize(d)?;
        let bytes = hex::decode(&hex_str).map_err(serde::de::Error::custom)?;
        bytes
            .try_into()
            .map_err(|_| serde::de::Error::custom("hash must be 32 bytes"))
    }
}

/// A single endorsement: a signature over a payload hash by an org member.
///
/// The `signature` field is variable-length to support both Ed25519 (64 bytes)
/// and post-quantum algorithms like ML-DSA-65 (3309 bytes).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Endorsement {
    /// DID of the signer
    pub signer_did: String,
    /// Organization the signer belongs to
    pub org_id: String,
    /// Signature bytes (variable-length: Ed25519 = 64, ML-DSA-65 = 3309)
    #[serde(with = "vec_hex")]
    pub signature: Vec<u8>,
    /// Hash of the signed payload (32 bytes)
    #[serde(with = "hash_hex")]
    pub payload_hash: [u8; 32],
    /// Unix timestamp when the endorsement was created
    pub timestamp: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_endorsement() {
        let e = Endorsement {
            signer_did: "did:bc:alice".to_string(),
            org_id: "org1".to_string(),
            signature: vec![1u8; 64],
            payload_hash: [2u8; 32],
            timestamp: 1_000_000,
        };
        assert_eq!(e.signer_did, "did:bc:alice");
        assert_eq!(e.org_id, "org1");
    }

    #[test]
    fn endorsement_supports_variable_length_signature() {
        // Ed25519-sized
        let e1 = Endorsement {
            signer_did: "did:bc:bob".to_string(),
            org_id: "org2".to_string(),
            signature: vec![0u8; 64],
            payload_hash: [0u8; 32],
            timestamp: 0,
        };
        assert_eq!(e1.signature.len(), 64);

        // ML-DSA-65-sized
        let e2 = Endorsement {
            signer_did: "did:bc:carol".to_string(),
            org_id: "org3".to_string(),
            signature: vec![0u8; 3309],
            payload_hash: [0u8; 32],
            timestamp: 0,
        };
        assert_eq!(e2.signature.len(), 3309);
    }

    #[test]
    fn serde_roundtrip_variable_signature() {
        let e = Endorsement {
            signer_did: "did:bc:test".to_string(),
            org_id: "org1".to_string(),
            signature: vec![42u8; 3309],
            payload_hash: [7u8; 32],
            timestamp: 999,
        };
        let json = serde_json::to_string(&e).unwrap();
        let decoded: Endorsement = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.signature.len(), 3309);
        assert_eq!(decoded, e);
    }
}
