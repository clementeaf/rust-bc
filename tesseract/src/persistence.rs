//! Persistence: store events, not field state.
//!
//! The field is deterministic from its seeds (Uniqueness Theorem).
//! We only persist the events that caused the deformations.
//! On startup, replay events to reconstruct the exact same field.
//!
//! This is like storing causes, not effects.
//! The crystallizations are consequences — they can be recomputed.

use crate::{evolve_to_equilibrium, Coord, Field};
use std::fs;
use std::path::Path;

/// A persisted event: everything needed to replay a seed.
#[derive(Clone, Debug)]
pub struct PersistedEvent {
    pub id: String,
    pub coord: Coord,
    pub capacity_region: Option<usize>,
    pub capacity_amount: Option<f64>,
}

/// Event log: append-only list of events that shaped the field.
pub struct EventLog {
    events: Vec<PersistedEvent>,
    path: Option<String>,
}

impl EventLog {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            path: None,
        }
    }

    /// Create a log backed by a file.
    pub fn with_file(path: impl Into<String>) -> Self {
        let p: String = path.into();
        let mut log = Self {
            events: Vec::new(),
            path: Some(p.clone()),
        };
        // Load existing events from file
        if Path::new(&p).exists() {
            if let Ok(contents) = fs::read_to_string(&p) {
                log.events = parse_events(&contents);
            }
        }
        log
    }

    /// Record and persist a seed event.
    pub fn record_seed(&mut self, coord: Coord, event_id: &str) {
        let event = PersistedEvent {
            id: event_id.to_string(),
            coord,
            capacity_region: None,
            capacity_amount: None,
        };
        self.events.push(event);
        self.flush();
    }

    /// Record and persist a capacity allocation.
    pub fn record_capacity(&mut self, region: usize, amount: f64) {
        let event = PersistedEvent {
            id: format!("capacity:{}:{}", region, amount),
            coord: Coord {
                t: 0,
                c: 0,
                o: region,
                v: 0,
            },
            capacity_region: Some(region),
            capacity_amount: Some(amount),
        };
        self.events.push(event);
        self.flush();
    }

    /// Replay all events onto a field, reconstructing its state.
    pub fn replay(&self, field: &mut Field) {
        for event in &self.events {
            if let (Some(region), Some(amount)) = (event.capacity_region, event.capacity_amount) {
                field.add_capacity(region, amount);
            } else {
                field.seed_named(event.coord, &event.id);
            }
        }
        evolve_to_equilibrium(field, 10);
    }

    /// Number of recorded events.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Get all events.
    pub fn events(&self) -> &[PersistedEvent] {
        &self.events
    }

    fn flush(&self) {
        if let Some(ref path) = self.path {
            let content = serialize_events(&self.events);
            let _ = fs::write(path, content);
        }
    }
}

/// Serialize events to a simple text format.
/// Each line: type|id|t,c,o,v[|region:amount]
fn serialize_events(events: &[PersistedEvent]) -> String {
    let mut lines = Vec::with_capacity(events.len());
    for ev in events {
        if let (Some(region), Some(amount)) = (ev.capacity_region, ev.capacity_amount) {
            lines.push(format!(
                "CAP|{}|{},{},{},{}|{}:{}",
                ev.id, ev.coord.t, ev.coord.c, ev.coord.o, ev.coord.v, region, amount
            ));
        } else {
            lines.push(format!(
                "SEED|{}|{},{},{},{}",
                ev.id, ev.coord.t, ev.coord.c, ev.coord.o, ev.coord.v
            ));
        }
    }
    lines.join("\n")
}

