//! DAG (Directed Acyclic Graph) consensus structures
//!
//! Implements the core DAG data structures for the consensus layer.
//! Supports block representation, slot-based ordering, and DAG traversal.

use std::collections::HashMap;

/// Block in the DAG consensus
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DagBlock {
    /// Unique block hash (SHA256)
    pub hash: [u8; 32],
    /// Parent block hash (DAG edge)
    pub parent_hash: [u8; 32],
    /// Block height in canonical chain
    pub height: u64,
    /// Slot number this block belongs to
    pub slot: u64,
    /// UNIX timestamp when block was created
    pub timestamp: u64,
    /// Block proposer's identity
    pub proposer: String,
    /// Block signature
    pub signature: [u8; 64],
    /// Transaction hashes included in block
    pub transactions: Vec<[u8; 32]>,
}

impl DagBlock {
    /// Create a new DAG block
    pub fn new(
        hash: [u8; 32],
        parent_hash: [u8; 32],
        height: u64,
        slot: u64,
        timestamp: u64,
        proposer: String,
        signature: [u8; 64],
    ) -> Self {
        DagBlock {
            hash,
            parent_hash,
            height,
            slot,
            timestamp,
            proposer,
            signature,
            transactions: Vec::new(),
        }
    }

    /// Add a transaction to this block
    pub fn add_transaction(&mut self, tx_hash: [u8; 32]) {
        self.transactions.push(tx_hash);
    }

    /// Check if this is a genesis block (parent is all zeros)
    pub fn is_genesis(&self) -> bool {
        self.parent_hash == [0u8; 32]
    }
}

/// DAG vertex representing a block in the consensus graph
#[derive(Debug, Clone)]
pub struct DagVertex {
    /// Block data
    pub block: DagBlock,
    /// Children blocks (vertices that reference this as parent)
    pub children: Vec<[u8; 32]>,
}

impl DagVertex {
    /// Create a new DAG vertex
    pub fn new(block: DagBlock) -> Self {
        DagVertex {
            block,
            children: Vec::new(),
        }
    }

    /// Add a child block hash
    pub fn add_child(&mut self, child_hash: [u8; 32]) {
        if !self.children.contains(&child_hash) {
            self.children.push(child_hash);
        }
    }
}

/// DAG edge representing a parent-child relationship
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DagEdge {
    /// Parent block hash
    pub from: [u8; 32],
    /// Child block hash
    pub to: [u8; 32],
    /// Edge weight (can be used for fork resolution)
    pub weight: u64,
}

impl DagEdge {
    /// Create a new DAG edge
    pub fn new(from: [u8; 32], to: [u8; 32], weight: u64) -> Self {
        DagEdge { from, to, weight }
    }
}

/// DAG representation as a collection of vertices
#[derive(Debug)]
pub struct Dag {
    /// Vertices indexed by block hash
    vertices: HashMap<[u8; 32], DagVertex>,
    /// Head block hash (most recent)
    head: Option<[u8; 32]>,
    /// Total blocks in DAG
    block_count: u64,
}

impl Dag {
    /// Create a new empty DAG
    pub fn new() -> Self {
        Dag {
            vertices: HashMap::new(),
            head: None,
            block_count: 0,
        }
    }

    /// Add a block to the DAG
    pub fn add_block(&mut self, block: DagBlock) -> Result<(), String> {
        // Check for duplicate
        if self.vertices.contains_key(&block.hash) {
            return Err("Block already exists".to_string());
        }

        // Check parent exists (unless genesis)
        if !block.is_genesis() && !self.vertices.contains_key(&block.parent_hash) {
            return Err("Parent block not found".to_string());
        }

        // Create vertex
        let vertex = DagVertex::new(block.clone());
        self.vertices.insert(block.hash, vertex);

        // Update parent's children list
        if !block.is_genesis() {
            if let Some(parent_vertex) = self.vertices.get_mut(&block.parent_hash) {
                parent_vertex.add_child(block.hash);
            }
        }

        // Update head
        self.head = Some(block.hash);
        self.block_count += 1;

        Ok(())
    }

    /// Get a block by hash
    pub fn get_block(&self, hash: &[u8; 32]) -> Option<DagBlock> {
        self.vertices.get(hash).map(|v| v.block.clone())
    }

    /// Get all vertices
    pub fn vertices(&self) -> &HashMap<[u8; 32], DagVertex> {
        &self.vertices
    }

    /// Get DAG head (most recent block)
    pub fn head(&self) -> Option<[u8; 32]> {
        self.head
    }

    /// Get total blocks
    pub fn block_count(&self) -> u64 {
        self.block_count
    }

    /// Traverse DAG from head to genesis (following parents)
    pub fn traverse_parents(&self, start: [u8; 32]) -> Vec<DagBlock> {
        let mut path = Vec::new();
        let mut current = start;

        while let Some(block) = self.get_block(&current) {
            path.push(block.clone());
            if block.is_genesis() {
                break;
            }
            current = block.parent_hash;
        }

        path
    }

