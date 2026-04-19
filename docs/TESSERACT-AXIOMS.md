# Tesseract Field — Formal Axioms

> v0.1 — 17 abril 2026
> Status: draft, derived from experimental prototype

---

## 1. The Field

**Definition 1.1 (Field).** A Tesseract Field is a tuple **F** = (S, D, P, Θ, α, ρ) where:

- **S** ∈ ℕ is the size of each dimension
- **D** = 4 is the number of dimensions
- **P**: 𝕋⁴ → [0, 1] is the probability function over the toroidal 4D space
- **Θ** ∈ (0, 1) is the crystallization threshold
- **α** ∈ (0, 1) is the influence factor
- **ρ**: {0,1,2,3,4} → ℝ≥0 is the resonance function

**Definition 1.2 (Coordinate).** A coordinate **x** = (t, c, o, v) ∈ ℤₛ⁴ is a point in the 4D toroidal space, where ℤₛ = {0, 1, ..., S-1} with arithmetic modulo S.

The four dimensions are symmetric — no dimension is privileged:
- t — temporal
- c — context (channel)
- o — organization (identity)
- v — version (state)

**Axiom 1 (Dimensional symmetry).** All four dimensions are algebraically identical. No operation on the field distinguishes one dimension from another. Formally: for any permutation σ of {t,c,o,v}, the field F is isomorphic to the field obtained by applying σ to all coordinates.

---

## 2. Distance

**Definition 2.1 (Toroidal distance in one dimension).**

For a, b ∈ ℤₛ:

```
d₁(a, b, S) = min(|a - b|, S - |a - b|)
```

**Definition 2.2 (Euclidean distance in 4D toroidal space).**

For **x** = (t₁, c₁, o₁, v₁) and **y** = (t₂, c₂, o₂, v₂):

```
d(**x**, **y**) = √( d₁(t₁,t₂,S)² + d₁(c₁,c₂,S)² + d₁(o₁,o₂,S)² + d₁(v₁,v₂,S)² )
```

**Axiom 2 (Metric space).** (𝕋⁴, d) is a metric space. The distance function satisfies:
- d(**x**, **y**) ≥ 0 (non-negativity)
- d(**x**, **y**) = 0 ⟺ **x** = **y** (identity)
- d(**x**, **y**) = d(**y**, **x**) (symmetry)
- d(**x**, **z**) ≤ d(**x**, **y**) + d(**y**, **z**) (triangle inequality)

---

## 3. Cells and State

**Definition 3.1 (Cell).** A cell at coordinate **x** is a tuple C(**x**) = (p, κ, I) where:

- p ∈ [0, 1] — probability
- κ ∈ {0, 1} — crystallization flag (0 = fluid, 1 = crystallized)
- I = {(eᵢ, wᵢ)} — influence set: pairs of event identifiers and weights

**Definition 3.2 (Crystallization).** A cell crystallizes when:

```
κ(**x**) = 1  ⟺  p(**x**) ≥ Θ
```

Once crystallized, a cell is immutable under evolution:

**Axiom 3 (Crystallization irreversibility under evolution).** If κ(**x**) = 1 at time step n, then κ(**x**) = 1 for all subsequent evolution steps m > n. Crystallization can only be reversed by an external `destroy` operation (attack), not by the field's natural dynamics.

---

## 4. Events (Seeding)

**Definition 4.1 (Event).** An event E = (center, id) seeds a probability distribution across the entire field. For each cell **x** ∈ 𝕋⁴:

```
ΔP(**x**) = 1 / (1 + d(center, **x**))
```

The probability at **x** is updated:

```
p(**x**) ← min(p(**x**) + ΔP(**x**), 1)
```

And the influence is recorded:

```
I(**x**) ← I(**x**) ∪ {(id, ΔP(**x**))}
```

**Axiom 4 (Orbital completeness).** An event distributes non-zero probability to every cell within its effective radius. The decay function 1/(1+d) is theoretically non-zero for all finite d, but the implementation enforces a practical cutoff:

```
∀ **x** ∈ 𝕋⁴, ∀ E : ΔP(**x**) > 0   (theoretical)
∀ **x** ∈ 𝕋⁴, ∀ E : ΔP(**x**) ≥ ε    (stored — ε = 0.05, SEED_RADIUS = 4)
```

Cells where ΔP(**x**) < ε are not stored (sparse optimization). This bounds memory to O(R⁴) per event instead of O(S⁴), enabling fields up to S=32 (1M logical cells) with ~2.4% occupancy. The cutoff does not affect crystallization: at d=4 the decay yields p=0.20 >> ε, and at d=5 it yields p=0.167 >> ε. Cells beyond SEED_RADIUS have p < ε and cannot contribute meaningfully to crystallization (Θ=0.85).

> **Note:** The theoretical form (∀x: ΔP > 0) holds mathematically. The implementation narrows it to a bounded orbital. All proofs that depend on Axiom 4 use only the local neighborhood (d ≤ SEED_RADIUS), so the practical cutoff does not invalidate them.

**Axiom 5 (Additive overlap).** When multiple events seed the field, their contributions sum:

```
p(**x**) = min( Σᵢ ΔPᵢ(**x**), 1 )
```

This enables emergent crystallization at points where no single event has sufficient probability, but the sum of overlapping orbitals crosses Θ.

---

## 5. Neighbors and Orthogonal Support

**Definition 5.1 (Direct neighbors).** The direct neighbors of **x** are the 2D = 8 cells at distance 1 along each axis:

```
N(**x**) = { **y** ∈ 𝕋⁴ : ∃ exactly one dimension i where d₁(xᵢ, yᵢ, S) = 1, and xⱼ = yⱼ for j ≠ i }
```