/// Parse events from the text format.
fn parse_events(content: &str) -> Vec<PersistedEvent> {
    let mut events = Vec::new();
    for line in content.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() < 3 {
            continue;
        }

        let coords: Vec<usize> = parts[2]
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();
        if coords.len() != 4 {
            continue;
        }
        let coord = Coord {
            t: coords[0],
            c: coords[1],
            o: coords[2],
            v: coords[3],
        };

        match parts[0] {
            "SEED" => {
                events.push(PersistedEvent {
                    id: parts[1].to_string(),
                    coord,
                    capacity_region: None,
                    capacity_amount: None,
                });
            }
            "CAP" if parts.len() >= 4 => {
                let cap_parts: Vec<&str> = parts[3].split(':').collect();
                if cap_parts.len() == 2 {
                    events.push(PersistedEvent {
                        id: parts[1].to_string(),
                        coord,
                        capacity_region: cap_parts[0].parse().ok(),
                        capacity_amount: cap_parts[1].parse().ok(),
                    });
                }
            }
            _ => {}
        }
    }
    events
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_and_replay_produces_same_field() {
        let mut field1 = Field::new(8);
        let mut log = EventLog::new();

        // Record events
        let c1 = Coord {
            t: 2,
            c: 3,
            o: 3,
            v: 3,
        };
        let c2 = Coord {
            t: 4,
            c: 3,
            o: 3,
            v: 3,
        };

        field1.seed_named(c1, "alice→bob:10");
        log.record_seed(c1, "alice→bob:10");

        field1.seed_named(c2, "bob→carol:5");
        log.record_seed(c2, "bob→carol:5");

        evolve_to_equilibrium(&mut field1, 10);

        // Replay onto a fresh field
        let mut field2 = Field::new(8);
        log.replay(&mut field2);

        // Same crystallization state
        assert_eq!(field1.crystallized_count(), field2.crystallized_count());
        assert_eq!(field1.get(c1).crystallized, field2.get(c1).crystallized,);
        assert_eq!(field1.get(c2).crystallized, field2.get(c2).crystallized,);
    }

    #[test]
    fn file_persistence_survives_restart() {
        let tmp = std::env::temp_dir().join("tesseract_test_log.txt");
        let path = tmp.to_str().unwrap().to_string();

        // Clean up
        let _ = fs::remove_file(&path);

        // Session 1: write events
        {
            let mut log = EventLog::with_file(&path);
            log.record_seed(
                Coord {
                    t: 1,
                    c: 1,
                    o: 1,
                    v: 1,
                },
                "tx-001",
            );
            log.record_seed(
                Coord {
                    t: 2,
                    c: 1,
                    o: 1,
                    v: 1,
                },
                "tx-002",
            );
            log.record_seed(
                Coord {
                    t: 1,
                    c: 2,
                    o: 1,
                    v: 1,
                },
                "tx-003",
            );
            log.record_capacity(1, 100.0);
            assert_eq!(log.len(), 4);
        }

        // Session 2: read events back
        {
            let log = EventLog::with_file(&path);
            assert_eq!(log.len(), 4);

            let mut field = Field::new(4);
            log.replay(&mut field);

            // Events were replayed — field should have crystallizations
            assert!(
                field.crystallized_count() > 0,
                "Replayed field should have crystallizations"
            );
            assert_eq!(field.capacity(1), Some(100.0));
        }

        // Clean up
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn serialize_roundtrip() {
        let events = vec![
            PersistedEvent {
                id: "tx-1".into(),
                coord: Coord {
                    t: 1,
                    c: 2,
                    o: 3,
                    v: 4,
                },
                capacity_region: None,
                capacity_amount: None,
            },
            PersistedEvent {
                id: "cap".into(),
                coord: Coord {
                    t: 0,
                    c: 0,
                    o: 5,
                    v: 0,
                },
                capacity_region: Some(5),
                capacity_amount: Some(50.0),
            },
        ];

        let serialized = serialize_events(&events);
        let parsed = parse_events(&serialized);

        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].id, "tx-1");
        assert_eq!(parsed[0].coord.t, 1);
        assert_eq!(parsed[1].capacity_region, Some(5));
        assert_eq!(parsed[1].capacity_amount, Some(50.0));
    }
}
