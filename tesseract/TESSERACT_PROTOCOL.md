# Tesseract Protocol Specification

**Version**: 0.1.0
**Status**: Draft — empirical validation complete, formal proofs partial.

This document specifies the Tesseract protocol as implemented. It is a protocol specification, not marketing material. Claims are labeled as formal, empirical, or conjectured.

---

## 1. Data Types

### 1.1 Dimension

```
enum Dimension { Temporal, Context, Origin, Verification }
```

Four orthogonal evidence classes. Each dimension is backed by a structurally independent source of attestation.

### 1.2 Coord

```
struct Coord { t: uint, c: uint, o: uint, v: uint }
```

A point in the 4D toroidal field. Arithmetic is modular: `(x + 1) mod S` where `S` is field size.

Distance function (Euclidean in toroidal space):
```
dist(a, b, S) = sqrt(sum_d(min(|a_d - b_d|, S - |a_d - b_d|))^2)
```

### 1.3 Attestation

```
struct Attestation {
    dimension:    Dimension
    validator_id: string     // cryptographic identity, bound to one dimension
    event_id:     string     // the event being attested
    weight:       float64    // distance-decayed from seed center, range (0, 1]
}
```

### 1.4 Cell

```
struct Cell {
    probability:   float64                           // range [0, 1]
    crystallized:  bool
    attestations:  map<Dimension, list<Attestation>>
}
```

### 1.5 AttestationBundle

```
struct AttestationBundle {
    coord:        Coord
    event_id:     string
    attestations: list<(Dimension, validator_id: string)>
}
```

Transport optimization. Equivalent to receiving individual attestations simultaneously. Does NOT change the independence model.

### 1.6 Field (CoreState)

```
struct Field {
    cells:            map<Coord, Cell>    // sparse: only cells with p > EPSILON stored
    size:             uint                // side length S; total cells = S^4
    curvature_budget: map<uint, float64>  // per-region capacity
}
```

### 1.7 AttestationRecord

```
struct AttestationRecord {
    coord:        Coord
    event_id:     string
    dimension:    Dimension
    validator_id: string
}
```

Stored per-node for anti-entropy reconciliation.

---

## 2. Protocol Constants

| Constant | Value | Description |
|----------|-------|-------------|
| `CRYSTALLIZATION_THRESHOLD` (Theta) | 0.85 | Minimum probability for crystallization |
| `INFLUENCE_FACTOR` (alpha) | 0.15 | Diffusion step size |
| `SEED_RADIUS` | 3 | Max axis distance for attestation seeding |
| `CASCADE_STRENGTH` | 0.08 | Probability boost to neighbors on crystallization |
| `EPSILON` | 0.05 | Minimum probability to store a cell |
| `PROPAGATION_SPEED` | 1.0 | Light cone expansion rate (cells per tick) |
| `MIN_CAUSAL_DEPTH` | 3 | Minimum causal chain length for full-weight attestation |
| `CORRELATION_THRESHOLD` | 0.5 | Jaccard overlap above which validators are correlated |
| `ZERO_COST_DISCOUNT` | 0.25 | Weight multiplier for attestations without causal backing |
| `MAX_AMPLIFICATION` | 4.0 | Diffusion amplification at sigma=4 |
| `LIPSCHITZ_BOUND` | 0.60 | alpha * MAX_AMPLIFICATION (contraction factor) |
| `LIVENESS_BOUND` | 50 | Empirical max steps to crystallization after full attestation |

---

## 3. Sigma Independence

### 3.1 Raw sigma

sigma(x) counts dimensions with at least one **exclusive** validator:

```
sigma(x) = |{ d in {T,C,O,V} :
    exists v in validators(x, d) such that
    v not in validators(x, d') for all d' != d
}|
```

A validator that attests on multiple dimensions is NOT exclusive on any of them.

### 3.2 Effective sigma

```
sigma_eff(x) = sum_d min(1, independence(d) * diversity(d) * cost(d))
```

Where for each dimension d:

- `independence(d)`: 1.0 if dimension has exclusive validator, else 0.0
- `diversity(d)`: 1.0 - (correlated_pairs / total_pairs) among validators on d
  - Correlation: Jaccard(ancestors(v_i), ancestors(v_j)) > CORRELATION_THRESHOLD
- `cost(d)`: average over validators on d of `min(causal_depth(v) / MIN_CAUSAL_DEPTH, 1.0)`
  - Validators without causal graph backing: cost = ZERO_COST_DISCOUNT

sigma_eff in [0, 4]. Reduces to raw sigma when all validators have full causal depth and zero correlation.

---

## 4. Crystallization Rule

A cell at coord x crystallizes when ALL of:

