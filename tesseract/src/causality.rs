//! Causality — light cones and partial order for the field.
//!
//! Core idea: an event can only influence what it could have reached.
//! Two events that cannot see each other are CONCURRENT — no ordering needed.
//! This replaces blockchain's total order with a partial order derived from
//! physics: nothing travels faster than the speed of light (propagation bound).
//!
//! Key types:
//!   - `EventId` — unique hash-based identifier
//!   - `CausalEvent` — an observation anchored in the field with a logical clock
//!   - `LightCone` — the set of coordinates reachable from an event
//!   - `CausalGraph` — partial order of events (DAG of causality)

use std::collections::{HashMap, HashSet};

use sha2::{Digest, Sha256};

use crate::Coord;

/// Unique event identifier derived from content hash.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct EventId(pub [u8; 32]);

impl EventId {
    pub fn from_content(data: &[u8]) -> Self {
        let hash = Sha256::digest(data);
        Self(hash.into())
    }

    pub fn short(&self) -> String {
        hex::encode(&self.0[..4])
    }
}

impl std::fmt::Display for EventId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode(&self.0[..8]))
    }
}

/// Propagation speed: how far an event's influence can reach per tick.
/// Like the speed of light — a hard upper bound, not a suggestion.
pub const PROPAGATION_SPEED: f64 = 1.0;

/// A causal event: something that happened at a place and time in the field.
#[derive(Clone, Debug)]
pub struct CausalEvent {
    pub id: EventId,
    /// Where in the field this event originated.
    pub origin: Coord,
    /// Logical timestamp (Lamport-like). Monotonically increasing per observer.
    pub logical_time: u64,
    /// Events this one directly depends on (has seen).
    pub parents: Vec<EventId>,
    /// Opaque payload.
    pub data: Vec<u8>,
}

impl CausalEvent {
    pub fn new(origin: Coord, logical_time: u64, parents: Vec<EventId>, data: Vec<u8>) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(origin.t.to_le_bytes());
        hasher.update(origin.c.to_le_bytes());
        hasher.update(origin.o.to_le_bytes());
        hasher.update(origin.v.to_le_bytes());
        hasher.update(logical_time.to_le_bytes());
        for p in &parents {
            hasher.update(p.0);
        }
        hasher.update(&data);
        let id = EventId(hasher.finalize().into());

        Self {
            id,
            origin,
            logical_time,
            parents,
            data,
        }
    }
}

/// Light cone: the causal boundary of an event.
/// An event at origin with logical_time `t0` can influence coordinate `c`
/// only if the field-distance from origin to `c` ≤ (now - t0) * PROPAGATION_SPEED.
///
/// This is the fundamental constraint: no action at a distance.
#[derive(Clone, Debug)]
pub struct LightCone {
    pub origin: Coord,
    pub birth_time: u64,
}

impl LightCone {
    pub fn new(origin: Coord, birth_time: u64) -> Self {
        Self { origin, birth_time }
    }

    /// Can this event's light cone reach `target` at `current_time`?
    /// Uses the field's toroidal distance.
    pub fn can_reach(&self, target: Coord, current_time: u64, field_size: usize) -> bool {
        if current_time < self.birth_time {
            return false;
        }
        let elapsed = (current_time - self.birth_time) as f64;
        let max_reach = elapsed * PROPAGATION_SPEED;
        let dist = crate::distance(self.origin, target, field_size);
        dist <= max_reach
    }

    /// Radius of influence at a given time.
    pub fn radius_at(&self, current_time: u64) -> f64 {
        if current_time < self.birth_time {
            return 0.0;
        }
        (current_time - self.birth_time) as f64 * PROPAGATION_SPEED
    }
}

/// Causal relationship between two events.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CausalOrder {
    /// A happened before B (A is in B's past light cone).
    Before,
    /// B happened before A.
    After,
    /// Neither can see the other — they are independent.
    Concurrent,
}

/// The causal graph: a DAG of events with partial ordering.
/// This replaces blockchain's total order (longest chain) with
/// physics-derived partial order (what can see what).
pub struct CausalGraph {
    events: HashMap<EventId, CausalEvent>,
    /// For each event, the set of all ancestors (transitive closure of parents).
    ancestors: HashMap<EventId, HashSet<EventId>>,
    /// Light cone for each event.
    cones: HashMap<EventId, LightCone>,
    /// Current logical time (global tick counter for the field).
    pub current_time: u64,
}