    /// Get all children of a block
    pub fn get_children(&self, hash: &[u8; 32]) -> Vec<DagBlock> {
        self.vertices
            .get(hash)
            .map(|v| {
                v.children
                    .iter()
                    .filter_map(|child_hash| self.get_block(child_hash))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Check if DAG is linear (no forks)
    pub fn is_linear(&self) -> bool {
        for vertex in self.vertices.values() {
            if vertex.children.len() > 1 {
                return false;
            }
        }
        true
    }

    /// Get height of chain (from genesis to head)
    pub fn chain_height(&self) -> u64 {
        if let Some(head_hash) = self.head {
            let path = self.traverse_parents(head_hash);
            path.len() as u64
        } else {
            0
        }
    }
}

impl Default for Dag {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dag_block_creation() {
        let block = DagBlock::new(
            [1u8; 32],
            [0u8; 32],
            1,
            1,
            1000,
            "proposer1".to_string(),
            [2u8; 64],
        );
        assert_eq!(block.height, 1);
        assert!(block.is_genesis());
    }

    #[test]
    fn test_dag_vertex_creation() {
        let block = DagBlock::new(
            [1u8; 32],
            [0u8; 32],
            1,
            1,
            1000,
            "proposer1".to_string(),
            [2u8; 64],
        );
        let vertex = DagVertex::new(block);
        assert!(vertex.children.is_empty());
    }

    #[test]
    fn test_dag_add_genesis_block() {
        let mut dag = Dag::new();
        let block = DagBlock::new(
            [1u8; 32],
            [0u8; 32],
            0,
            0,
            1000,
            "proposer".to_string(),
            [2u8; 64],
        );
        assert!(dag.add_block(block).is_ok());
        assert_eq!(dag.block_count(), 1);
    }

    #[test]
    fn test_dag_add_child_block() {
        let mut dag = Dag::new();
        let genesis = DagBlock::new(
            [1u8; 32],
            [0u8; 32],
            0,
            0,
            1000,
            "proposer".to_string(),
            [2u8; 64],
        );
        let child = DagBlock::new(
            [2u8; 32],
            [1u8; 32],
            1,
            1,
            2000,
            "proposer".to_string(),
            [3u8; 64],
        );

        dag.add_block(genesis).unwrap();
        dag.add_block(child).unwrap();
        assert_eq!(dag.block_count(), 2);
    }

    #[test]
    fn test_dag_duplicate_block() {
        let mut dag = Dag::new();
        let block = DagBlock::new(
            [1u8; 32],
            [0u8; 32],
            0,
            0,
            1000,
            "proposer".to_string(),
            [2u8; 64],
        );
        dag.add_block(block.clone()).unwrap();
        assert!(dag.add_block(block).is_err());
    }

    #[test]
    fn test_dag_missing_parent() {
        let mut dag = Dag::new();
        let block = DagBlock::new(
            [1u8; 32],
            [99u8; 32],
            1,
            1,
            1000,
            "proposer".to_string(),
            [2u8; 64],
        );
        assert!(dag.add_block(block).is_err());
    }

    #[test]
    fn test_dag_get_block() {
        let mut dag = Dag::new();
        let block = DagBlock::new(
            [1u8; 32],
            [0u8; 32],
            0,
            0,
            1000,
            "proposer".to_string(),
            [2u8; 64],
        );
        dag.add_block(block.clone()).unwrap();
        assert_eq!(dag.get_block(&[1u8; 32]).unwrap(), block);
    }

    #[test]
    fn test_dag_traverse_parents() {
        let mut dag = Dag::new();
        let genesis = DagBlock::new(
            [1u8; 32],
            [0u8; 32],
            0,
            0,
            1000,
            "p1".to_string(),
            [2u8; 64],
        );
        let child = DagBlock::new(
            [2u8; 32],
            [1u8; 32],
            1,
            1,
            2000,
            "p2".to_string(),
            [3u8; 64],
        );

        dag.add_block(genesis).unwrap();
        dag.add_block(child).unwrap();

        let path = dag.traverse_parents([2u8; 32]);
        assert_eq!(path.len(), 2);
        assert_eq!(path[0].height, 1);
        assert_eq!(path[1].height, 0);
    }

    #[test]
    fn test_dag_is_linear() {
        let mut dag = Dag::new();
        let block = DagBlock::new(
            [1u8; 32],
            [0u8; 32],
            0,
            0,
            1000,
            "p".to_string(),
            [2u8; 64],
        );
        dag.add_block(block).unwrap();
        assert!(dag.is_linear());
    }

    #[test]
    fn test_dag_chain_height() {
        let mut dag = Dag::new();
        let block = DagBlock::new(
            [1u8; 32],
            [0u8; 32],
            0,
            0,
            1000,
            "p".to_string(),
            [2u8; 64],
        );
        dag.add_block(block).unwrap();
        assert_eq!(dag.chain_height(), 1);
    }
}