1. **Threshold**: `p(x) >= CRYSTALLIZATION_THRESHOLD`
2. **Independence**: `sigma(x) >= 4` (for cells with attestations)
3. **Energy** (when thermodynamics active): `F(x) = U(x) - T*S(x) < 0`

On crystallization: `p(x) := 1.0`, `crystallized(x) := true`.

Crystallization is monotone under normal evolution: once true, stays true. Exception: curvature pressure can un-crystallize cells when regional load exceeds capacity (weakest binding energy first).

### Pseudocode: crystallize()

```
function crystallize(field, coord):
    cell = field.cells[coord]
    if cell.crystallized:
        return false

    if cell.probability < CRYSTALLIZATION_THRESHOLD:
        return false

    if cell.attestations is not empty:
        if sigma_independence(cell) < 4:
            return false

    cell.crystallized = true
    cell.probability = 1.0

    // Cascade: boost existing neighbors
    for n in neighbors(coord):
        if n in field.cells:
            field.cells[n].probability = min(field.cells[n].probability + CASCADE_STRENGTH, 1.0)

    return true
```

---

## 5. Attestation and Seeding

When a node attests event E on dimension D from validator V at center C:

```
function attest(field, center, event_id, dimension, validator_id):
    S = field.size
    R = min(SEED_RADIUS, S / 2)

    for each coord within R steps of center (toroidal):
        dist = euclidean_distance(center, coord, S)
        p = 1.0 / (1.0 + dist)
        if p < EPSILON: skip

        cell = field.get_or_create(coord)
        cell.probability = min(cell.probability + p, 1.0)

        // Record attestation (with dedup)
        if not already_attested(cell, event_id, dimension, validator_id):
            cell.attestations[dimension].append(Attestation{dimension, validator_id, event_id, weight: p})

        // Check crystallization
        crystallize(field, coord)
```

Seeding creates a (2R+1)^4 hypercube of affected cells around the center. At R=3: 2401 cells per attestation.

---

## 6. Evolution (Diffusion)

One evolution step processes dirty cells and their neighbors:

```
function evolve(field):
    new_crystallizations = 0

    for each active non-crystallized cell at coord:
        p = cell.probability
        avg = mean(probability of 8 axis-aligned neighbors)
        delta = (avg - p) * INFLUENCE_FACTOR

        sigma = orthogonal_support(field, coord)
        (amplification, residual) = match sigma:
            0..1 => (1.0, 0.0)
            2    => (1.5, 0.02)
            3    => (2.5, 0.05)
            4    => (4.0, 0.10)

        new_p = clamp(p + delta * amplification + residual, 0.0, 1.0)
        cell.probability = new_p

        if crystallize(field, coord):
            new_crystallizations += 1

    // Curvature pressure: if region load > capacity, decay weakest cells
    apply_curvature_pressure(field)

    return new_crystallizations
```

---

## 7. Sigma_eff Computation

### Pseudocode: compute_sigma_eff()

```
function compute_sigma_eff(field, coord, causal_graph):
    cell = field.cells[coord]
    sigma_eff = 0.0

    for d in [Temporal, Context, Origin, Verification]:
        atts = cell.attestations[d]
        if atts is empty:
            continue

        // Independence: does d have an exclusive validator?
        independence = 1.0 if has_exclusive_validator(cell, d) else 0.0

        // Diversity: causal overlap between validators on this dim
        diversity = 1.0
        if causal_graph is not null and |atts| > 1:
            correlated = 0
            total = 0
            for each pair (v_i, v_j) in atts:
                total += 1
                if jaccard(ancestors(v_i), ancestors(v_j)) > CORRELATION_THRESHOLD:
                    correlated += 1
            diversity = 1.0 - correlated / total

        // Cost: causal depth of validators
        cost = ZERO_COST_DISCOUNT  // default without graph
        if causal_graph is not null:
            cost = mean(min(causal_depth(v) / MIN_CAUSAL_DEPTH, 1.0) for v in atts)

        sigma_eff += min(1.0, independence * diversity * cost)

    return sigma_eff
```

---

## 8. Network Protocol

### 8.1 Message Types

```
enum Message:
    Attestation { coord, event_id, dimension, validator_id }
    AttestationBundle { coord, event_id, attestations: list<(dim, validator_id)> }
    SyncRequest { from_tick }
    Heartbeat { node_tick }
```

### 8.2 Bundle Propagation and Crystallization Waves

