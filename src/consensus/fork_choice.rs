//! Fork choice rule for DAG consensus.
//!
//! `ForkChoice` wraps the canonical-chain selection logic and exposes a
//! configurable rule so callers can choose between:
//! - `HeaviestSubtree` — pick the child with the most descendants (default)
//! - `LongestChain`    — pick the child that leads to the greatest block height

use crate::consensus::dag::Dag;

/// Strategy used to select the canonical chain at each fork point.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ForkChoiceRule {
    /// Choose the subtree with the greatest number of descendants.
    /// Ties are broken by the lexicographically smallest block hash.
    #[default]
    HeaviestSubtree,

    #[allow(dead_code)]
    /// Choose the branch that reaches the greatest block height.
    /// Ties are broken by the lexicographically smallest block hash.
    LongestChain,
}

/// Fork choice engine.
///
/// ```rust
/// use rust_bc::consensus::{ForkChoice, ForkChoiceRule};
/// let fc = ForkChoice::new(ForkChoiceRule::LongestChain);
/// ```
#[derive(Debug, Clone)]
pub struct ForkChoice {
    #[allow(dead_code)]
    rule: ForkChoiceRule,
}

impl ForkChoice {
    /// Create a new `ForkChoice` with the given rule.
    pub fn new(rule: ForkChoiceRule) -> Self {
        Self { rule }
    }

    #[allow(dead_code)]
    /// Return the canonical chain (genesis → tip) according to the configured
    /// rule.  Returns an empty `Vec` when the DAG has no genesis block.
    pub fn canonical_chain(&self, dag: &Dag) -> Vec<[u8; 32]> {
        match self.rule {
            ForkChoiceRule::HeaviestSubtree => dag.canonical_chain(),
            ForkChoiceRule::LongestChain => self.longest_chain(dag),
        }
    }

    #[allow(dead_code)]
    /// Resolve a set of competing tips to the canonical one.
    ///
    /// Returns `None` if `candidates` is empty or none appear in the canonical
    /// chain.
    pub fn resolve(&self, dag: &Dag, candidates: &[[u8; 32]]) -> Option<[u8; 32]> {
        if candidates.is_empty() {
            return None;
        }
        let chain = self.canonical_chain(dag);
        let positions: std::collections::HashMap<[u8; 32], usize> =
            chain.iter().enumerate().map(|(i, h)| (*h, i)).collect();

        candidates
            .iter()
            .filter_map(|h| positions.get(h).map(|pos| (*h, *pos)))
            .max_by_key(|(_, pos)| *pos)
            .map(|(h, _)| h)
    }

    // --- LongestChain implementation ---

    /// Walk the DAG from genesis, at each fork choosing the child whose
    /// subtree contains the block with the greatest height.
    fn longest_chain(&self, dag: &Dag) -> Vec<[u8; 32]> {
        let genesis_hash = dag
            .vertices()
            .values()
            .find(|v| v.block.is_genesis())
            .map(|v| v.block.hash);

        let mut current = match genesis_hash {
            Some(h) => h,
            None => return Vec::new(),
        };

        let mut chain = vec![current];

        while let Some(v) = dag.vertices().get(&current) {
            let children = v.children.clone();

            if children.is_empty() {
                break;
            }

            let best = children
                .iter()
                .max_by_key(|h| {
                    let max_height = self.max_height_in_subtree(dag, h);
                    (max_height, std::cmp::Reverse(*h))
                })
                .copied()
                .expect("children is non-empty");

            chain.push(best);
            current = best;
        }

        chain
    }

    /// Recursively find the maximum block height reachable from `hash`.
    #[allow(clippy::only_used_in_recursion)]
    fn max_height_in_subtree(&self, dag: &Dag, hash: &[u8; 32]) -> u64 {
        let vertex = match dag.vertices().get(hash) {
            Some(v) => v,
            None => return 0,
        };

        let own_height = vertex.block.height;
        let children = vertex.children.clone();

        if children.is_empty() {
            return own_height;
        }

        children
            .iter()
            .map(|c| self.max_height_in_subtree(dag, c))
            .max()
            .unwrap_or(own_height)
    }
}

