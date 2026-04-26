//! Economics: fixed supply, conservation of curvature, genesis allocation.
//!
//! Total curvature is defined at genesis and NEVER changes.
//! Transfers redistribute curvature — they don't create or destroy it.
//! This is the economic foundation: scarcity by geometry.

use crate::persistence::EventLog;
use crate::{Coord, Field};

/// Genesis configuration: defines the total curvature of the universe.
#[derive(Clone, Debug)]
pub struct Genesis {
    /// Total curvature that will ever exist. Constant. Like the speed of light.
    pub total_supply: f64,
    /// Initial allocations: (region, amount, label)
    pub allocations: Vec<(usize, f64, String)>,
}

impl Genesis {
    pub fn new(total_supply: f64) -> Self {
        Self {
            total_supply,
            allocations: Vec::new(),
        }
    }

    /// Allocate curvature to a region. Fails if total exceeds supply.
    pub fn allocate(
        &mut self,
        region: usize,
        amount: f64,
        label: impl Into<String>,
    ) -> Result<(), String> {
        let allocated: f64 = self.allocations.iter().map(|(_, a, _)| a).sum();
        if allocated + amount > self.total_supply {
            return Err(format!(
                "Cannot allocate {:.2}: would exceed total supply {:.2} (already allocated {:.2})",
                amount, self.total_supply, allocated
            ));
        }
        self.allocations.push((region, amount, label.into()));
        Ok(())
    }

    /// Apply genesis to a field.
    pub fn apply(&self, field: &mut Field) {
        for (region, amount, _) in &self.allocations {
            field.set_capacity(*region, *amount);
        }
    }

    /// Total allocated so far.
    pub fn allocated(&self) -> f64 {
        self.allocations.iter().map(|(_, a, _)| a).sum()
    }

    /// Remaining unallocated curvature.
    pub fn remaining(&self) -> f64 {
        self.total_supply - self.allocated()
    }
}

/// Economy: manages curvature transfers with strict conservation.
pub struct Economy {
    pub field: Field,
    pub genesis: Genesis,
    log: EventLog,
}

impl Economy {
    pub fn new(field_size: usize, genesis: Genesis) -> Self {
        let mut field = Field::new(field_size);
        genesis.apply(&mut field);
        Self {
            field,
            genesis,
            log: EventLog::new(),
        }
    }

    pub fn with_persistence(field_size: usize, genesis: Genesis, path: &str) -> Self {
        let log = EventLog::with_file(path);
        let mut field = Field::new(field_size);
        genesis.apply(&mut field);
        log.replay(&mut field);
        Self {
            field,
            genesis,
            log,
        }
    }

    /// Total curvature across all regions — should ALWAYS equal genesis supply.
    pub fn total_curvature(&self) -> f64 {
        let mut total = 0.0;
        for alloc in &self.genesis.allocations {
            let region = alloc.0;
            total += self.field.capacity(region).unwrap_or(0.0);
        }
        total
    }

    /// Get curvature balance for a region.
    pub fn balance(&self, region: usize) -> f64 {
        self.field.capacity(region).unwrap_or(0.0)
    }

    /// Transfer curvature from one region to another.
    /// Strictly conserving: amount leaves sender, same amount arrives at receiver.
    /// Nothing created. Nothing destroyed. Only redistributed.
    pub fn transfer(
        &mut self,
        from_region: usize,
        to_region: usize,
        amount: f64,
        event_id: &str,
        coord: Coord,
        _timestamp: u64,
    ) -> Result<(), String> {
        if amount <= 0.0 {
            return Err("Amount must be positive".into());
        }

        let from_balance = self.balance(from_region);
        if from_balance < amount {
            return Err(format!(
                "Region {} has {:.2} curvature but tried to send {:.2}",
                from_region, from_balance, amount
            ));
        }

        // Conservation: subtract from sender, add to receiver
        // Total before = total after. Always.
        self.field.set_capacity(from_region, from_balance - amount);
        let to_balance = self.balance(to_region);
        self.field.set_capacity(to_region, to_balance + amount);

        // Seed the event as a deformation in the field
        let label = format!(
            "-{:.2}:r{}→r{}[{}]",
            amount, from_region, to_region, event_id
        );
        self.field.seed_named(coord, &label);
        self.log.record_seed(coord, &label);

        Ok(())
    }

    /// Evolve the field.
    pub fn settle(&mut self) {
        crate::evolve_to_equilibrium(&mut self.field, 10);
    }