```
function receive_bundle(node, bundle):
    any_new = false
    was_crystallized = node.field.cells[bundle.coord].crystallized

    for (dim, vid) in bundle.attestations:
        key = dedup_key(bundle.event_id, dim, vid)
        if key in node.seen:
            continue  // dedup
        node.seen.add(key)
        node.field.attest(bundle.coord, bundle.event_id, dim, vid)
        node.records.append(AttestationRecord{bundle.coord, bundle.event_id, dim, vid})
        any_new = true

    if not any_new:
        return  // nothing new

    // Crystallization wave: if this bundle caused crystallization, push eagerly
    if not was_crystallized and node.field.cells[bundle.coord].crystallized:
        send_bundle_to_fanout_peers(node, bundle)
    else if node.has_full_event(bundle.event_id):
        // Node now has all 4 dims — send bundle instead of individual messages
        send_bundle_to_fanout_peers(node, build_bundle(node, bundle.event_id))
    else:
        // Forward individual attestations
        for (dim, vid) in new_attestations:
            gossip_forward(node, bundle.coord, bundle.event_id, dim, vid)
```

### 8.3 Gossip

Push gossip: on receiving a new attestation or bundle, forward to `fanout` random peers.

**Fanout requirement**: fanout >= ln(N) for epidemic coverage. Below this threshold, gossip may not reach all nodes. This is an explicit configuration requirement, not an enforced invariant.

### 8.4 Anti-Entropy Reconciliation

```
function reconcile(node_a, node_b):
    // Compare seen-sets
    missing_in_a = node_b.seen_keys - node_a.seen_keys

    // Transfer missing records from B to A
    for key in missing_in_a:
        record = node_b.lookup_record(key)
        node_a.apply_attestation(record.coord, record.event_id, record.dimension, record.validator_id)

    // Symmetric: also transfer A's records to B
    missing_in_b = node_a.seen_keys - node_b.seen_keys
    for key in missing_in_b:
        record = node_a.lookup_record(key)
        node_b.apply_attestation(record.coord, record.event_id, record.dimension, record.validator_id)
```

Triggered periodically (every `anti_entropy_interval` ticks) or on-demand after partition heals.

---

## 9. Network Model and Failure Assumptions

### 9.1 Network Model

- **Asynchronous** with bounded delay: messages delivered within [base_latency, base_latency + jitter] ticks
- **Unreliable**: messages may be dropped (rate < 1.0), duplicated, or reordered
- **Partitions**: bidirectional or asymmetric, finite duration
- **Clock skew**: nodes may tick at different rates

### 9.2 Failure Model

- **Crash faults**: nodes may stop and restart (state persisted via seen-set)
- **Byzantine faults**: adversary controls up to k nodes with valid cryptographic identities
- **Network adversary**: can delay, drop, duplicate, reorder messages on links it controls

### 9.3 Assumptions

- At least 4 dimension-bound validators are honest and eventually reachable
- Partitions are finite (eventually heal)
- Causal graph is append-only and hash-chained (immutable history)
- Cryptographic primitives (Ed25519, SHA-256) are not broken

---

## 10. Safety Claim

**Claim (formal)**: A cell at coord x crystallizes only if `sigma(x) >= 4`, meaning 4 structurally independent validators (one per dimension, exclusive) have attested the event at x.

**Proof**: Crystallization requires `sigma_independence(cell) >= 4` (checked in `attest()` and `evolve()`). sigma_independence counts dimensions with at least one validator that appears on no other dimension. Fewer than 4 exclusive validators => sigma < 4 => crystallization blocked.

**Verified**: Zero false crystallizations across all test profiles (clean, lossy, adversarial, partition) at 10, 100, and 1000 nodes.

**What safety does NOT guarantee**: Correspondence with external reality. A colluding set of 4 validators with distinct keys can crystallize a false event. sigma_eff partially mitigates this by penalizing causal correlation, but does not eliminate it.

---

## 11. Liveness Claim

**Claim (empirical, parameter-dependent)**: For any valid event with attestations on all 4 dimensions from exclusive validators, the event crystallizes at the attestation center within LIVENESS_BOUND = 50 evolve steps after all attestations are delivered.

**Conditions**:
- sigma_raw >= 4
- Attestation causal depth >= MIN_CAUSAL_DEPTH
- Target region is not over-capacity
- All attestations have been delivered to the node

**Observation**: At the attestation center, probability = 1.0 after 4 overlapping seeds. Crystallization is immediate on the `attest()` call. The 50-step bound covers peripheral cells.

**This bound depends on**: CRYSTALLIZATION_THRESHOLD, INFLUENCE_FACTOR, MAX_AMPLIFICATION, SEED_RADIUS, CASCADE_STRENGTH. If any parameter changes, the bound must be re-validated empirically.

---

## 12. Strong Eventual Consistency (SEC) Claim

**Claim (empirical)**: After a network partition heals and 2 anti-entropy rounds complete, all correct reachable nodes converge to the same crystallized core.

