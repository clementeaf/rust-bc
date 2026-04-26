//! Node: a participant in the tesseract network.
//!
//! Each node owns a region of the 4D field and exchanges
//! boundary cells with neighboring nodes. The field is
//! distributed — no single node holds the entire space.

use crate::contribution::ContributionMetrics;
use crate::persistence::EventLog;
use crate::{Cell, Coord, Field, SEED_RADIUS};

/// A region of the 4D field owned by a node.
#[derive(Clone, Debug)]
pub struct Region {
    /// Inclusive start coordinate per axis.
    pub start: [usize; 4],
    /// Exclusive end coordinate per axis.
    pub end: [usize; 4],
}

impl Region {
    pub fn contains(&self, coord: Coord) -> bool {
        let axes = [coord.t, coord.c, coord.o, coord.v];
        (0..4).all(|i| axes[i] >= self.start[i] && axes[i] < self.end[i])
    }
}

/// A node in the tesseract network.
/// Persists events locally as cache. Recovers from neighbors on restart.
pub struct Node {
    pub id: String,
    pub field: Field,
    pub region: Region,
    pub metrics: ContributionMetrics,
    log: EventLog,
}

impl Node {
    /// Create a node with in-memory only log (no persistence).
    pub fn new(id: impl Into<String>, field_size: usize, region: Region) -> Self {
        Self {
            id: id.into(),
            field: Field::new(field_size),
            region,
            metrics: ContributionMetrics::default(),
            log: EventLog::new(),
        }
    }

    /// Create a node with file-backed persistence.
    /// On creation, replays existing events from disk (cache recovery).
    pub fn with_persistence(
        id: impl Into<String>,
        field_size: usize,
        region: Region,
        path: &str,
    ) -> Self {
        let log = EventLog::with_file(path);
        let mut field = Field::new(field_size);
        log.replay(&mut field);
        Self {
            id: id.into(),
            field,
            region,
            metrics: ContributionMetrics::default(),
            log,
        }
    }

    /// Seed an event on this node. Persists to local log automatically.
    pub fn seed(&mut self, coord: Coord, event_id: &str) {
        self.field.seed_named(coord, event_id);
        self.log.record_seed(coord, event_id);
        self.metrics.events_processed += 1;
    }

    /// Number of persisted events.
    pub fn event_count(&self) -> usize {
        self.log.len()
    }

    /// Recover field state from neighbor boundary cells.
    /// Used when local cache is lost — neighbors provide the truth.
    /// The sending node earns a `recoveries_assisted` credit.
    pub fn recover_from_neighbors(&mut self, neighbor_cells: &[(Coord, Cell)]) {
        self.receive_boundary(neighbor_cells);
    }

    /// Evolve the local field.
    pub fn evolve(&mut self) -> usize {
        self.metrics.cells_maintained = self.field.active_cells() as u64;
        self.field.evolve()
    }

    /// Extract boundary cells: cells near the edge of this node's region
    /// that should be shared with neighbors.
    pub fn boundary_cells(&self) -> Vec<(Coord, Cell)> {
        let margin = SEED_RADIUS;
        let mut result = Vec::new();

        for (coord, cell) in self.field.active_entries() {
            let axes = [coord.t, coord.c, coord.o, coord.v];
            let near_edge = (0..4).any(|i| {
                axes[i] < self.region.start[i] + margin
                    || axes[i] >= self.region.end[i].saturating_sub(margin)
            });
            if near_edge {
                result.push((coord, cell.clone()));
            }
        }
        result
    }

    /// Receive boundary cells from a neighbor node.
    ///
    /// Merge semantics (CRDT-like, partition-safe):
    /// - Probability: max(local, remote) — higher context wins.
    /// - Crystallization: once crystallized, never un-crystallized by merge.
    /// - Influences: union by event_id — no duplicates, no deletions.
    ///
    /// After a network partition, both sides may have crystallized different
    /// events. This merge accepts ALL crystallizations from both partitions.
    /// Contradictory states (e.g., double-spend) coexist at the field level.
    /// Resolution of economic conflicts is the wallet layer's responsibility
    /// (L2), not the field's (L1). See docs/ISSUE.md for the design rationale.
    pub fn receive_boundary(&mut self, cells: &[(Coord, Cell)]) {
        for (coord, remote_cell) in cells {
            let local = self.field.get_mut(*coord);
            // Take the higher probability (remote may have more context)
            if remote_cell.probability > local.probability {
                local.probability = remote_cell.probability;
            }
            // Merge crystallization state
            if remote_cell.crystallized && !local.crystallized {
                local.crystallized = true;
                local.probability = 1.0;
            }
            // Merge influences (avoid duplicates by event_id)
            for inf in &remote_cell.influences {
                let already_has = local.influences.iter().any(|i| i.event_id == inf.event_id);
                if !already_has {
                    local.influences.push(inf.clone());
                }
            }
        }
        self.metrics.boundary_exchanges += 1;
    }
}

