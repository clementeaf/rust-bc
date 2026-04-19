//! Wallet and monetary transactions on the tesseract field.
//!
//! Balances are not stored — they are derived from the field.
//! A balance is the sum of crystallized credits minus debits
//! in a participant's region of the space.

use crate::{Coord, Field, evolve_to_equilibrium};
use crate::mapper::{CoordMapper, Event};
use std::collections::HashMap;

/// A monetary transfer: the space bends between sender and receiver.
#[derive(Clone, Debug)]
pub struct Transfer {
    pub id: String,
    pub from: String,
    pub to: String,
    pub amount: f64,
    pub timestamp: u64,
    pub channel: String,
}

/// Ledger built on the tesseract field.
pub struct TesseractLedger {
    pub field: Field,
    pub mapper: CoordMapper,
    /// Initial balances seeded (genesis allocations).
    genesis: HashMap<String, f64>,
    /// All transfers executed (for balance derivation).
    transfers: Vec<Transfer>,
}

impl TesseractLedger {
    pub fn new(field_size: usize) -> Self {
        Self {
            field: Field::new(field_size),
            mapper: CoordMapper::new(field_size).with_time_bucket(60),
            genesis: HashMap::new(),
            transfers: Vec::new(),
        }
    }

    /// Genesis allocation: give initial tokens to a participant.
    /// This is the first deformation — the space bends at the participant's region.
    pub fn genesis_allocate(&mut self, participant: &str, amount: f64, timestamp: u64) {
        let event = Event {
            id: format!("genesis:{}", participant),
            timestamp,
            channel: "genesis".into(),
            org: participant.into(),
            data: format!("allocate:{}", amount),
        };
        let coord = self.mapper.map(&event);
        let label = format!("+{}→{}[{}]", amount, participant, coord);
        self.field.seed_named(coord, &label);
        self.genesis.insert(participant.to_string(), amount);
    }

    /// Transfer tokens from one participant to another.
    /// Returns the coordinate where the transfer crystallizes.
    ///
    /// The transfer is a deformation of the space. It seeds TWO orbitals:
    /// - A debit at the sender's region
    /// - A credit at the receiver's region
    /// The overlap between them IS the transfer record.
    pub fn transfer(&mut self, tx: Transfer) -> Result<Coord, String> {
        // Check balance
        let balance = self.balance(&tx.from);
        if balance < tx.amount {
            return Err(format!(
                "{} has {:.2} but tried to send {:.2}",
                tx.from, balance, tx.amount
            ));
        }

        // Double-spend check: same tx id = same coordinate = same deformation
        let event = Event {
            id: tx.id.clone(),
            timestamp: tx.timestamp,
            channel: tx.channel.clone(),
            org: tx.from.clone(),
            data: format!("{:.2}:{}→{}", tx.amount, tx.from, tx.to),
        };
        let coord = self.mapper.map(&event);

        // Check if this exact transfer already crystallized
        if self.field.get(coord).crystallized {
            let record = self.field.get(coord).record();
            if record.contains(&tx.id) {
                return Err(format!("double-spend: tx {} already crystallized at {}", tx.id, coord));
            }
        }

        // Seed the debit (sender's region)
        let debit_label = format!("-{:.2}:{}→{}[{}]", tx.amount, tx.from, tx.to, tx.id);
        self.field.seed_named(coord, &debit_label);

        // Seed the credit (receiver's region)
        let credit_event = Event {
            id: format!("{}-credit", tx.id),
            timestamp: tx.timestamp,
            channel: tx.channel.clone(),
            org: tx.to.clone(),
            data: format!("+{:.2}:{}→{}", tx.amount, tx.from, tx.to),
        };
        let credit_coord = self.mapper.map(&credit_event);
        let credit_label = format!("+{:.2}:{}←{}[{}]", tx.amount, tx.to, tx.from, tx.id);
        self.field.seed_named(credit_coord, &credit_label);

        self.transfers.push(tx);

        Ok(coord)
    }

    /// Evolve the field to let transfers crystallize.
    pub fn settle(&mut self) {
        evolve_to_equilibrium(&mut self.field, 10);
    }

    /// Derive balance from genesis allocations and crystallized transfers.
    /// Balance = genesis + credits - debits.
    pub fn balance(&self, participant: &str) -> f64 {
        let genesis = self.genesis.get(participant).copied().unwrap_or(0.0);

        let debits: f64 = self.transfers.iter()
            .filter(|t| t.from == participant)
            .map(|t| t.amount)
            .sum();

        let credits: f64 = self.transfers.iter()
            .filter(|t| t.to == participant)
            .map(|t| t.amount)
            .sum();

        genesis + credits - debits
    }

    /// Check if a specific transfer has crystallized.
    pub fn is_confirmed(&self, tx_id: &str) -> bool {
        self.transfers.iter()
            .filter(|t| t.id == tx_id)
            .any(|t| {
                let event = Event {
                    id: t.id.clone(),
                    timestamp: t.timestamp,
                    channel: t.channel.clone(),
                    org: t.from.clone(),
                    data: format!("{:.2}:{}→{}", t.amount, t.from, t.to),
                };
                let coord = self.mapper.map(&event);
                self.field.get(coord).crystallized
            })
    }
}
