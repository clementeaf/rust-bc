//! Conservation — value cannot be created or destroyed, only transformed.
//!
//! Like energy conservation in physics: the total quantity in the field is
//! invariant. Transfers are zero-sum operations — what leaves one cell
//! arrives at another, nothing more, nothing less.
//!
//! This makes double-spend not a "detected violation" but a physical
//! impossibility: you can't move what you don't have, and moving it
//! removes it from the source.
//!
//! Genesis events are the Big Bang — the only moment value is created.
//! After genesis, conservation is absolute.

use std::collections::HashMap;

use sha2::{Digest, Sha256};

use crate::Coord;

/// A conserved quantity held at a location in the field.
/// Analogous to mass-energy: it bends the field (via curvature)
/// and can only be transferred, never created after genesis.
#[derive(Clone, Debug)]
pub struct Balance {
    pub amount: u64,
    /// Monotonic counter — incremented on every mutation.
    /// Prevents replay: a transfer referencing nonce N is only valid
    /// if the source is currently at nonce N.
    pub nonce: u64,
}

impl Balance {
    pub fn new(amount: u64) -> Self {
        Self { amount, nonce: 0 }
    }

    pub fn zero() -> Self {
        Self { amount: 0, nonce: 0 }
    }
}

/// A single input to a transfer: where value comes FROM.
#[derive(Clone, Debug)]
pub struct TransferInput {
    /// Location in the field.
    pub coord: Coord,
    /// Amount being transferred out.
    pub amount: u64,
    /// Expected nonce — must match current nonce at coord.
    /// This is the conservation equivalent of "UTXO spent exactly once".
    pub expected_nonce: u64,
}

/// A single output of a transfer: where value goes TO.
#[derive(Clone, Debug)]
pub struct TransferOutput {
    /// Destination in the field.
    pub coord: Coord,
    /// Amount arriving.
    pub amount: u64,
}

/// A zero-sum transfer: sum(inputs) == sum(outputs), always.
/// This is not validated — it is structurally enforced.
#[derive(Clone, Debug)]
pub struct Transfer {
    pub inputs: Vec<TransferInput>,
    pub outputs: Vec<TransferOutput>,
    /// Content hash — uniquely identifies this transfer.
    pub hash: [u8; 32],
}

/// Why a transfer was rejected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConservationError {
    /// sum(inputs) != sum(outputs) — conservation violated.
    Imbalanced { inputs_total: u64, outputs_total: u64 },
    /// Source doesn't have enough value.
    InsufficientBalance { coord: Coord, has: u64, needs: u64 },
    /// Nonce mismatch — stale or replayed transfer.
    NonceMismatch { coord: Coord, expected: u64, actual: u64 },
    /// Transfer has no inputs or no outputs.
    Empty,
}

impl std::fmt::Display for ConservationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Imbalanced { inputs_total, outputs_total } =>
                write!(f, "conservation violated: inputs={inputs_total} != outputs={outputs_total}"),
            Self::InsufficientBalance { coord, has, needs } =>
                write!(f, "insufficient balance at {coord}: has {has}, needs {needs}"),
            Self::NonceMismatch { coord, expected, actual } =>
                write!(f, "nonce mismatch at {coord}: expected {expected}, got {actual}"),
            Self::Empty =>
                write!(f, "transfer must have at least one input and one output"),
        }
    }
}

impl std::error::Error for ConservationError {}

impl Transfer {
    /// Create a transfer. Fails at construction if not balanced.
    pub fn new(
        inputs: Vec<TransferInput>,
        outputs: Vec<TransferOutput>,
    ) -> Result<Self, ConservationError> {
        if inputs.is_empty() || outputs.is_empty() {
            return Err(ConservationError::Empty);
        }

        let in_total: u64 = inputs.iter().map(|i| i.amount).sum();
        let out_total: u64 = outputs.iter().map(|o| o.amount).sum();

        if in_total != out_total {
            return Err(ConservationError::Imbalanced {
                inputs_total: in_total,
                outputs_total: out_total,
            });
        }

        let hash = Self::compute_hash(&inputs, &outputs);
        Ok(Self { inputs, outputs, hash })
    }

    pub fn total(&self) -> u64 {
        self.inputs.iter().map(|i| i.amount).sum()
    }

