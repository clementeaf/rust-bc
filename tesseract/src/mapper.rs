//! Coordinate mapper: real-world events → 4D coordinates.
//!
//! Maps transaction data to deterministic positions in the tesseract field.
//! Related events land near each other → orbitals overlap → emergent links.

use crate::Coord;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};

/// An event in the real world that needs to be placed in the field.
#[derive(Clone, Debug)]
pub struct Event {
    /// Unique identifier (e.g., transaction hash)
    pub id: String,
    /// Timestamp (unix seconds)
    pub timestamp: u64,
    /// Channel or context (e.g., "payments", "identity", "supply-chain")
    pub channel: String,
    /// Organization or identity of the initiator
    pub org: String,
    /// Payload data
    pub data: String,
}

/// Maps events to 4D coordinates deterministically.
///
/// **Identity binding:** Use [`SignedEvent`] for production. It derives
/// `org` from the signer's Ed25519 public key, preventing identity spoofing.
/// Raw [`Event`] with manual `org` is kept for tests and backward compatibility.
pub struct CoordMapper {
    pub size: usize,
    /// Time bucket size in seconds. Events within the same bucket
    /// get the same t-coordinate → temporal proximity.
    pub time_bucket_secs: u64,
    /// Base timestamp (genesis). Coordinates are relative to this.
    pub genesis_time: u64,
}

impl CoordMapper {
    pub fn new(size: usize) -> Self {
        Self {
            size,
            time_bucket_secs: 60, // 1-minute buckets by default
            genesis_time: 0,
        }
    }

    pub fn with_time_bucket(mut self, secs: u64) -> Self {
        self.time_bucket_secs = secs;
        self
    }

    pub fn with_genesis(mut self, genesis: u64) -> Self {
        self.genesis_time = genesis;
        self
    }

    /// Map an event to a 4D coordinate.
    ///
    /// - t: derived from timestamp (bucketed, wrapping)
    /// - c: derived from channel name (hashed)
    /// - o: derived from org/identity (hashed)
    /// - v: derived from event id (hashed) — ensures uniqueness within (t,c,o)
    pub fn map(&self, event: &Event) -> Coord {
        let t = self.map_time(event.timestamp);
        let c = self.hash_to_axis(&event.channel);
        let o = self.hash_to_axis(&event.org);
        let v = self.hash_to_axis(&event.id);

        Coord { t, c, o, v }
    }

    /// Time axis: bucket timestamps so nearby events share the same t.
    fn map_time(&self, timestamp: u64) -> usize {
        let elapsed = timestamp.saturating_sub(self.genesis_time);
        let bucket = elapsed / self.time_bucket_secs;
        (bucket as usize) % self.size
    }

    /// Hash a string to an axis position [0, size).
    ///
    /// Uses SHA-256 for cross-platform, cross-version determinism.
    /// `DefaultHasher` is explicitly non-stable across Rust releases,
    /// which would break coordinate agreement between nodes.
    fn hash_to_axis(&self, input: &str) -> usize {
        let hash = Sha256::digest(input.as_bytes());
        // Take first 8 bytes as little-endian u64
        let val = u64::from_le_bytes(hash[..8].try_into().unwrap());
        (val as usize) % self.size
    }
}

// --- Cryptographic Identity Binding ---

/// Derive a deterministic org identifier from a public key.
/// org = hex(SHA-256(public_key_bytes)[..8]) — 16 hex chars.
/// This is the ONLY way to produce a valid org for `SignedEvent`.
pub fn org_from_public_key(public_key: &VerifyingKey) -> String {
    let hash = Sha256::digest(public_key.as_bytes());
    hex::encode(&hash[..8])
}

/// A cryptographically signed event. The `org` field is derived from
/// the signer's public key — it cannot be spoofed.
///
/// To create: use `SignedEvent::sign(event_data, &signing_key)`.
/// To verify: call `signed_event.verify()` — returns `Ok(Event)` with
/// the org field guaranteed to match the signer's identity.
#[derive(Clone, Debug)]
pub struct SignedEvent {
    /// Event id.
    pub id: String,
    /// Timestamp (unix seconds).
    pub timestamp: u64,
    /// Channel or context.
    pub channel: String,
    /// Payload data.
    pub data: String,
    /// Ed25519 public key of the signer.
    pub public_key: VerifyingKey,
    /// Ed25519 signature over (id || timestamp || channel || data).
    pub signature: Signature,
}

