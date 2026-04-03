//! Endorsement data types

mod sig_hex {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(sig: &[u8; 64], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&hex::encode(sig))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 64], D::Error> {
        let hex_str = String::deserialize(d)?;
        let bytes = hex::decode(&hex_str).map_err(serde::de::Error::custom)?;
        bytes
            .try_into()
            .map_err(|_| serde::de::Error::custom("signature must be 64 bytes"))
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Endorsement {
    /// DID of the signer
    pub signer_did: String,
    /// Organization the signer belongs to
    pub org_id: String,
    /// Ed25519 signature (64 bytes, same layout as `DagBlock.signature`)
    #[serde(with = "sig_hex")]
    pub signature: [u8; 64],
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
            signature: [1u8; 64],
            payload_hash: [2u8; 32],
            timestamp: 1_000_000,
        };
        assert_eq!(e.signer_did, "did:bc:alice");
        assert_eq!(e.org_id, "org1");
    }

    #[test]
    fn signature_is_64_bytes() {
        let e = Endorsement {
            signer_did: "did:bc:bob".to_string(),
            org_id: "org2".to_string(),
            signature: [0u8; 64],
            payload_hash: [0u8; 32],
            timestamp: 0,
        };
        assert_eq!(e.signature.len(), 64);
        assert_eq!(e.payload_hash.len(), 32);
    }
}
