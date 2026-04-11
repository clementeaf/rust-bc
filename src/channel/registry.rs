//! Channel registry trait and in-memory implementation

use std::collections::HashMap;
use std::sync::Mutex;

use crate::storage::errors::{StorageError, StorageResult};

use super::Channel;

/// Trait for creating and querying channels
pub trait ChannelRegistry: Send + Sync {
    #[allow(dead_code)]
    fn create_channel(&self, channel: &Channel) -> StorageResult<()>;
    #[allow(dead_code)]
    fn get_channel(&self, channel_id: &str) -> StorageResult<Channel>;
    #[allow(dead_code)]
    fn list_channels(&self) -> StorageResult<Vec<Channel>>;
    #[allow(dead_code)]
    fn update_channel(&self, channel: &Channel) -> StorageResult<()>;
}

/// In-memory channel registry backed by a `HashMap`
pub struct MemoryChannelRegistry {
    inner: Mutex<HashMap<String, Channel>>,
}

impl MemoryChannelRegistry {
    pub fn new() -> Self {
        MemoryChannelRegistry {
            inner: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for MemoryChannelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ChannelRegistry for MemoryChannelRegistry {
    fn create_channel(&self, channel: &Channel) -> StorageResult<()> {
        let mut map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if map.contains_key(&channel.channel_id) {
            return Err(StorageError::KeyNotFound(format!(
                "channel '{}' already exists",
                channel.channel_id
            )));
        }
        map.insert(channel.channel_id.clone(), channel.clone());
        Ok(())
    }

    fn get_channel(&self, channel_id: &str) -> StorageResult<Channel> {
        self.inner
            .lock()
            .unwrap()
            .get(channel_id)
            .cloned()
            .ok_or_else(|| StorageError::KeyNotFound(channel_id.to_string()))
    }

    fn list_channels(&self) -> StorageResult<Vec<Channel>> {
        Ok(self
            .inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .values()
            .cloned()
            .collect())
    }

    fn update_channel(&self, channel: &Channel) -> StorageResult<()> {
        let mut map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if !map.contains_key(&channel.channel_id) {
            return Err(StorageError::KeyNotFound(channel.channel_id.clone()));
        }
        map.insert(channel.channel_id.clone(), channel.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::endorsement::policy::EndorsementPolicy;

    fn sample_channel(id: &str) -> Channel {
        Channel {
            channel_id: id.to_string(),
            member_org_ids: vec!["org1".to_string()],
            orderer_org_ids: vec!["orderer1".to_string()],
            created_at: 1_000_000,
            endorsement_policy: EndorsementPolicy::AnyOf(vec!["org1".to_string()]),
        }
    }

    #[test]
    fn create_and_get() {
        let reg = MemoryChannelRegistry::new();
        let ch = sample_channel("ch1");
        reg.create_channel(&ch).unwrap();
        let retrieved = reg.get_channel("ch1").unwrap();
        assert_eq!(retrieved.channel_id, "ch1");
    }

    #[test]
    fn create_duplicate_returns_error() {
        let reg = MemoryChannelRegistry::new();
        let ch = sample_channel("ch1");
        reg.create_channel(&ch).unwrap();
        assert!(reg.create_channel(&ch).is_err());
    }

    #[test]
    fn list_channels() {
        let reg = MemoryChannelRegistry::new();
        reg.create_channel(&sample_channel("ch1")).unwrap();
        reg.create_channel(&sample_channel("ch2")).unwrap();
        let channels = reg.list_channels().unwrap();
        assert_eq!(channels.len(), 2);
    }

    #[test]
    fn update_channel() {
        let reg = MemoryChannelRegistry::new();
        let ch = sample_channel("ch1");
        reg.create_channel(&ch).unwrap();

        let mut updated = ch.clone();
        updated.member_org_ids.push("org2".to_string());
        reg.update_channel(&updated).unwrap();

        let retrieved = reg.get_channel("ch1").unwrap();
        assert!(retrieved.is_member("org2"));
    }

    #[test]
    fn channel_not_found() {
        let reg = MemoryChannelRegistry::new();
        let result = reg.get_channel("nonexistent");
        assert!(matches!(result, Err(StorageError::KeyNotFound(_))));
    }

    #[test]
    fn update_nonexistent_returns_error() {
        let reg = MemoryChannelRegistry::new();
        let ch = sample_channel("ch1");
        assert!(reg.update_channel(&ch).is_err());
    }
}
