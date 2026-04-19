//! Coordinate mapper: real-world events → 4D coordinates.
//!
//! Maps transaction data to deterministic positions in the tesseract field.
//! Related events land near each other → orbitals overlap → emergent links.

use crate::Coord;
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
/// **Identity binding (production consideration):**
/// The `org` field in [`Event`] is currently an unverified string.
/// In production, `org` MUST be derived from a verified public key
/// (e.g., `SHA-256(pub_key) % size`) to prevent identity spoofing.
/// The parent project's `src/identity/` module provides Ed25519 and
/// ML-DSA-65 signing providers suitable for this purpose.
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
        let e1 = Event { id: "tx-001".into(), timestamp: 1000, channel: "payments".into(), org: "alice".into(), data: "".into() };
        let e2 = Event { id: "tx-002".into(), timestamp: 1005, channel: "payments".into(), org: "bob".into(), data: "".into() };
        assert_eq!(mapper.map(&e1).c, mapper.map(&e2).c, "Same channel → same c-axis");
    }

    #[test]
    fn same_org_same_o_axis() {
        let mapper = CoordMapper::new(32);
        let e1 = Event { id: "tx-001".into(), timestamp: 1000, channel: "payments".into(), org: "alice".into(), data: "".into() };
        let e2 = Event { id: "tx-002".into(), timestamp: 2000, channel: "identity".into(), org: "alice".into(), data: "".into() };
        assert_eq!(mapper.map(&e1).o, mapper.map(&e2).o, "Same org → same o-axis");
    }

    #[test]
    fn nearby_timestamps_same_t() {
        let mapper = CoordMapper::new(32).with_time_bucket(60);
        let e1 = Event { id: "tx-001".into(), timestamp: 1020, channel: "x".into(), org: "y".into(), data: "".into() };
        let e2 = Event { id: "tx-002".into(), timestamp: 1050, channel: "x".into(), org: "y".into(), data: "".into() };
        // Both in bucket 17 (1020/60=17, 1050/60=17)
        assert_eq!(mapper.map(&e1).t, mapper.map(&e2).t, "Same 60s bucket → same t");
    }

    #[test]
    fn different_buckets_different_t() {
        let mapper = CoordMapper::new(32).with_time_bucket(60);
        let e1 = Event { id: "tx-001".into(), timestamp: 1020, channel: "x".into(), org: "y".into(), data: "".into() };
        let e2 = Event { id: "tx-002".into(), timestamp: 1080, channel: "x".into(), org: "y".into(), data: "".into() };
        // Could still collide by wrapping, but likely different
        let c1 = mapper.map(&e1);
        let c2 = mapper.map(&e2);
        // At least t should differ (different buckets)
        assert_ne!(c1.t, c2.t, "61s apart in 60s bucket → different t");
    }

    #[test]
    fn semantic_proximity() {
        // Two transactions in the same channel, same org, close in time
        // should land in nearby coordinates (only t and v differ)
        let mapper = CoordMapper::new(32).with_time_bucket(60);
        let e1 = Event { id: "tx-001".into(), timestamp: 1020, channel: "payments".into(), org: "alice".into(), data: "send 10".into() };
        let e2 = Event { id: "tx-002".into(), timestamp: 1050, channel: "payments".into(), org: "alice".into(), data: "send 5".into() };

        let c1 = mapper.map(&e1);
        let c2 = mapper.map(&e2);

        assert_eq!(c1.t, c2.t, "Same time bucket");
        assert_eq!(c1.c, c2.c, "Same channel");
        assert_eq!(c1.o, c2.o, "Same org");
        // v differs (different event id) — but they share 3 of 4 axes
        // Their orbitals WILL overlap → emergent connection
    }
}