impl SignedEvent {
    /// Sign event data with a private key. The org is derived automatically.
    pub fn sign(
        id: impl Into<String>,
        timestamp: u64,
        channel: impl Into<String>,
        data: impl Into<String>,
        signing_key: &SigningKey,
    ) -> Self {
        let id = id.into();
        let channel = channel.into();
        let data = data.into();

        let message = Self::signing_message(&id, timestamp, &channel, &data);
        let signature = signing_key.sign(&message);
        let public_key = signing_key.verifying_key();

        Self {
            id,
            timestamp,
            channel,
            data,
            public_key,
            signature,
        }
    }

    /// Verify the signature and produce a trusted `Event` with org derived from the public key.
    /// If verification fails, the event is rejected — no field impact.
    pub fn verify(&self) -> Result<Event, &'static str> {
        let message = Self::signing_message(&self.id, self.timestamp, &self.channel, &self.data);
        self.public_key
            .verify(&message, &self.signature)
            .map_err(|_| "invalid signature")?;

        Ok(Event {
            id: self.id.clone(),
            timestamp: self.timestamp,
            channel: self.channel.clone(),
            org: org_from_public_key(&self.public_key),
            data: self.data.clone(),
        })
    }

    /// The canonical message bytes that get signed.
    fn signing_message(id: &str, timestamp: u64, channel: &str, data: &str) -> Vec<u8> {
        let mut msg = Vec::new();
        msg.extend_from_slice(id.as_bytes());
        msg.extend_from_slice(&timestamp.to_le_bytes());
        msg.extend_from_slice(channel.as_bytes());
        msg.extend_from_slice(data.as_bytes());
        msg
    }

    /// The org identifier derived from this event's signer.
    pub fn org(&self) -> String {
        org_from_public_key(&self.public_key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_mapping() {
        let mapper = CoordMapper::new(32);
        let event = Event {
            id: "tx-001".into(),
            timestamp: 1000,
            channel: "payments".into(),
            org: "alice-corp".into(),
            data: "10 tokens to bob".into(),
        };
        let c1 = mapper.map(&event);
        let c2 = mapper.map(&event);
        assert_eq!(c1, c2, "Same event must produce same coordinate");
    }

    #[test]
    fn same_channel_same_c_axis() {
        let mapper = CoordMapper::new(32);
        let e1 = Event {
            id: "tx-001".into(),
            timestamp: 1000,
            channel: "payments".into(),
            org: "alice".into(),
            data: "".into(),
        };
        let e2 = Event {
            id: "tx-002".into(),
            timestamp: 1005,
            channel: "payments".into(),
            org: "bob".into(),
            data: "".into(),
        };
        assert_eq!(
            mapper.map(&e1).c,
            mapper.map(&e2).c,
            "Same channel → same c-axis"
        );
    }

    #[test]
    fn same_org_same_o_axis() {
        let mapper = CoordMapper::new(32);
        let e1 = Event {
            id: "tx-001".into(),
            timestamp: 1000,
            channel: "payments".into(),
            org: "alice".into(),
            data: "".into(),
        };
        let e2 = Event {
            id: "tx-002".into(),
            timestamp: 2000,
            channel: "identity".into(),
            org: "alice".into(),
            data: "".into(),
        };
        assert_eq!(
            mapper.map(&e1).o,
            mapper.map(&e2).o,
            "Same org → same o-axis"
        );
    }

    #[test]
    fn nearby_timestamps_same_t() {
        let mapper = CoordMapper::new(32).with_time_bucket(60);
        let e1 = Event {
            id: "tx-001".into(),
            timestamp: 1020,
            channel: "x".into(),
            org: "y".into(),
            data: "".into(),
        };
        let e2 = Event {
            id: "tx-002".into(),
            timestamp: 1050,
            channel: "x".into(),
            org: "y".into(),
            data: "".into(),
        };
        // Both in bucket 17 (1020/60=17, 1050/60=17)
        assert_eq!(
            mapper.map(&e1).t,
            mapper.map(&e2).t,
            "Same 60s bucket → same t"
        );
    }

    #[test]
    fn different_buckets_different_t() {
        let mapper = CoordMapper::new(32).with_time_bucket(60);
        let e1 = Event {
            id: "tx-001".into(),
            timestamp: 1020,
            channel: "x".into(),
            org: "y".into(),
            data: "".into(),
        };
        let e2 = Event {
            id: "tx-002".into(),
            timestamp: 1080,
            channel: "x".into(),
            org: "y".into(),
            data: "".into(),
        };
        // Could still collide by wrapping, but likely different
        let c1 = mapper.map(&e1);
        let c2 = mapper.map(&e2);
        // At least t should differ (different buckets)
        assert_ne!(c1.t, c2.t, "61s apart in 60s bucket → different t");
    }

    // --- SignedEvent tests ---

    #[test]
    fn signed_event_verifies() {
        let key = SigningKey::generate(&mut rand_core::OsRng);
        let se = SignedEvent::sign("tx-001", 1000, "payments", "10 tokens", &key);
        let event = se.verify().expect("valid signature should verify");
        assert_eq!(event.id, "tx-001");
        assert_eq!(event.org, org_from_public_key(&key.verifying_key()));
    }

    #[test]
    fn tampered_event_rejected() {
        let key = SigningKey::generate(&mut rand_core::OsRng);
        let mut se = SignedEvent::sign("tx-001", 1000, "payments", "10 tokens", &key);
        se.data = "999 tokens".into(); // tamper
        assert!(se.verify().is_err(), "tampered event must be rejected");
    }

    #[test]
    fn different_keys_different_org() {
        let key1 = SigningKey::generate(&mut rand_core::OsRng);
        let key2 = SigningKey::generate(&mut rand_core::OsRng);
        let org1 = org_from_public_key(&key1.verifying_key());
        let org2 = org_from_public_key(&key2.verifying_key());
        assert_ne!(org1, org2, "different keys must produce different orgs");
    }

    #[test]
    fn same_key_same_org() {
        let key = SigningKey::generate(&mut rand_core::OsRng);
        let org1 = org_from_public_key(&key.verifying_key());
        let org2 = org_from_public_key(&key.verifying_key());
        assert_eq!(org1, org2, "same key must always produce same org");
    }

    #[test]
    fn spoofed_key_wrong_org() {
        let real_key = SigningKey::generate(&mut rand_core::OsRng);
        let attacker_key = SigningKey::generate(&mut rand_core::OsRng);

        let se = SignedEvent::sign("tx-001", 1000, "payments", "steal", &attacker_key);
        let event = se.verify().unwrap();

        let real_org = org_from_public_key(&real_key.verifying_key());
        assert_ne!(
            event.org, real_org,
            "attacker cannot claim real identity's org"
        );
    }

    #[test]
    fn signed_event_maps_deterministically() {
        let key = SigningKey::generate(&mut rand_core::OsRng);
        let mapper = CoordMapper::new(32);

        let se = SignedEvent::sign("tx-001", 1000, "payments", "data", &key);
        let event = se.verify().unwrap();
        let c1 = mapper.map(&event);
        let c2 = mapper.map(&event);
        assert_eq!(c1, c2, "verified event must map deterministically");
    }

    #[test]
    fn semantic_proximity() {
        // Two transactions in the same channel, same org, close in time
        // should land in nearby coordinates (only t and v differ)
        let mapper = CoordMapper::new(32).with_time_bucket(60);
        let e1 = Event {
            id: "tx-001".into(),
            timestamp: 1020,
            channel: "payments".into(),
            org: "alice".into(),
            data: "send 10".into(),
        };
        let e2 = Event {
            id: "tx-002".into(),
            timestamp: 1050,
            channel: "payments".into(),
            org: "alice".into(),
            data: "send 5".into(),
        };

        let c1 = mapper.map(&e1);
        let c2 = mapper.map(&e2);

        assert_eq!(c1.t, c2.t, "Same time bucket");
        assert_eq!(c1.c, c2.c, "Same channel");
        assert_eq!(c1.o, c2.o, "Same org");
        // v differs (different event id) — but they share 3 of 4 axes
        // Their orbitals WILL overlap → emergent connection
    }
}
