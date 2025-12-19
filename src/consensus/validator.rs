//! Block validator for DAG consensus
//!
//! Implements block validity checks for the consensus layer.

use crate::consensus::dag::DagBlock;
use crate::consensus::scheduler::SlotScheduler;

/// Result of block validation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidityResult {
    /// Block is valid
    Valid,
    /// Block is invalid with reason
    Invalid(String),
}

/// Block validator
pub struct BlockValidator;

impl BlockValidator {
    /// Validate a block's format
    pub fn validate_format(block: &DagBlock) -> ValidityResult {
        if block.hash == [0u8; 32] {
            return ValidityResult::Invalid("Block hash cannot be zero".to_string());
        }

        if block.proposer.is_empty() {
            return ValidityResult::Invalid("Proposer cannot be empty".to_string());
        }

        if block.proposer.len() > 256 {
            return ValidityResult::Invalid("Proposer name too long".to_string());
        }

        ValidityResult::Valid
    }

    /// Validate block signature format (basic check, real verification deferred)
    pub fn validate_signature(block: &DagBlock) -> ValidityResult {
        if block.signature == [0u8; 64] {
            return ValidityResult::Invalid("Signature cannot be zero".to_string());
        }

        ValidityResult::Valid
    }

    /// Validate parent hash exists (for non-genesis blocks)
    pub fn validate_parent(block: &DagBlock) -> ValidityResult {
        if block.is_genesis() {
            if block.parent_hash != [0u8; 32] {
                return ValidityResult::Invalid("Genesis block must have zero parent".to_string());
            }
        } else {
            if block.parent_hash == [0u8; 32] {
                return ValidityResult::Invalid("Non-genesis block cannot have zero parent".to_string());
            }
        }

        ValidityResult::Valid
    }

    /// Validate slot assignment
    pub fn validate_slot(block: &DagBlock, scheduler: &SlotScheduler) -> ValidityResult {
        if !scheduler.validate_block_slot(block.slot, block.timestamp) {
            return ValidityResult::Invalid(format!(
                "Block timestamp {} not within slot {} bounds",
                block.timestamp, block.slot
            ));
        }

        let expected_proposer = scheduler.get_proposer(block.slot);
        if block.proposer != expected_proposer {
            return ValidityResult::Invalid(format!(
                "Expected proposer {} for slot {}, got {}",
                expected_proposer, block.slot, block.proposer
            ));
        }

        ValidityResult::Valid
    }

    /// Validate block height (must be >= parent height)
    pub fn validate_height(block: &DagBlock, parent_height: u64) -> ValidityResult {
        if block.height <= parent_height && !block.is_genesis() {
            return ValidityResult::Invalid(format!(
                "Block height {} must be > parent height {}",
                block.height, parent_height
            ));
        }

        ValidityResult::Valid
    }

    /// Full block validation
    pub fn validate(block: &DagBlock, scheduler: &SlotScheduler) -> ValidityResult {
        // Format check
        if let ValidityResult::Invalid(e) = Self::validate_format(block) {
            return ValidityResult::Invalid(e);
        }

        // Signature check
        if let ValidityResult::Invalid(e) = Self::validate_signature(block) {
            return ValidityResult::Invalid(e);
        }

        // Parent check
        if let ValidityResult::Invalid(e) = Self::validate_parent(block) {
            return ValidityResult::Invalid(e);
        }

        // Slot check
        if let ValidityResult::Invalid(e) = Self::validate_slot(block, scheduler) {
            return ValidityResult::Invalid(e);
        }

        ValidityResult::Valid
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_valid_block() -> DagBlock {
        DagBlock::new(
            [1u8; 32],
            [0u8; 32],
            0,
            0,
            1000,
            "validator1".to_string(),
            [2u8; 64],
        )
    }

    fn create_test_scheduler() -> SlotScheduler {
        let validators = vec!["validator1".to_string()];
        SlotScheduler::new(1, validators, 1000)
    }

    #[test]
    fn test_validate_format_valid() {
        let block = create_valid_block();
        assert_eq!(BlockValidator::validate_format(&block), ValidityResult::Valid);
    }

    #[test]
    fn test_validate_format_zero_hash() {
        let mut block = create_valid_block();
        block.hash = [0u8; 32];
        assert!(matches!(BlockValidator::validate_format(&block), ValidityResult::Invalid(_)));
    }

    #[test]
    fn test_validate_format_empty_proposer() {
        let mut block = create_valid_block();
        block.proposer = "".to_string();
        assert!(matches!(BlockValidator::validate_format(&block), ValidityResult::Invalid(_)));
    }

    #[test]
    fn test_validate_signature_valid() {
        let block = create_valid_block();
        assert_eq!(BlockValidator::validate_signature(&block), ValidityResult::Valid);
    }

    #[test]
    fn test_validate_signature_zero() {
        let mut block = create_valid_block();
        block.signature = [0u8; 64];
        assert!(matches!(BlockValidator::validate_signature(&block), ValidityResult::Invalid(_)));
    }

    #[test]
    fn test_validate_parent_genesis() {
        let block = create_valid_block();
        assert_eq!(BlockValidator::validate_parent(&block), ValidityResult::Valid);
    }

    #[test]
    fn test_validate_parent_non_genesis_invalid() {
        let mut block = create_valid_block();
        block.parent_hash = [0u8; 32];
        block.height = 1;
        assert!(matches!(BlockValidator::validate_parent(&block), ValidityResult::Invalid(_)));
    }

    #[test]
    fn test_validate_slot_valid() {
        let block = create_valid_block();
        let scheduler = create_test_scheduler();
        assert_eq!(BlockValidator::validate_slot(&block, &scheduler), ValidityResult::Valid);
    }

    #[test]
    fn test_validate_slot_wrong_proposer() {
        let mut block = create_valid_block();
        block.proposer = "validator2".to_string();
        let scheduler = create_test_scheduler();
        assert!(matches!(BlockValidator::validate_slot(&block, &scheduler), ValidityResult::Invalid(_)));
    }

    #[test]
    fn test_validate_height() {
        let block = create_valid_block();
        assert_eq!(BlockValidator::validate_height(&block, 0), ValidityResult::Valid);
    }

    #[test]
    fn test_full_validation() {
        let block = create_valid_block();
        let scheduler = create_test_scheduler();
        assert_eq!(BlockValidator::validate(&block, &scheduler), ValidityResult::Valid);
    }
}