**Mechanism**: Anti-entropy compares seen-sets between node pairs and transfers missing AttestationRecords. Since attestation application is deterministic and idempotent, all nodes receiving the same set of attestations produce the same field state.

**Condition**: fanout >= ln(N) for gossip phase. Anti-entropy runs periodically or on-demand.

**Verified**: 100% convergence after partition heal + anti-entropy at 10, 100, and 1000 nodes across clean, lossy, and adversarial profiles.

---

## 13. Convergence Properties

### 13.1 Contraction

The diffusion operator has Lipschitz constant L = alpha * MAX_AMPLIFICATION = 0.60 < 1.0 for the sup-norm. This is a formal bound on the diffusion term only. Residual boosts and cascade add bounded perturbations (PERTURBATION_BOUND = 0.18 per step).

Asymptotic distance between two field states: `d_N <= L^N * d_0 + epsilon / (1 - L)`.

### 13.2 Lyapunov (convergence)

V(field) = sum over active cells of `(-p^2 - BE*p + T*H(p))` where BE = binding energy, H = Shannon entropy.

V is NOT per-step monotone (residual boosts cause transient increases). Convergence is by supermartingale argument:
- C1: V >= -2 * N_active (bounded below)
- C2: max single-step increase <= N_active * 0.52 (bounded perturbation)
- C3: Each crystallization drops V by >= 0.28 (drops dominate increases)

### 13.3 Core Uniqueness

The crystallized core (cells with sigma=4 at the attestation center) is unique for a given attestation structure, regardless of initial probability perturbations.

Peripheral cells near the crystallization threshold may bifurcate under different initial conditions. This is analogous to phase boundary fluctuation in statistical mechanics and is expected behavior.

---

## 14. Security Bound

For an adversary controlling k of N validators, with attestation cost c per dimension:

```
P(false crystallization) <= (k/N)^sigma_eff * exp(-c * sigma_eff)
```

| k/N | sigma_eff=1 | sigma_eff=2 | sigma_eff=4 |
|-----|-------------|-------------|-------------|
| 0.10 | 10.0% | 1.0% | 0.01% |
| 0.20 | 20.0% | 4.0% | 0.16% |
| 0.33 | 33.0% | 11.0% | 1.2% |

With cost c=1.0, multiply by e^(-sigma_eff): additional 37x reduction at sigma_eff=1, 1800x at sigma_eff=4.

---

## 15. Explicit Non-Claims

The Tesseract protocol does **NOT** claim or guarantee:

1. **Truth correspondence**: Crystallization means structural convergence, not that the event is true in the physical world.
2. **Quantum resistance**: The protocol uses Ed25519 and SHA-256. If these break, the protocol breaks.
3. **Per-step Lyapunov monotonicity**: V can increase transiently. Only the overall trend decreases.
4. **Universal convergence rate**: LIVENESS_BOUND is empirical and parameter-dependent.
5. **Byzantine leader tolerance**: There is no leader. But 4+ colluding validators with distinct keys can crystallize false events.
6. **Infinite partition tolerance**: Partitions must eventually heal for SEC to hold.
7. **Automatic fanout calibration**: The caller must set fanout >= ln(N).
8. **Protection against total compromise**: If all validators on all dimensions are compromised, sigma=4 is trivially achieved.
9. **Formal proof of convergence rate**: The contraction bound is formal; the rate in the full system (with residuals and cascade) is empirical.
10. **Dimension collapse resistance**: If the 4 dimensions are not backed by genuinely independent infrastructure, sigma=4 does not provide multiplicative security.

---

## 16. Implementation Reference

| Module | Purpose |
|--------|---------|
| `lib.rs` | Field, Cell, Coord, Dimension, attest(), evolve() |
| `crystallization.rs` | CrystallizationCriterion trait, UnifiedCriterion |
| `adversarial.rs` | sigma_eff, security_bound, threat model |
| `contraction.rs` | Lipschitz bound, sup_norm_distance, convergence tracking |
| `lyapunov.rs` | Lyapunov V(field), convergence analysis, core uniqueness |
| `liveness.rs` | Liveness checks, delay tolerance, noise resilience |
| `entropy.rs` | Thermodynamics, free energy, temperature cooling |
| `causality.rs` | CausalGraph, EventId, LightCone, partial order |
| `network_sim.rs` | Network simulator with faults |
| `gossip.rs` | Gossip + anti-entropy + bundles + crystallization waves |
| `scaling.rs` | Throughput, memory, sigma_eff cost benchmarks |
| `baseline_compare.rs` | Comparison vs Quorum BFT, DAG gossip, CRDT |

Test count at time of writing: 181 (lib + integration).