/// A network of tesseract nodes.
pub struct Network {
    pub nodes: Vec<Node>,
    pub field_size: usize,
}

impl Network {
    /// Create a network that partitions the t-axis among N nodes.
    /// Each node owns a slice of t, full range of c/o/v.
    pub fn new(field_size: usize, num_nodes: usize) -> Self {
        let slice = field_size / num_nodes;
        let nodes = (0..num_nodes)
            .map(|i| {
                let start_t = i * slice;
                let end_t = if i == num_nodes - 1 {
                    field_size
                } else {
                    (i + 1) * slice
                };
                Node::new(
                    format!("node-{}", i),
                    field_size,
                    Region {
                        start: [start_t, 0, 0, 0],
                        end: [end_t, field_size, field_size, field_size],
                    },
                )
            })
            .collect();

        Self { nodes, field_size }
    }

    /// Seed an event on the node whose region contains the coordinate.
    pub fn seed(&mut self, coord: Coord, event_id: &str) {
        for node in &mut self.nodes {
            if node.region.contains(coord) {
                node.seed(coord, event_id);
                return;
            }
        }
        // Fallback: seed on first node
        self.nodes[0].seed(coord, event_id);
    }

    /// Distributed seed: multiple parties seed the same event from their
    /// respective nodes. Each party seeds on their own node; boundary
    /// exchange propagates the orbitals. The overlapping orbitals from
    /// multiple nodes reinforce the event — like multiple witnesses
    /// observing the same phenomenon from different positions.
    ///
    /// `parties` contains (node_index, party_label) pairs.
    pub fn distributed_seed(&mut self, coord: Coord, event_id: &str, parties: &[(usize, &str)]) {
        for (node_idx, party) in parties {
            if *node_idx < self.nodes.len() {
                let label = format!("{}[{}]", event_id, party);
                self.nodes[*node_idx].seed(coord, &label);
            }
        }
    }

    /// One round of: evolve all nodes + exchange boundaries.
    pub fn step(&mut self) -> usize {
        // 1. Evolve all nodes independently
        let mut total_new = 0;
        for node in &mut self.nodes {
            total_new += node.evolve();
        }

        // 2. Exchange boundary cells between all pairs
        let n = self.nodes.len();
        for i in 0..n {
            for j in (i + 1)..n {
                // Extract boundaries
                let boundary_i = self.nodes[i].boundary_cells();
                let boundary_j = self.nodes[j].boundary_cells();

                // Exchange
                self.nodes[i].receive_boundary(&boundary_j);
                self.nodes[j].receive_boundary(&boundary_i);
            }
        }

        total_new
    }

    /// Run until equilibrium across all nodes.
    pub fn run_to_equilibrium(&mut self, stable_for: usize) {
        let mut stable = 0;
        for _ in 1..=500 {
            if self.step() == 0 {
                stable += 1;
            } else {
                stable = 0;
            }
            if stable >= stable_for {
                break;
            }
        }
    }

    /// Query a cell from whichever node has it.
    pub fn get(&self, coord: Coord) -> &Cell {
        for node in &self.nodes {
            if node.region.contains(coord) {
                return node.field.get(coord);
            }
        }
        self.nodes[0].field.get(coord)
    }

    /// Simulate a node dying and recovering from neighbors.
    /// Clears the node's field entirely, then recovers from
    /// boundary exchange with other nodes in the network.
    pub fn simulate_node_recovery(&mut self, node_idx: usize) {
        // 1. Node dies — field is empty
        let size = self.field_size;
        let region = self.nodes[node_idx].region.clone();
        let id = self.nodes[node_idx].id.clone();
        self.nodes[node_idx] = Node::new(id, size, region);

        // 2. All other nodes send their boundary cells to the recovered node
        let n = self.nodes.len();
        for i in 0..n {
            if i == node_idx {
                continue;
            }
            let boundary = self.nodes[i].boundary_cells();
            self.nodes[node_idx].recover_from_neighbors(&boundary);
            self.nodes[i].metrics.recoveries_assisted += 1;
        }

        // 3. Evolve to let the recovered node stabilize
        self.run_to_equilibrium(10);
    }
}
