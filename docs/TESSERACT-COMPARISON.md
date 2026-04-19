# Tesseract vs Nakamoto vs BFT — Formal Comparison

> v0.1 — 17 abril 2026
> Depends on: `TESSERACT-AXIOMS.md`, `TESSERACT-PROOFS.md`, `TESSERACT-CONVERGENCE.md`

---

## 1. The Three Models

### 1.1 Nakamoto Consensus (Bitcoin, 2008)

**Primitive:** Proof of Work — find nonce such that H(block || nonce) < target.

**Security assumption:** No single entity controls >50% of total hashrate.

**Finality:** Probabilistic. A block at depth k is reversed with probability:
```
P(reversal) ≈ (q/p)^k    where q = attacker hashrate, p = honest hashrate
```
Never reaches zero. 6 confirmations ≈ 99.9% for q < 0.1.

**State model:** Linear chain. One dimension (time). Total order of all transactions.

### 1.2 BFT Consensus (PBFT, 1999)

**Primitive:** Byzantine fault tolerance — 2f+1 of 3f+1 validators agree.

**Security assumption:** No more than f = ⌊(n-1)/3⌋ validators are Byzantine.

**Finality:** Deterministic after 3 communication rounds. A committed block is final.

**State model:** Linear chain with validator set. One dimension (time) + validator identity.

### 1.3 Tesseract Field (proposed)

**Primitive:** Convergence of probability distributions in 4D toroidal space.

**Security assumption:** None computational. Security derives from geometry.

**Finality:** Deterministic. Crystallization is a fixed point of a contraction mapping.

**State model:** 4D probability field. Four symmetric dimensions.

---

## 2. Security Model Comparison

### 2.1 What the attacker must do

| Model | Attack goal | Attack method |
|---|---|---|
| **Nakamoto** | Create longer chain | Outpace honest miners (>50% hashrate) |
| **BFT** | Create conflicting commit | Corrupt >1/3 validators |
| **Tesseract** | Destroy a crystallization permanently | Destroy all cells sustaining the orbital |

### 2.2 Attack cost

**Nakamoto:**
```
Cost = hashrate_fraction × time × energy_cost
```
Linear in hashrate. A 51% attack on Bitcoin costs ~$1-10B/hour (2026 estimates). The cost is **economic** — it has a price.

**BFT:**
```
Cost = compromise(⌈n/3⌉ + 1) validators
```
Linear in validator count. Requires social/technical compromise of specific entities. The cost is **operational** — it requires targeting specific actors.

**Tesseract:**
```
Cost = destroy(S^D) cells simultaneously
```
Exponential in dimensions. For S=32, D=4: ~1M cells must be destroyed simultaneously. But destroying cells is not enough — the orbital reconverges (Proof 1). For permanent destruction:
```
Cost = destroy(S^D) cells AND prevent all evolution steps indefinitely
```
The cost is **ontological** — it requires suspending the geometry of the space.

### 2.3 Cost scaling

| Model | Add nodes/power | Attack cost change |
|---|---|---|
| **Nakamoto** | Double hashrate | Attack cost doubles (linear) |
| **BFT** | Add n validators | Attack cost grows as n/3 (linear) |
| **Tesseract** | Increase S by 1 | Attack cost grows as (S+1)⁴/S⁴ ≈ 4S³ (polynomial per dimension, exponential in D) |

### 2.4 What survives the attack

| Model | During attack | After attack ends |
|---|---|---|
| **Nakamoto** | Chain reorganized, double-spends succeed | New chain is "true", old chain discarded permanently |
| **BFT** | Conflicting blocks committed (safety violation) | Manual intervention required, no automatic recovery |
| **Tesseract** | Cells destroyed, probability reduced | Field self-heals to original equilibrium automatically |

**Key difference:** Nakamoto and BFT attacks produce **permanent state changes**. Tesseract attacks produce **temporary perturbations** — the field returns to the same unique equilibrium.

---

## 3. Finality Comparison

### 3.1 Nakamoto finality

Probabilistic. Never absolute. The probability of reversal decreases exponentially with depth but never reaches zero:

```
P(final | k confirmations) = 1 - (q/p)^k
```

For q = 0.3 (30% attacker):
- 1 confirmation: 73% final
- 6 confirmations: 99.97% final
- 100 confirmations: ~100% final (but never exactly 100%)

### 3.2 BFT finality

Deterministic after commit. Once 2f+1 validators sign, the block is final. Period.

But: if >f validators are Byzantine, safety can be violated — two conflicting blocks can both be "final". The finality guarantee is conditional on the honest majority assumption.

### 3.3 Tesseract finality

Deterministic and unconditional. A crystallized cell is a fixed point of the evolution dynamics. It cannot be un-crystallized by evolution (Axiom 3). If destroyed, it reconverges to the same state (Convergence theorem, stability).

```
P(final | crystallized) = 1    (exact, not approximate)
```

No assumption about honest majorities. No assumption about computational power. The finality is a mathematical property of the field, not a social agreement.

| Property | Nakamoto | BFT | Tesseract |
|---|---|---|---|
| Finality type | Probabilistic | Deterministic | Deterministic |
| Conditioned on | Honest majority hashrate | Honest majority validators | Nothing |
| Can be reversed | Yes (with enough hashrate) | Yes (with >f Byzantine) | No (perturbations return to same state) |
| Time to finality | ~60 min (6 blocks) | ~1-3 seconds | Immediate upon crystallization |