**Definition 5.2 (Orthogonal support).** The orthogonal support σ(**x**) counts how many distinct axes have at least one neighbor with probability > 0.5:

```
σ(**x**) = |{ axis a ∈ {t,c,o,v} : ∃ **y** ∈ N(**x**) on axis a with p(**y**) > 0.5 }|
```

σ(**x**) ∈ {0, 1, 2, 3, 4}.

---

## 6. Evolution

**Definition 6.1 (Evolution step).** One evolution step updates all non-crystallized cells simultaneously:

For each **x** with κ(**x**) = 0:

```
μ(**x**) = (1/|N(**x**)|) · Σ_{**y** ∈ N(**x**)} p(**y**)          [neighbor average]

δ(**x**) = (μ(**x**) - p(**x**)) · α                                [influence delta]

(A, R) = ρ(σ(**x**))                                                [amplification, resonance]

p'(**x**) = clamp( p(**x**) + δ(**x**) · A + R,  0,  1 )           [new probability]
```

**Definition 6.2 (Resonance function).** The resonance function maps orthogonal support to amplification and resonance:

```
ρ(0) = ρ(1) = (1.0, 0.00)
ρ(2) = (1.5, 0.02)
ρ(3) = (2.5, 0.05)
ρ(4) = (4.0, 0.10)
```

**Axiom 6 (Simultaneous update).** All cells are updated simultaneously using the probabilities from the previous step. No cell sees the updated value of another cell within the same step. This prevents order-dependent behavior and ensures dimensional symmetry is preserved.

**Axiom 7 (Resonance drives convergence).** For a cell with σ(**x**) ≥ 2 and R > 0, the probability increases monotonically each step (assuming neighbors remain stable). This guarantees that any cell with sufficient orthogonal support will eventually crystallize:

```
σ(**x**) ≥ 2 ∧ R > 0 ⟹ ∃ n : p_n(**x**) ≥ Θ
```

---

## 7. Destruction (Attack Model)

**Definition 7.1 (Destroy).** An external destruction operation on cell **x**:

```
destroy(**x**): p(**x**) ← 0, κ(**x**) ← 0
```

The influence set I(**x**) is not cleared — the cell retains memory of what influenced it, but loses its probability and crystallization.

**Axiom 8 (Destruction is local).** Destroying a cell affects ONLY that cell. It does not modify the probability, crystallization, or influences of any other cell. Formally:

```
destroy(**x**) ⟹ ∀ **y** ≠ **x** : C(**y**) unchanged
```

---

## 8. Derived Properties (to prove as theorems)

The following properties were observed experimentally. Formal proofs are needed:

**Conjecture 8.1 (Self-healing).** If a crystallized cell **x** is destroyed, and its orthogonal support σ(**x**) ≥ 2 from still-crystallized neighbors, then there exists a finite number of evolution steps n such that **x** re-crystallizes.

**Conjecture 8.2 (Non-propagation of falsehood).** A cell crystallized by external force (not by evolution or seeding) at a point with no overlapping event orbitals does not cause additional crystallizations in its neighborhood beyond what the natural field evolution would produce.

**Conjecture 8.3 (Attack cost scaling).** In a field with K crystallized cells supported by events in D dimensions, the number of cells that must be simultaneously destroyed to permanently eliminate a state scales as O(K^(D-1)/D).

**Conjecture 8.4 (Emergent crystallization).** If two events E₁ and E₂ are seeded at centers **c₁** and **c₂** with d(**c₁**, **c₂**) ≤ d_max, then there exist points **x** between them where:

```
ΔP₁(**x**) + ΔP₂(**x**) ≥ Θ
```

These points crystallize without being seeded. The value d_max depends on Θ and the decay function.

**Conjecture 8.5 (Emergent resilience).** An emergent crystallization (Conjecture 8.4) self-heals after destruction with fewer evolution steps than a directly seeded cell, due to higher orthogonal support from the parent orbitals.

---

## 9. Notation Summary

| Symbol | Meaning |
|---|---|
| **F** | Tesseract Field |
| S | Size per dimension |
| D = 4 | Number of dimensions |
| 𝕋⁴ | 4D toroidal space ℤₛ⁴ |
| **x** | Coordinate (t,c,o,v) |
| d(**x**,**y**) | Euclidean distance in 𝕋⁴ |
| P(**x**), p(**x**) | Probability at **x** |
| κ(**x**) | Crystallization flag |
| I(**x**) | Influence set |
| Θ | Crystallization threshold (0.85) |
| α | Influence factor (0.15) |
| ρ | Resonance function |
| σ(**x**) | Orthogonal support (0-4 axes) |
| N(**x**) | Direct neighbors of **x** |
| E | Event (center, id) |
| μ(**x**) | Neighbor probability average |

---

## 10. Prototype parameter values

| Parameter | Symbol | Value | Rationale |
|---|---|---|---|
| Field size | S | 4 or 8 | Small for prototype; theory is size-independent |
| Dimensions | D | 4 | Minimum for tesseract; extensible to D>4 |
| Crystallization threshold | Θ | 0.85 | High enough to prevent noise crystallization, low enough for orbital overlap |
| Influence factor | α | 0.15 | Controls evolution speed; higher = faster convergence |
| Resonance at σ=4 | R₄ | 0.10 | Ensures full-support cells always reach Θ |
| Decay function | — | 1/(1+d) | Inverse linear; never zero; matches physical orbital decay |

---

*These axioms formalize the experimental prototype in `tesseract/src/lib.rs`. Conjectures 8.1-8.5 have experimental evidence (89 passing tests across 9 suites) and semi-formal proofs in TESSERACT-PROOFS.md.*