impl CausalGraph {
    pub fn new() -> Self {
        Self {
            events: HashMap::new(),
            ancestors: HashMap::new(),
            cones: HashMap::new(),
            current_time: 0,
        }
    }

    /// Insert an event into the causal graph.
    /// Returns false if any parent is unknown (causal violation).
    pub fn insert(&mut self, event: CausalEvent) -> bool {
        // Verify all parents exist
        for parent_id in &event.parents {
            if !self.events.contains_key(parent_id) {
                return false;
            }
        }

        // Build ancestor set: union of all parents' ancestors + parents themselves
        let mut all_ancestors = HashSet::new();
        for parent_id in &event.parents {
            all_ancestors.insert(parent_id.clone());
            if let Some(parent_ancestors) = self.ancestors.get(parent_id) {
                all_ancestors.extend(parent_ancestors.iter().cloned());
            }
        }

        let cone = LightCone::new(event.origin, event.logical_time);
        let id = event.id.clone();

        self.events.insert(id.clone(), event);
        self.ancestors.insert(id.clone(), all_ancestors);
        self.cones.insert(id.clone(), cone);

        // Advance global time
        if let Some(ev) = self.events.get(&id) {
            if ev.logical_time >= self.current_time {
                self.current_time = ev.logical_time + 1;
            }
        }

        true
    }

    /// Determine the causal relationship between two events.
    pub fn order(&self, a: &EventId, b: &EventId) -> CausalOrder {
        if a == b {
            return CausalOrder::Before; // reflexive
        }

        let b_ancestors = self.ancestors.get(b);
        let a_ancestors = self.ancestors.get(a);

        let a_before_b = b_ancestors.map(|anc| anc.contains(a)).unwrap_or(false);
        let b_before_a = a_ancestors.map(|anc| anc.contains(b)).unwrap_or(false);

        match (a_before_b, b_before_a) {
            (true, false) => CausalOrder::Before,
            (false, true) => CausalOrder::After,
            _ => CausalOrder::Concurrent,
        }
    }

    /// Get all events concurrent with a given event.
    /// These are events that exist in different "branches" of reality
    /// and don't need to be ordered — like spacelike-separated events in physics.
    pub fn concurrent_with(&self, id: &EventId) -> Vec<&EventId> {
        self.events
            .keys()
            .filter(|other| *other != id && self.order(id, other) == CausalOrder::Concurrent)
            .collect()
    }

    /// Can event `source` causally influence coordinate `target` right now?
    pub fn can_influence(&self, source: &EventId, target: Coord, field_size: usize) -> bool {
        self.cones
            .get(source)
            .map(|cone| cone.can_reach(target, self.current_time, field_size))
            .unwrap_or(false)
    }

    /// Get the "causal depth" of an event — longest path from any root.
    /// Analogous to block height, but partial-order aware.
    pub fn depth(&self, id: &EventId) -> usize {
        let event = match self.events.get(id) {
            Some(e) => e,
            None => return 0,
        };

        if event.parents.is_empty() {
            return 0;
        }

        event
            .parents
            .iter()
            .map(|p| self.depth(p) + 1)
            .max()
            .unwrap_or(0)
    }

    /// All events that have no dependents (tips of the DAG).
    /// Analogous to "unconfirmed transactions" — the frontier.
    pub fn tips(&self) -> Vec<&EventId> {
        let all_parents: HashSet<&EventId> = self
            .events
            .values()
            .flat_map(|e| e.parents.iter())
            .collect();

        self.events
            .keys()
            .filter(|id| !all_parents.contains(id))
            .collect()
    }