    fn compute_hash(inputs: &[TransferInput], outputs: &[TransferOutput]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        for inp in inputs {
            hasher.update(inp.coord.t.to_le_bytes());
            hasher.update(inp.coord.c.to_le_bytes());
            hasher.update(inp.coord.o.to_le_bytes());
            hasher.update(inp.coord.v.to_le_bytes());
            hasher.update(inp.amount.to_le_bytes());
            hasher.update(inp.expected_nonce.to_le_bytes());
        }
        for out in outputs {
            hasher.update(out.coord.t.to_le_bytes());
            hasher.update(out.coord.c.to_le_bytes());
            hasher.update(out.coord.o.to_le_bytes());
            hasher.update(out.coord.v.to_le_bytes());
            hasher.update(out.amount.to_le_bytes());
        }
        hasher.finalize().into()
    }
}

/// The conserved field: tracks value distribution across coordinates.
/// Total supply is fixed after genesis. Every operation preserves the invariant:
///   sum(all balances) == genesis_supply
pub struct ConservedField {
    balances: HashMap<Coord, Balance>,
    /// Total supply — set at genesis, immutable forever after.
    pub total_supply: u64,
    /// History of applied transfers (for audit).
    history: Vec<Transfer>,
}

impl ConservedField {
    /// Create a new conserved field with zero supply.
    /// Use `genesis()` to inject the initial value.
    pub fn new() -> Self {
        Self {
            balances: HashMap::new(),
            total_supply: 0,
            history: Vec::new(),
        }
    }

    /// The Big Bang: create value from nothing, exactly once.
    /// After this, conservation is absolute — no more minting.
    /// Returns false if genesis was already performed.
    pub fn genesis(&mut self, distributions: &[(Coord, u64)]) -> bool {
        if self.total_supply > 0 {
            return false; // already had a Big Bang
        }

        let total: u64 = distributions.iter().map(|(_, amt)| amt).sum();
        for (coord, amount) in distributions {
            let balance = self.balances.entry(*coord).or_insert_with(Balance::zero);
            balance.amount += amount;
        }
        self.total_supply = total;
        true
    }

    /// Apply a transfer. Enforces:
    /// 1. Balance (structurally guaranteed by Transfer::new)
    /// 2. Sufficient funds at each source
    /// 3. Nonce freshness (no replay)
    ///
    /// This is atomic: either all inputs/outputs apply, or none.
    pub fn apply(&mut self, transfer: &Transfer) -> Result<(), ConservationError> {
        // Validate all inputs BEFORE mutating anything
        for inp in &transfer.inputs {
            let balance = self.balance_at(inp.coord);
            if balance.nonce != inp.expected_nonce {
                return Err(ConservationError::NonceMismatch {
                    coord: inp.coord,
                    expected: inp.expected_nonce,
                    actual: balance.nonce,
                });
            }
            if balance.amount < inp.amount {
                return Err(ConservationError::InsufficientBalance {
                    coord: inp.coord,
                    has: balance.amount,
                    needs: inp.amount,
                });
            }
        }

        // Apply: debit inputs
        for inp in &transfer.inputs {
            let balance = self.balances.entry(inp.coord).or_insert_with(Balance::zero);
            balance.amount -= inp.amount;
            balance.nonce += 1;
        }

        // Apply: credit outputs
        for out in &transfer.outputs {
            let balance = self.balances.entry(out.coord).or_insert_with(Balance::zero);
            balance.amount += out.amount;
        }

        self.history.push(transfer.clone());
        Ok(())
    }

    /// Get the balance at a coordinate.
    pub fn balance_at(&self, coord: Coord) -> &Balance {
        static ZERO: std::sync::LazyLock<Balance> = std::sync::LazyLock::new(Balance::zero);
        self.balances.get(&coord).unwrap_or(&ZERO)
    }

    /// Verify the conservation invariant holds.
    /// Should ALWAYS be true — if this returns false, there's a bug.
    pub fn is_conserved(&self) -> bool {
        let sum: u64 = self.balances.values().map(|b| b.amount).sum();
        sum == self.total_supply
    }

    /// Number of non-zero balances.
    pub fn active_positions(&self) -> usize {
        self.balances.values().filter(|b| b.amount > 0).count()
    }