    /// Verify conservation: total curvature == genesis supply.
    /// This should ALWAYS be true. If not, there's a bug.
    pub fn verify_conservation(&self) -> bool {
        let total = self.total_curvature() + self.genesis.remaining();
        (total - self.genesis.total_supply).abs() < 0.0001
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn genesis_supply_is_fixed() {
        let mut gen = Genesis::new(1_000_000.0);
        gen.allocate(1, 500_000.0, "founder").unwrap();
        gen.allocate(2, 300_000.0, "early").unwrap();
        gen.allocate(3, 200_000.0, "reserve").unwrap();

        // Can't allocate more than total
        let result = gen.allocate(4, 1.0, "overflow");
        assert!(result.is_err(), "Should reject allocation exceeding supply");

        assert_eq!(gen.allocated(), 1_000_000.0);
        assert_eq!(gen.remaining(), 0.0);
    }

    #[test]
    fn transfer_conserves_curvature() {
        let mut gen = Genesis::new(1000.0);
        gen.allocate(1, 600.0, "alice").unwrap();
        gen.allocate(2, 400.0, "bob").unwrap();

        let mut eco = Economy::new(8, gen);

        assert_eq!(eco.balance(1), 600.0);
        assert_eq!(eco.balance(2), 400.0);
        assert!(eco.verify_conservation());

        // Transfer 200 from alice to bob
        eco.transfer(
            1,
            2,
            200.0,
            "tx-001",
            Coord {
                t: 1,
                c: 1,
                o: 1,
                v: 1,
            },
            0,
        )
        .unwrap();

        assert_eq!(eco.balance(1), 400.0);
        assert_eq!(eco.balance(2), 600.0);
        assert!(
            eco.verify_conservation(),
            "Conservation must hold after transfer"
        );
    }

    #[test]
    fn cannot_transfer_more_than_balance() {
        let mut gen = Genesis::new(100.0);
        gen.allocate(1, 50.0, "alice").unwrap();
        gen.allocate(2, 50.0, "bob").unwrap();

        let mut eco = Economy::new(8, gen);

        let result = eco.transfer(
            1,
            2,
            80.0,
            "overdraft",
            Coord {
                t: 1,
                c: 1,
                o: 1,
                v: 1,
            },
            0,
        );
        assert!(result.is_err());
        assert_eq!(eco.balance(1), 50.0); // unchanged
        assert!(eco.verify_conservation());
    }

    #[test]
    fn chain_of_transfers_conserves() {
        let mut gen = Genesis::new(1000.0);
        gen.allocate(1, 1000.0, "origin").unwrap();
        gen.allocate(2, 0.0, "bob").unwrap();
        gen.allocate(3, 0.0, "carol").unwrap();
        gen.allocate(4, 0.0, "dave").unwrap();

        let mut eco = Economy::new(8, gen);

        // Chain: 1→2→3→4
        eco.transfer(
            1,
            2,
            500.0,
            "tx-1",
            Coord {
                t: 1,
                c: 1,
                o: 1,
                v: 1,
            },
            0,
        )
        .unwrap();
        eco.transfer(
            2,
            3,
            300.0,
            "tx-2",
            Coord {
                t: 2,
                c: 1,
                o: 2,
                v: 1,
            },
            0,
        )
        .unwrap();
        eco.transfer(
            3,
            4,
            100.0,
            "tx-3",
            Coord {
                t: 3,
                c: 1,
                o: 3,
                v: 1,
            },
            0,
        )
        .unwrap();

        assert_eq!(eco.balance(1), 500.0);
        assert_eq!(eco.balance(2), 200.0);
        assert_eq!(eco.balance(3), 200.0);
        assert_eq!(eco.balance(4), 100.0);

        // Total = 500 + 200 + 200 + 100 = 1000 = genesis supply
        assert!(
            eco.verify_conservation(),
            "Chain of transfers must conserve"
        );
    }

    #[test]
    fn many_transfers_never_break_conservation() {
        let mut gen = Genesis::new(10_000.0);
        for i in 0..10 {
            gen.allocate(i, 1000.0, &format!("participant-{}", i))
                .unwrap();
        }

        let mut eco = Economy::new(8, gen);

        // 50 random transfers between participants
        for i in 0..50 {
            let from = i % 10;
            let to = (i * 3 + 1) % 10;
            if from == to {
                continue;
            }
            let balance = eco.balance(from);
            if balance >= 10.0 {
                eco.transfer(
                    from,
                    to,
                    10.0,
                    &format!("tx-{}", i),
                    Coord {
                        t: i % 8,
                        c: (i * 2) % 8,
                        o: from,
                        v: to,
                    },
                    i as u64,
                )
                .unwrap();
            }
        }

        assert!(
            eco.verify_conservation(),
            "50 transfers must conserve total supply"
        );
        assert_eq!(eco.total_curvature() + eco.genesis.remaining(), 10_000.0);
    }

    #[test]
    fn double_spend_impossible() {
        let mut gen = Genesis::new(100.0);
        gen.allocate(1, 100.0, "alice").unwrap();
        gen.allocate(2, 0.0, "bob").unwrap();
        gen.allocate(3, 0.0, "carol").unwrap();

        let mut eco = Economy::new(8, gen);

        // Alice sends 100 to Bob
        eco.transfer(
            1,
            2,
            100.0,
            "tx-1",
            Coord {
                t: 1,
                c: 1,
                o: 1,
                v: 1,
            },
            0,
        )
        .unwrap();

        // Alice tries to send 100 to Carol — impossible, balance is 0
        let result = eco.transfer(
            1,
            3,
            100.0,
            "tx-2",
            Coord {
                t: 2,
                c: 2,
                o: 1,
                v: 2,
            },
            0,
        );
        assert!(result.is_err(), "Double spend must be impossible");

        assert_eq!(eco.balance(1), 0.0);
        assert_eq!(eco.balance(2), 100.0);
        assert_eq!(eco.balance(3), 0.0);
        assert!(eco.verify_conservation());
    }
}
