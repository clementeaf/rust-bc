use serde::{Deserialize, Serialize};

/// A versioned read of a key from the ledger state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KVRead {
    pub key: String,
    pub version: u64,
}

/// A pending write of a key-value pair to the ledger state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KVWrite {
    pub key: String,
    pub value: Vec<u8>,
}

/// The read-write set produced during transaction simulation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ReadWriteSet {
    pub reads: Vec<KVRead>,
    pub writes: Vec<KVWrite>,
}

impl ReadWriteSet {
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.reads.is_empty() && self.writes.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kv_read_serde_roundtrip() {
        let r = KVRead {
            key: "foo".to_string(),
            version: 7,
        };
        let json = serde_json::to_string(&r).unwrap();
        let back: KVRead = serde_json::from_str(&json).unwrap();
        assert_eq!(r, back);
    }

    #[test]
    fn kv_write_serde_roundtrip() {
        let w = KVWrite {
            key: "bar".to_string(),
            value: b"hello".to_vec(),
        };
        let json = serde_json::to_string(&w).unwrap();
        let back: KVWrite = serde_json::from_str(&json).unwrap();
        assert_eq!(w, back);
    }

    #[test]
    fn empty_rwset_is_empty() {
        let rw = ReadWriteSet::default();
        assert!(rw.is_empty());
    }

    #[test]
    fn non_empty_rwset_is_not_empty() {
        let rw = ReadWriteSet {
            reads: vec![KVRead {
                key: "k".to_string(),
                version: 1,
            }],
            writes: vec![],
        };
        assert!(!rw.is_empty());
    }

    #[test]
    fn rwset_with_only_write_is_not_empty() {
        let rw = ReadWriteSet {
            reads: vec![],
            writes: vec![KVWrite {
                key: "k".to_string(),
                value: vec![1, 2, 3],
            }],
        };
        assert!(!rw.is_empty());
    }

    #[test]
    fn rwset_serde_roundtrip() {
        let rw = ReadWriteSet {
            reads: vec![KVRead {
                key: "a".to_string(),
                version: 3,
            }],
            writes: vec![KVWrite {
                key: "b".to_string(),
                value: b"val".to_vec(),
            }],
        };
        let json = serde_json::to_string(&rw).unwrap();
        let back: ReadWriteSet = serde_json::from_str(&json).unwrap();
        assert_eq!(rw, back);
    }
}