    /// Number of transfers applied.
    pub fn transfer_count(&self) -> usize {
        self.history.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(t: usize, c: usize, o: usize, v: usize) -> Coord {
        Coord { t, c, o, v }
    }

    #[test]
    fn genesis_sets_total_supply() {
        let mut field = ConservedField::new();
        let ok = field.genesis(&[(c(0,0,0,0), 1000), (c(1,0,0,0), 500)]);
        assert!(ok);
        assert_eq!(field.total_supply, 1500);
        assert!(field.is_conserved());
    }

    #[test]
    fn genesis_only_once() {
        let mut field = ConservedField::new();
        assert!(field.genesis(&[(c(0,0,0,0), 1000)]));
        assert!(!field.genesis(&[(c(1,0,0,0), 500)]));
        assert_eq!(field.total_supply, 1000);
    }

    #[test]
    fn balanced_transfer_preserves_invariant() {
        let mut field = ConservedField::new();
        field.genesis(&[(c(0,0,0,0), 1000)]);

        let tx = Transfer::new(
            vec![TransferInput { coord: c(0,0,0,0), amount: 300, expected_nonce: 0 }],
            vec![TransferOutput { coord: c(1,0,0,0), amount: 300 }],
        ).unwrap();

        field.apply(&tx).unwrap();

        assert_eq!(field.balance_at(c(0,0,0,0)).amount, 700);
        assert_eq!(field.balance_at(c(1,0,0,0)).amount, 300);
        assert!(field.is_conserved());
    }

    #[test]
    fn imbalanced_transfer_rejected_at_construction() {
        let result = Transfer::new(
            vec![TransferInput { coord: c(0,0,0,0), amount: 100, expected_nonce: 0 }],
            vec![TransferOutput { coord: c(1,0,0,0), amount: 200 }],
        );
        assert!(matches!(result, Err(ConservationError::Imbalanced { .. })));
    }

    #[test]
    fn insufficient_balance_rejected() {
        let mut field = ConservedField::new();
        field.genesis(&[(c(0,0,0,0), 100)]);

        let tx = Transfer::new(
            vec![TransferInput { coord: c(0,0,0,0), amount: 200, expected_nonce: 0 }],
            vec![TransferOutput { coord: c(1,0,0,0), amount: 200 }],
        ).unwrap();

        let result = field.apply(&tx);
        assert!(matches!(result, Err(ConservationError::InsufficientBalance { .. })));
        // Field unchanged
        assert_eq!(field.balance_at(c(0,0,0,0)).amount, 100);
        assert!(field.is_conserved());
    }

    #[test]
    fn nonce_prevents_replay() {
        let mut field = ConservedField::new();
        field.genesis(&[(c(0,0,0,0), 1000)]);

        let tx = Transfer::new(
            vec![TransferInput { coord: c(0,0,0,0), amount: 100, expected_nonce: 0 }],
            vec![TransferOutput { coord: c(1,0,0,0), amount: 100 }],
        ).unwrap();

        field.apply(&tx).unwrap();

        // Replay same transfer — nonce now 1, transfer expects 0
        let result = field.apply(&tx);
        assert!(matches!(result, Err(ConservationError::NonceMismatch { .. })));
        assert!(field.is_conserved());
    }

    #[test]
    fn multi_input_multi_output() {
        let mut field = ConservedField::new();
        field.genesis(&[
            (c(0,0,0,0), 500),
            (c(1,0,0,0), 300),
        ]);

        // Gather from two sources, distribute to three destinations
        let tx = Transfer::new(
            vec![
                TransferInput { coord: c(0,0,0,0), amount: 400, expected_nonce: 0 },
                TransferInput { coord: c(1,0,0,0), amount: 200, expected_nonce: 0 },
            ],
            vec![
                TransferOutput { coord: c(2,0,0,0), amount: 250 },
                TransferOutput { coord: c(3,0,0,0), amount: 250 },
                TransferOutput { coord: c(4,0,0,0), amount: 100 },
            ],
        ).unwrap();

        field.apply(&tx).unwrap();

        assert_eq!(field.balance_at(c(0,0,0,0)).amount, 100);
        assert_eq!(field.balance_at(c(1,0,0,0)).amount, 100);
        assert_eq!(field.balance_at(c(2,0,0,0)).amount, 250);
        assert_eq!(field.balance_at(c(3,0,0,0)).amount, 250);
        assert_eq!(field.balance_at(c(4,0,0,0)).amount, 100);
        assert!(field.is_conserved());
    }

    #[test]
    fn chained_transfers_increment_nonce() {
        let mut field = ConservedField::new();
        field.genesis(&[(c(0,0,0,0), 1000)]);

        for i in 0..5u64 {
            let tx = Transfer::new(
                vec![TransferInput { coord: c(0,0,0,0), amount: 100, expected_nonce: i }],
                vec![TransferOutput { coord: c((i + 1) as usize,0,0,0), amount: 100 }],
            ).unwrap();
            field.apply(&tx).unwrap();
        }

        assert_eq!(field.balance_at(c(0,0,0,0)).amount, 500);
        assert_eq!(field.balance_at(c(0,0,0,0)).nonce, 5);
        assert_eq!(field.transfer_count(), 5);
        assert!(field.is_conserved());
    }

    #[test]
    fn empty_transfer_rejected() {
        let result = Transfer::new(vec![], vec![]);
        assert!(matches!(result, Err(ConservationError::Empty)));
    }
}
