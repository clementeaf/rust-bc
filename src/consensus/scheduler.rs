//! Slot scheduler for DAG consensus
//!
//! Implements fixed-duration slots with deterministic proposer assignment.

use std::collections::HashMap;

/// Represents a consensus slot
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slot {
    /// Slot number (0-indexed)
    pub number: u64,
    /// Proposer for this slot (deterministically assigned)
    pub proposer: String,
    /// Slot start time (UNIX timestamp)
    pub start_time: u64,
    /// Slot end time (UNIX timestamp)
    pub end_time: u64,
}

impl Slot {
    /// Create a new slot
    pub fn new(number: u64, proposer: String, start_time: u64, end_time: u64) -> Self {
        Slot {
            number,
            proposer,
            start_time,
            end_time,
        }
    }

    /// Check if a given timestamp is within this slot
    pub fn contains_timestamp(&self, timestamp: u64) -> bool {
        timestamp >= self.start_time && timestamp < self.end_time
    }

    /// Get slot duration in seconds
    pub fn duration(&self) -> u64 {
        self.end_time - self.start_time
    }
}

/// Slot scheduler with deterministic proposer assignment
pub struct SlotScheduler {
    /// Duration of each slot in seconds
    slot_duration: u64,
    /// List of validators (proposers)
    validators: Vec<String>,
    /// Cache of slots
    slot_cache: HashMap<u64, Slot>,
    /// Genesis time (start of slot 0)
    genesis_time: u64,
}

impl SlotScheduler {
    /// Create a new slot scheduler
    pub fn new(slot_duration: u64, validators: Vec<String>, genesis_time: u64) -> Self {
        SlotScheduler {
            slot_duration,
            validators,
            slot_cache: HashMap::new(),
            genesis_time,
        }
    }

    /// Get the proposer for a given slot using deterministic assignment
    pub fn get_proposer(&self, slot_number: u64) -> String {
        if self.validators.is_empty() {
            return "unknown".to_string();
        }
        let index = (slot_number as usize) % self.validators.len();
        self.validators[index].clone()
    }

    /// Get or create a slot
    pub fn get_slot(&mut self, slot_number: u64) -> Slot {
        if let Some(slot) = self.slot_cache.get(&slot_number) {
            return slot.clone();
        }

        let start_time = self.genesis_time + (slot_number * self.slot_duration);
        let end_time = start_time + self.slot_duration;
        let proposer = self.get_proposer(slot_number);

        let slot = Slot::new(slot_number, proposer, start_time, end_time);
        self.slot_cache.insert(slot_number, slot.clone());
        slot
    }

    /// Get current slot number for a given timestamp
    pub fn get_current_slot(&self, timestamp: u64) -> u64 {
        if timestamp < self.genesis_time {
            return 0;
        }
        (timestamp - self.genesis_time) / self.slot_duration
    }

    /// Get slot number from timestamp
    pub fn timestamp_to_slot(&self, timestamp: u64) -> u64 {
        self.get_current_slot(timestamp)
    }

    /// Get timestamp range for a slot
    pub fn slot_to_timestamps(&self, slot_number: u64) -> (u64, u64) {
        let start = self.genesis_time + (slot_number * self.slot_duration);
        let end = start + self.slot_duration;
        (start, end)
    }

    /// Check if a block timestamp is valid for its slot
    pub fn validate_block_slot(&self, slot_number: u64, timestamp: u64) -> bool {
        let (start, end) = self.slot_to_timestamps(slot_number);
        timestamp >= start && timestamp < end
    }

    /// Get all validators
    pub fn validators(&self) -> &[String] {
        &self.validators
    }

    /// Get genesis time
    pub fn genesis_time(&self) -> u64 {
        self.genesis_time
    }

    /// Get slot duration
    pub fn slot_duration(&self) -> u64 {
        self.slot_duration
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_scheduler() -> SlotScheduler {
        let validators = vec![
            "validator1".to_string(),
            "validator2".to_string(),
            "validator3".to_string(),
        ];
        SlotScheduler::new(1, validators, 1000)
    }

    #[test]
    fn test_slot_creation() {
        let slot = Slot::new(0, "proposer".to_string(), 1000, 2000);
        assert_eq!(slot.number, 0);
        assert_eq!(slot.proposer, "proposer");
    }

    #[test]
    fn test_slot_contains_timestamp() {
        let slot = Slot::new(0, "p".to_string(), 1000, 2000);
        assert!(slot.contains_timestamp(1500));
        assert!(!slot.contains_timestamp(500));
        assert!(!slot.contains_timestamp(2500));
    }

    #[test]
    fn test_slot_duration() {
        let slot = Slot::new(0, "p".to_string(), 1000, 2000);
        assert_eq!(slot.duration(), 1000);
    }

    #[test]
    fn test_scheduler_creation() {
        let scheduler = create_test_scheduler();
        assert_eq!(scheduler.validators().len(), 3);
        assert_eq!(scheduler.slot_duration(), 1);
        assert_eq!(scheduler.genesis_time(), 1000);
    }

    #[test]
    fn test_deterministic_proposer_assignment() {
        let scheduler = create_test_scheduler();
        assert_eq!(scheduler.get_proposer(0), "validator1");
        assert_eq!(scheduler.get_proposer(1), "validator2");
        assert_eq!(scheduler.get_proposer(2), "validator3");
        assert_eq!(scheduler.get_proposer(3), "validator1"); // Wraps around
    }

    #[test]
    fn test_get_current_slot() {
        let scheduler = create_test_scheduler();
        assert_eq!(scheduler.get_current_slot(1000), 0);
        assert_eq!(scheduler.get_current_slot(1001), 1);
        assert_eq!(scheduler.get_current_slot(1002), 2);
    }

    #[test]
    fn test_timestamp_to_slot() {
        let scheduler = create_test_scheduler();
        assert_eq!(scheduler.timestamp_to_slot(1000), 0);
        assert_eq!(scheduler.timestamp_to_slot(1001), 1);
        assert_eq!(scheduler.timestamp_to_slot(1050), 50);
    }

    #[test]
    fn test_slot_to_timestamps() {
        let scheduler = create_test_scheduler();
        let (start, end) = scheduler.slot_to_timestamps(0);
        assert_eq!(start, 1000);
        assert_eq!(end, 1001);

        let (start, end) = scheduler.slot_to_timestamps(5);
        assert_eq!(start, 1005);
        assert_eq!(end, 1006);
    }

    #[test]
    fn test_validate_block_slot() {
        let scheduler = create_test_scheduler();
        assert!(scheduler.validate_block_slot(0, 1000));
        assert!(!scheduler.validate_block_slot(0, 1001));
        assert!(scheduler.validate_block_slot(1, 1001));
    }

    #[test]
    fn test_get_slot() {
        let mut scheduler = create_test_scheduler();
        let slot = scheduler.get_slot(5);
        assert_eq!(slot.number, 5);
        assert_eq!(slot.proposer, "validator2");
    }
}