    pub fn event(&self, id: &EventId) -> Option<&CausalEvent> {
        self.events.get(id)
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Iterator over all event IDs in the graph.
    /// Used by gravity to compute mass from the graph itself.
    pub fn all_event_ids(&self) -> impl Iterator<Item = &EventId> {
        self.events.keys()
    }

    /// Get the ancestor set of an event (transitive closure of parents).
    /// Used by adversarial module to detect causal correlation between validators.
    pub fn ancestors_of(&self, id: &EventId) -> Option<&HashSet<EventId>> {
        self.ancestors.get(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn coord(t: usize, c: usize, o: usize, v: usize) -> Coord {
        Coord { t, c, o, v }
    }

    fn genesis() -> CausalEvent {
        CausalEvent::new(coord(0, 0, 0, 0), 0, vec![], b"genesis".to_vec())
    }

    #[test]
    fn genesis_event_has_no_parents() {
        let mut graph = CausalGraph::new();
        let g = genesis();
        let id = g.id.clone();
        assert!(graph.insert(g));
        assert_eq!(graph.depth(&id), 0);
        assert_eq!(graph.tips().len(), 1);
    }

    #[test]
    fn child_sees_parent_as_before() {
        let mut graph = CausalGraph::new();
        let g = genesis();
        let gid = g.id.clone();
        graph.insert(g);

        let child = CausalEvent::new(coord(1, 0, 0, 0), 1, vec![gid.clone()], b"child".to_vec());
        let cid = child.id.clone();
        graph.insert(child);

        assert_eq!(graph.order(&gid, &cid), CausalOrder::Before);
        assert_eq!(graph.order(&cid, &gid), CausalOrder::After);
        assert_eq!(graph.depth(&cid), 1);
    }

    #[test]
    fn independent_events_are_concurrent() {
        let mut graph = CausalGraph::new();
        let g = genesis();
        let gid = g.id.clone();
        graph.insert(g);

        // Two children of genesis that don't know about each other
        let a = CausalEvent::new(
            coord(1, 0, 0, 0),
            1,
            vec![gid.clone()],
            b"branch_a".to_vec(),
        );
        let b = CausalEvent::new(
            coord(0, 1, 0, 0),
            1,
            vec![gid.clone()],
            b"branch_b".to_vec(),
        );
        let aid = a.id.clone();
        let bid = b.id.clone();
        graph.insert(a);
        graph.insert(b);

        assert_eq!(graph.order(&aid, &bid), CausalOrder::Concurrent);

        let concurrent = graph.concurrent_with(&aid);
        assert!(concurrent.contains(&&bid));
    }

    #[test]
    fn merge_resolves_concurrency() {
        let mut graph = CausalGraph::new();
        let g = genesis();
        let gid = g.id.clone();
        graph.insert(g);

        let a = CausalEvent::new(coord(1, 0, 0, 0), 1, vec![gid.clone()], b"a".to_vec());
        let b = CausalEvent::new(coord(0, 1, 0, 0), 1, vec![gid.clone()], b"b".to_vec());
        let aid = a.id.clone();
        let bid = b.id.clone();
        graph.insert(a);
        graph.insert(b);

        // Merge event sees both branches
        let merge = CausalEvent::new(
            coord(2, 1, 0, 0),
            2,
            vec![aid.clone(), bid.clone()],
            b"merge".to_vec(),
        );
        let mid = merge.id.clone();
        graph.insert(merge);

        // Merge is after both
        assert_eq!(graph.order(&aid, &mid), CausalOrder::Before);
        assert_eq!(graph.order(&bid, &mid), CausalOrder::Before);
        assert_eq!(graph.depth(&mid), 2);
    }

    #[test]
    fn light_cone_respects_propagation_speed() {
        let cone = LightCone::new(coord(5, 5, 5, 5), 0);

        // At time 0, can only reach origin
        assert!(cone.can_reach(coord(5, 5, 5, 5), 0, 20));
        // 1 unit away needs at least 1 tick
        assert!(!cone.can_reach(coord(6, 5, 5, 5), 0, 20));
        assert!(cone.can_reach(coord(6, 5, 5, 5), 1, 20));

        // Diagonal distance = sqrt(4) = 2.0, needs at least 2 ticks
        assert!(!cone.can_reach(coord(6, 6, 6, 6), 1, 20));
        assert!(cone.can_reach(coord(6, 6, 6, 6), 2, 20));
    }

    #[test]
    fn reject_event_with_unknown_parent() {
        let mut graph = CausalGraph::new();
        let fake_parent = EventId::from_content(b"nonexistent");
        let orphan = CausalEvent::new(coord(0, 0, 0, 0), 0, vec![fake_parent], b"orphan".to_vec());
        assert!(!graph.insert(orphan));
    }

    #[test]
    fn tips_excludes_referenced_events() {
        let mut graph = CausalGraph::new();
        let g = genesis();
        let gid = g.id.clone();
        graph.insert(g);

        let child = CausalEvent::new(coord(1, 0, 0, 0), 1, vec![gid.clone()], b"c".to_vec());
        let cid = child.id.clone();
        graph.insert(child);

        let tips = graph.tips();
        assert_eq!(tips.len(), 1);
        assert!(tips.contains(&&cid));
        // Genesis is no longer a tip
        assert!(!tips.contains(&&gid));
    }

    #[test]
    fn transitive_ancestry() {
        let mut graph = CausalGraph::new();
        let g = genesis();
        let gid = g.id.clone();
        graph.insert(g);

        let a = CausalEvent::new(coord(1, 0, 0, 0), 1, vec![gid.clone()], b"a".to_vec());
        let aid = a.id.clone();
        graph.insert(a);

        let b = CausalEvent::new(coord(2, 0, 0, 0), 2, vec![aid.clone()], b"b".to_vec());
        let bid = b.id.clone();
        graph.insert(b);

        // Genesis is before b (transitively through a)
        assert_eq!(graph.order(&gid, &bid), CausalOrder::Before);
    }

    // ── Property-based tests ─────────────────────────────────────────────

    use proptest::prelude::*;

    fn arb_coord(max: usize) -> impl Strategy<Value = Coord> {
        (0..max, 0..max, 0..max, 0..max).prop_map(|(t, c, o, v)| Coord { t, c, o, v })
    }

    proptest! {
        #![proptest_config(proptest::test_runner::Config::with_cases(100))]

        /// EventId is deterministic: same inputs → same id.
        #[test]
        fn event_id_deterministic(
            t in 0..20usize, c in 0..20usize, o in 0..20usize, v in 0..20usize,
            time in 0..100u64,
            data in proptest::collection::vec(any::<u8>(), 0..64),
        ) {
            let coord = Coord { t, c, o, v };
            let e1 = CausalEvent::new(coord, time, vec![], data.clone());
            let e2 = CausalEvent::new(coord, time, vec![], data);
            prop_assert_eq!(e1.id, e2.id);
        }

        /// Different data → different EventId (collision resistance).
        #[test]
        fn event_id_collision_resistant(
            t in 0..20usize,
            d1 in proptest::collection::vec(any::<u8>(), 1..64),
            d2 in proptest::collection::vec(any::<u8>(), 1..64),
        ) {
            prop_assume!(d1 != d2);
            let coord = Coord { t, c: 0, o: 0, v: 0 };
            let e1 = CausalEvent::new(coord, 0, vec![], d1);
            let e2 = CausalEvent::new(coord, 0, vec![], d2);
            prop_assert_ne!(e1.id, e2.id);
        }

        /// Causal order is antisymmetric: if A < B then B > A.
        #[test]
        fn causal_order_antisymmetric(chain_len in 2..8usize) {
            let mut graph = CausalGraph::new();
            let g = CausalEvent::new(coord(0,0,0,0), 0, vec![], b"g".to_vec());
            let mut last = g.id.clone();
            graph.insert(g);

            for i in 1..chain_len {
                let ev = CausalEvent::new(
                    coord(i, 0, 0, 0), i as u64,
                    vec![last.clone()], format!("e{i}").into_bytes(),
                );
                last = ev.id.clone();
                graph.insert(ev);
            }

            let first = graph.all_event_ids().next().unwrap().clone();
            if first != last {
                let order_ab = graph.order(&first, &last);
                let order_ba = graph.order(&last, &first);
                match order_ab {
                    CausalOrder::Before => prop_assert_eq!(order_ba, CausalOrder::After),
                    CausalOrder::After => prop_assert_eq!(order_ba, CausalOrder::Before),
                    CausalOrder::Concurrent => prop_assert_eq!(order_ba, CausalOrder::Concurrent),
                }
            }
        }

        /// Light cone: origin is always reachable at emission time.
        #[test]
        fn light_cone_origin_always_reachable(
            origin in arb_coord(20), t0 in 0..50u64, size in 5..30usize
        ) {
            let cone = LightCone::new(origin, t0);
            prop_assert!(cone.can_reach(origin, t0, size));
        }

        /// Light cone monotonicity: if reachable at time T, also reachable at T+1.
        #[test]
        fn light_cone_monotonic(
            origin in arb_coord(15), target in arb_coord(15),
            t0 in 0..20u64, dt in 1..30u64, size in 8..20usize,
        ) {
            let cone = LightCone::new(origin, t0);
            let now = t0 + dt;
            if cone.can_reach(target, now, size) {
                prop_assert!(cone.can_reach(target, now + 1, size));
            }
        }
    }
}