---

## 4. Consensus Properties (CAP/FLP)

### 4.1 FLP Impossibility

The FLP theorem (1985) states: in an asynchronous system with even one faulty process, no deterministic protocol can guarantee consensus.

- **Nakamoto** sidesteps FLP by using probabilistic consensus (not deterministic).
- **BFT** sidesteps FLP by assuming partial synchrony (timeouts).
- **Tesseract** sidesteps FLP by not being a distributed protocol at all. The field is a mathematical object that evolves deterministically. There are no "processes" that can fail — there are probability distributions that converge. FLP does not apply to contraction mappings.

### 4.2 CAP Theorem

The CAP theorem states: a distributed system can provide at most 2 of 3: Consistency, Availability, Partition tolerance.

- **Nakamoto:** AP — available and partition-tolerant, eventual consistency.
- **BFT:** CP — consistent and partition-tolerant, unavailable during partition.
- **Tesseract:** AP — available and partition-tolerant, with eventual consistency. During a partition, each side evolves independently and may crystallize different states. Upon reconnection, the CRDT-like merge (max-probability, crystallization-wins, influence-union) preserves all crystallizations from both partitions. This is structurally equivalent to a state-based CRDT: always available, always partition-tolerant, eventually consistent after merge.

**Tesseract does NOT violate CAP.** It falls squarely in the AP category, like Nakamoto consensus but with a different convergence mechanism. The "strong consistency" (unique equilibrium per the convergence theorem) applies within a connected partition, not across a network split. After reconnection, the merged field converges to a new equilibrium that includes all events from both sides. Economic conflicts (e.g., double-spend across partitions) are resolved at the wallet layer (L2), not the field (L1).

---

## 5. Fault Tolerance

| Model | Fault type | Tolerance |
|---|---|---|
| **Nakamoto** | Crash faults | Any number (chain continues with remaining miners) |
| **Nakamoto** | Byzantine faults | <50% hashrate |
| **BFT** | Crash faults | <n/2 |
| **BFT** | Byzantine faults | <n/3 |
| **Tesseract** | Cell destruction | Any number (field self-heals) |
| **Tesseract** | False injection | Any number (no propagation without orthogonal support) |
| **Tesseract** | Total field destruction | Not tolerated (requires field to exist) |

**Tesseract's only vulnerability:** the field itself must exist. If all cells are destroyed simultaneously with no remaining probability, there is nothing to reconverge from. This is analogous to destroying every copy of a blockchain — the data is gone.

The difference: in blockchain, N copies must be independently destroyed. In the tesseract, the orbital distribution means every cell contains probability from every event — destroying "one copy" doesn't exist. You must destroy the entire field.

---

## 6. Computational Requirements

| Model | Per-block cost | Verification cost | Storage |
|---|---|---|---|
| **Nakamoto** | O(2^difficulty) hashes | O(1) hash check | O(chain_length) |
| **BFT** | O(n²) messages | O(n) signature checks | O(chain_length) |
| **Tesseract** | O(S^D) per evolution step | O(1) read cell | O(S^D) field state |

Tesseract's evolution step is expensive: O(S⁴) for each step. For S=32: ~1M operations per step. For S=256: ~4B operations per step.

**But:** evolution steps are local computations (each cell depends only on 8 neighbors). This is trivially parallelizable — each cell can be computed independently. On a GPU or distributed system, a step of a 256⁴ field could complete in milliseconds.

And critically: **evolution steps are not "consensus rounds"**. They are field dynamics. The field converges in 20-50 steps regardless of size (the convergence rate depends on α and R, not on S). A larger field does not need more steps — it needs more computation per step, which parallelizes perfectly.

---

## 7. Summary

### What Tesseract does that neither Nakamoto nor BFT can:

1. **Self-healing without protocol** — no backup, no redundant nodes, no state sync
2. **Unconditional finality** — not conditioned on honest majorities
3. **Automatic falsehood rejection** — no validation step required
4. **Emergent state** — new states arise from geometric proximity without explicit creation
5. **Attack resilience that doesn't consume resources** — recovery cost is zero

### What Tesseract does NOT have (yet):

1. **Production network protocol** — prototype has HTTP-based peer sync, not a production gossip layer
2. **Cryptographic identity binding** — org identity is hash-based, not signature-verified (see `mapper.rs`)
3. **Distributed double-spend resolution** — field accepts all events; economic conflicts resolved at wallet layer (L2)
4. **Production-scale validation** — largest test is 32⁴ = 1,048,576 cells; real-world load testing pending

### The fundamental difference:

```
Nakamoto/BFT: security is a PROTOCOL PROPERTY — it depends on rules being followed
Tesseract:    security is a GEOMETRIC PROPERTY — it depends on the shape of the space
```

Protocols can be broken by breaking the rules. Geometry cannot be broken by breaking rules — it is not a rule. It is the structure of the space itself.

---

*Phase 1 (Formalization) complete. Documents:*
- *TESSERACT-CONSENSUS.md — conceptual thesis (24 sections)*
- *TESSERACT-AXIOMS.md — 8 axioms, 5 conjectures*
- *TESSERACT-PROOFS.md — 5 proofs*
- *TESSERACT-CONVERGENCE.md — convergence, uniqueness, stability theorems*
- *TESSERACT-COMPARISON.md — formal comparison with Nakamoto and BFT*

*Prototype has scaled to 32⁴ = 1,048,576 cells. Next: production network protocol and real-world load testing.*