impl Default for ForkChoice {
    fn default() -> Self {
        Self::new(ForkChoiceRule::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::dag::{Dag, DagBlock};

    fn mk(id: u8) -> [u8; 32] {
        let mut h = [0u8; 32];
        h[0] = id;
        h
    }

    fn block(hash: u8, parent: u8, height: u64) -> DagBlock {
        DagBlock::new(
            mk(hash),
            mk(parent),
            height,
            0,
            1000,
            "p".to_string(),
            vec![2u8; 64],
        )
    }

    fn forked_dag() -> Dag {
        // genesis=1 → branch A: 2→4 (height 2), branch B: 3 (height 1)
        let mut dag = Dag::new();
        dag.add_block(block(1, 0, 0)).unwrap();
        dag.add_block(block(2, 1, 1)).unwrap();
        dag.add_block(block(3, 1, 1)).unwrap();
        dag.add_block(block(4, 2, 2)).unwrap();
        dag
    }

    // --- HeaviestSubtree ---

    #[test]
    fn heaviest_subtree_linear() {
        let fc = ForkChoice::default();
        let mut dag = Dag::new();
        dag.add_block(block(1, 0, 0)).unwrap();
        dag.add_block(block(2, 1, 1)).unwrap();
        assert_eq!(fc.canonical_chain(&dag), vec![mk(1), mk(2)]);
    }

    #[test]
    fn heaviest_subtree_picks_heavier_fork() {
        let fc = ForkChoice::default();
        let dag = forked_dag();
        assert_eq!(fc.canonical_chain(&dag), vec![mk(1), mk(2), mk(4)]);
    }

    #[test]
    fn heaviest_subtree_resolve_canonical_tip() {
        let fc = ForkChoice::default();
        let dag = forked_dag();
        assert_eq!(fc.resolve(&dag, &[mk(3), mk(4)]), Some(mk(4)));
    }

    #[test]
    fn heaviest_subtree_resolve_stale_tip_returns_none() {
        let fc = ForkChoice::default();
        let dag = forked_dag();
        assert_eq!(fc.resolve(&dag, &[mk(3)]), None);
    }

    // --- LongestChain ---

    #[test]
    fn longest_chain_linear() {
        let fc = ForkChoice::new(ForkChoiceRule::LongestChain);
        let mut dag = Dag::new();
        dag.add_block(block(1, 0, 0)).unwrap();
        dag.add_block(block(2, 1, 1)).unwrap();
        dag.add_block(block(3, 2, 2)).unwrap();
        assert_eq!(fc.canonical_chain(&dag), vec![mk(1), mk(2), mk(3)]);
    }

    #[test]
    fn longest_chain_picks_deeper_branch() {
        let fc = ForkChoice::new(ForkChoiceRule::LongestChain);
        let dag = forked_dag(); // branch A reaches height 2, B stays at 1
        assert_eq!(fc.canonical_chain(&dag), vec![mk(1), mk(2), mk(4)]);
    }

    #[test]
    fn longest_chain_resolve() {
        let fc = ForkChoice::new(ForkChoiceRule::LongestChain);
        let dag = forked_dag();
        assert_eq!(fc.resolve(&dag, &[mk(3), mk(4)]), Some(mk(4)));
    }

    #[test]
    fn longest_chain_empty_dag() {
        let fc = ForkChoice::new(ForkChoiceRule::LongestChain);
        assert!(fc.canonical_chain(&Dag::new()).is_empty());
    }

    // --- rule equality ---

    #[test]
    fn default_rule_is_heaviest_subtree() {
        assert_eq!(ForkChoiceRule::default(), ForkChoiceRule::HeaviestSubtree);
    }
}
