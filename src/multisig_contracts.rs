/**
 * Multi-Signature Contracts Module
 *
 * Require multiple signatures for critical operations:
 * - Signature threshold (e.g., 2-of-3, 3-of-5)
 * - Pending operations queue
 * - Timeout for unsigned operations
 */

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Operation that requires signatures
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MultiSigOperation {
    Transfer { to: String, amount: u64 },
    ConfigUpdate { key: String, value: String },
    MemberAdd { address: String },
    MemberRemove { address: String },
    ThresholdChange { new_threshold: u8 },
    Custom { op_type: String, params: Vec<String> },
}

/// Signature record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signature {
    pub signer: String,
    pub signed_at: u64,
    pub signature_hash: String,
}

/// Pending operation awaiting signatures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingOperation {
    pub id: u64,
    pub operation: MultiSigOperation,
    pub created_block: u64,
    pub timeout_blocks: u64,
    pub signatures: HashMap<String, Signature>,
    pub executed: bool,
}

impl PendingOperation {
    pub fn new(id: u64, operation: MultiSigOperation, created_block: u64, timeout_blocks: u64) -> Self {
        PendingOperation {
            id,
            operation,
            created_block,
            timeout_blocks,
            signatures: HashMap::new(),
            executed: false,
        }
    }

    /// Check if operation has expired
    pub fn is_expired(&self, current_block: u64) -> bool {
        current_block > self.created_block + self.timeout_blocks
    }

    /// Check if operation has enough signatures
    pub fn is_approved(&self, threshold: u8) -> bool {
        self.signatures.len() >= threshold as usize
    }

    /// Add a signature
    pub fn add_signature(&mut self, signer: String, signature_hash: String, signed_at: u64) -> Result<(), String> {
        if self.signatures.contains_key(&signer) {
            return Err("Signer already signed this operation".to_string());
        }

        self.signatures.insert(
            signer.clone(),
            Signature {
                signer,
                signed_at,
                signature_hash,
            },
        );

        Ok(())
    }

    /// Get number of signatures
    pub fn signature_count(&self) -> u8 {
        self.signatures.len() as u8
    }
}

/// Multi-Signature Contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiSigContract {
    pub contract_address: String,
    pub owner: String,
    pub members: Vec<String>,
    pub signature_threshold: u8,
    pub pending_operations: HashMap<u64, PendingOperation>,
    pub next_operation_id: u64,
    pub timeout_blocks: u64,
    pub executed_count: u64,
}

impl MultiSigContract {
    pub fn new(
        contract_address: String,
        owner: String,
        members: Vec<String>,
        signature_threshold: u8,
        timeout_blocks: u64,
    ) -> Result<Self, String> {
        if members.is_empty() {
            return Err("Must have at least one member".to_string());
        }

        if signature_threshold == 0 || signature_threshold as usize > members.len() {
            return Err(format!(
                "Threshold {} must be between 1 and {}",
                signature_threshold,
                members.len()
            ));
        }

        Ok(MultiSigContract {
            contract_address,
            owner,
            members,
            signature_threshold,
            pending_operations: HashMap::new(),
            next_operation_id: 1,
            timeout_blocks,
            executed_count: 0,
        })
    }

    /// Propose an operation (only owner)
    pub fn propose_operation(
        &mut self,
        caller: &str,
        operation: MultiSigOperation,
        current_block: u64,
    ) -> Result<u64, String> {
        if caller != self.owner {
            return Err("Only owner can propose operations".to_string());
        }

        let op_id = self.next_operation_id;
        let pending = PendingOperation::new(op_id, operation, current_block, self.timeout_blocks);
        self.pending_operations.insert(op_id, pending);
        self.next_operation_id += 1;

        Ok(op_id)
    }

    /// Sign an operation
    pub fn sign_operation(
        &mut self,
        operation_id: u64,
        signer: String,
        signature_hash: String,
        current_block: u64,
    ) -> Result<(), String> {
        if !self.members.contains(&signer) {
            return Err("Signer is not a member of this multi-sig".to_string());
        }

        let operation = self
            .pending_operations
            .get_mut(&operation_id)
            .ok_or("Operation not found")?;

        if operation.executed {
            return Err("Operation already executed".to_string());
        }

        if operation.is_expired(current_block) {
            return Err("Operation has expired".to_string());
        }

        operation.add_signature(signer, signature_hash, current_block)?;

        Ok(())
    }

    /// Execute an operation if it has enough signatures
    pub fn execute_operation(&mut self, operation_id: u64, current_block: u64) -> Result<String, String> {
        let operation = self
            .pending_operations
            .get_mut(&operation_id)
            .ok_or("Operation not found")?;

        if operation.executed {
            return Err("Operation already executed".to_string());
        }

        if operation.is_expired(current_block) {
            return Err("Operation has expired".to_string());
        }

        if !operation.is_approved(self.signature_threshold) {
            return Err(format!(
                "Not enough signatures: {} / {}",
                operation.signatures.len(),
                self.signature_threshold
            ));
        }

        operation.executed = true;
        self.executed_count += 1;

        let result = match &operation.operation {
            MultiSigOperation::Transfer { to, amount } => {
                format!("Transfer executed: {} tokens to {}", amount, to)
            }
            MultiSigOperation::ConfigUpdate { key, value } => {
                format!("Config updated: {} = {}", key, value)
            }
            MultiSigOperation::MemberAdd { address } => {
                format!("Member added: {}", address)
            }
            MultiSigOperation::MemberRemove { address } => {
                format!("Member removed: {}", address)
            }
            MultiSigOperation::ThresholdChange { new_threshold } => {
                format!("Threshold changed to {}", new_threshold)
            }
            MultiSigOperation::Custom { op_type, params } => {
                format!("Custom operation executed: {} with params: {}", op_type, params.join(", "))
            }
        };

        Ok(result)
    }

    /// Revoke a signature
    pub fn revoke_signature(&mut self, operation_id: u64, signer: &str) -> Result<(), String> {
        let operation = self
            .pending_operations
            .get_mut(&operation_id)
            .ok_or("Operation not found")?;

        if operation.executed {
            return Err("Cannot revoke signature on executed operation".to_string());
        }

        if operation.signatures.remove(signer).is_none() {
            return Err("Signer has not signed this operation".to_string());
        }

        Ok(())
    }

    /// Add a member (owner only)
    pub fn add_member(&mut self, caller: &str, new_member: String) -> Result<(), String> {
        if caller != self.owner {
            return Err("Only owner can add members".to_string());
        }

        if self.members.contains(&new_member) {
            return Err("Member already exists".to_string());
        }

        self.members.push(new_member);
        Ok(())
    }

    /// Remove a member (owner only)
    pub fn remove_member(&mut self, caller: &str, member: &str) -> Result<(), String> {
        if caller != self.owner {
            return Err("Only owner can remove members".to_string());
        }

        if member == self.owner {
            return Err("Cannot remove owner from members".to_string());
        }

        let member_str = member.to_string();
        if let Some(pos) = self.members.iter().position(|x| *x == member_str) {
            self.members.remove(pos);
        } else {
            return Err("Member not found".to_string());
        }

        // Adjust threshold if needed
        if self.signature_threshold as usize > self.members.len() {
            self.signature_threshold = self.members.len() as u8;
        }

        Ok(())
    }

    /// Change signature threshold (owner only)
    pub fn change_threshold(&mut self, caller: &str, new_threshold: u8) -> Result<(), String> {
        if caller != self.owner {
            return Err("Only owner can change threshold".to_string());
        }

        if new_threshold == 0 || new_threshold as usize > self.members.len() {
            return Err(format!(
                "Threshold {} must be between 1 and {}",
                new_threshold,
                self.members.len()
            ));
        }

        self.signature_threshold = new_threshold;
        Ok(())
    }

    /// Get operation details
    pub fn get_operation(&self, operation_id: u64) -> Option<PendingOperation> {
        self.pending_operations.get(&operation_id).cloned()
    }

    /// Get all pending operations
    pub fn get_pending_operations(&self) -> Vec<PendingOperation> {
        self.pending_operations
            .values()
            .filter(|op| !op.executed)
            .cloned()
            .collect()
    }

    /// Cleanup expired operations
    pub fn cleanup_expired(&mut self, current_block: u64) -> u64 {
        let mut removed_count = 0;
        let expired_ids: Vec<u64> = self
            .pending_operations
            .iter()
            .filter(|(_, op)| op.is_expired(current_block) && !op.executed)
            .map(|(id, _)| *id)
            .collect();

        for id in expired_ids {
            self.pending_operations.remove(&id);
            removed_count += 1;
        }

        removed_count
    }

    /// Get member list
    pub fn get_members(&self) -> Vec<String> {
        self.members.clone()
    }

    /// Check if address is a member
    pub fn is_member(&self, address: &str) -> bool {
        self.members.contains(&address.to_string())
    }

    /// Get statistics
    pub fn get_statistics(&self) -> MultiSigStatistics {
        let total_operations = self.next_operation_id - 1;
        let pending_count = self.pending_operations.iter().filter(|(_, op)| !op.executed).count() as u64;

        MultiSigStatistics {
            members: self.members.len() as u64,
            threshold: self.signature_threshold,
            total_operations,
            executed_operations: self.executed_count,
            pending_operations: pending_count,
        }
    }
}

/// Multi-Signature statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiSigStatistics {
    pub members: u64,
    pub threshold: u8,
    pub total_operations: u64,
    pub executed_operations: u64,
    pub pending_operations: u64,
}
