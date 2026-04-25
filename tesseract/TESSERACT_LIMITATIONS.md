# Assumptions and Limitations

This document derives the security limitations of Tesseract directly from its formal theorems (T1–T6, T2', T3', T4') and protocol definitions. Each limitation is traced to the specific assumption it depends on, with a concrete counterexample showing what happens when the assumption fails.

---

## 1. Minimal Assumptions

The following assumptions are **necessary and sufficient** for the security claims of Tesseract. Removing any one of them invalidates the corresponding theorem.

### A1. Bounded Field (S finite)

**Required by**: T1 (contraction), T2/T2' (convergence), T5 (uniqueness), T6 (liveness).

**What it provides**: A finite state space ensures that the crystallized set K_n stabilizes in finite time, the diffusion operator has finite Lipschitz constant, and the Lyapunov function V is bounded.

**If violated**: S = ∞ would make V unbounded below, the crystallized set could grow without bound, and convergence is not guaranteed. The field would require infinite memory.

**Assessment**: This assumption is trivially satisfied in any implementation. Not a practical limitation.

### A2. Deterministic Attestation Application

**Required by**: T4/T4' (SEC), T5 (uniqueness).

**What it provides**: Given the same set of attestations applied in any order, all correct nodes produce the same field state. This is the foundation of SEC: identical inputs → identical outputs.

**If violated**: Non-deterministic `attest()` (e.g., depending on floating-point ordering, thread scheduling, or system clock) would cause correct nodes to diverge even with identical seen-sets.

**Counterexample**: Suppose `attest()` uses the system clock to seed random perturbation. Node A processes attestations at t=100, Node B at t=200. They receive the same attestations but produce different probabilities. Core uniqueness (T5) and SEC (T4') both fail.

**Assessment**: Satisfied by construction — `attest()` uses only its arguments, no external state. Must be verified in any reimplementation.

### A3. Honest Dimension Coverage (≥ 4 exclusive validators)

**Required by**: T3/T3' (security bound), T6 (liveness).

**What it provides**: At least one honest validator per dimension ensures that valid events can achieve σ = 4 and crystallize. The security bound assumes the honest majority is distributed across all 4 dimensions.

**If violated (dimension starvation)**: If honest validators cover only 3 of 4 dimensions, no valid event can reach σ = 4. The system is safe (nothing false crystallizes) but not live (nothing true crystallizes either).

**Counterexample**: An adversary controls all validators on the Verification dimension. Honest validators cover T, C, O. Every event achieves at most σ = 3. The system is frozen — maximum safety, zero liveness.

**If violated (dimension collapse)**: If two "dimensions" are backed by the same physical infrastructure (e.g., Temporal and Context both use the same clock source), a single infrastructure failure compromises two dimensions simultaneously. The effective independence is 2, not 4. The security bound degrades from (k/N)^4 to (k/N)^2.

**Counterexample**: Temporal validators use NTP server X. Context validators also depend on NTP server X. Adversary compromises X → controls both dimensions. Effective σ = 2, not 4. Security bound is (k/N)² instead of (k/N)⁴.

**Assessment**: This is the most critical practical assumption. The protocol cannot detect dimension collapse. Deployment must ensure dimensions are backed by genuinely independent infrastructure. This is an operational requirement, not a protocol property.

### A4. Finite Partition Duration

**Required by**: T4/T4' (SEC), T6 (liveness).

**What it provides**: Partitions eventually heal, allowing anti-entropy to propagate all attestations to all correct nodes.

**If violated**: A permanent partition divides the network into two groups that can never reconcile. Each group may crystallize different events. SEC is impossible (CAP theorem — you cannot have Consistency and Availability under Partition).

**Counterexample**: Nodes {0..49} and {50..99} are permanently partitioned. Node 0 originates event E. After anti-entropy within each group: group A has Core = {E}, group B has Core = {}. The crystallized cores disagree permanently.

**Assessment**: Fundamental limitation (CAP theorem). No protocol can achieve SEC under permanent partition. Tesseract chooses Availability during partition (nodes continue to crystallize locally) and Consistency after partition heals (anti-entropy reconciles).

### A5. Append-Only Causal Graph

**Required by**: T3' (targeted adversary bound), σ_eff computation.

**What it provides**: The causal graph is immutable. Adversaries cannot rewrite history to reduce their correlation penalty in σ_eff.

**If violated**: If the adversary can forge or rewrite causal history, it can manufacture deep, independent-looking causal chains for its validators, making σ_eff appear high when it should be low. The σ_eff penalty for causal correlation becomes ineffective.

**Counterexample**: Adversary creates validators V_T, V_C, V_O, V_V. Each has causal depth = 0 (no history). σ_eff applies ZERO_COST_DISCOUNT → σ_eff = 4 × 0.25 = 1.0 (penalized). Now the adversary forges causal chains of depth 10 for each validator. σ_eff rises to 4.0 (full credit). The security bound degrades from (k/N)^1 to (k/N)^4... but the adversary gets the favorable bound dishonestly.

Wait — this actually helps the adversary bypass cost checks. Corrected: the adversary forges causal depth to avoid the ZERO_COST_DISCOUNT, making cheap attestations look expensive.

**Assessment**: Satisfied by hash-chaining (SHA-256). Forging a causal chain requires breaking SHA-256 preimage resistance. Under A6, this is infeasible.

### A6. Cryptographic Integrity

**Required by**: All theorems (implicitly). σ-independence assumes validator IDs are unforgeable.

**What it provides**: Validator identity binding. A validator's attestation is tied to its Ed25519 public key. The adversary cannot impersonate honest validators.

**If violated**: If the adversary can forge Ed25519 signatures, it can impersonate honest validators and claim exclusive attestation on any dimension. σ = 4 is trivially achievable for any false event.

**Counterexample**: Adversary breaks Ed25519. It generates valid signatures for honest_T, honest_C, honest_O, honest_V on a fabricated event. σ = 4. The false event crystallizes. All safety guarantees collapse.

**Assessment**: Standard cryptographic assumption. If Ed25519 breaks, all signature-based systems fail, not just Tesseract. Quantum computers capable of solving discrete log would break this — the protocol would need to migrate to post-quantum signatures (e.g., ML-DSA-65, already implemented in the parent project).

---

## 2. Derived Limitations from Theorems

Each theorem's security claim is valid only under its stated assumptions. Below, we trace what happens at the boundary of each theorem.

### From T1 (Contraction, L = 0.60)

**Conditional claim**: The diffusion operator contracts with L = α · A_max = 0.60, provided σ-structures of the two compared states are identical.

**Limitation**: If the same attestations produce different σ values at a cell (impossible under A2, but possible if floating-point rounding differs between implementations), L could exceed 0.60 at that cell. The contraction bound is not robust to σ-disagreement.

**Quantified impact**: At σ = 4, A = 4.0, α = 0.15 → w = 0.60. If one state has σ = 3 (A = 2.5, w = 0.375) and the other has σ = 4 (w = 0.60), the effective L for that cell is max(0.60, 0.625) = 0.625 < 1. Contraction still holds but with a weaker constant.

**Worst case**: σ = 0 vs σ = 4 at the same cell: L_eff = max(0.85, 0.60) = 0.85 < 1. Still contractive, but barely.

### From T3' (Targeted Security Bound)

**Conditional claim**: P(false) ≤ (k/N)^4 · exp(-4c).

**Limitation at c = 0**: Without attestation cost, P(false) = (k/N)^4.

| k/N | P(false) at c=0 |
|-----|-----------------|
| 0.10 | 0.01% |
| 0.25 | 0.39% |
| 0.33 | 1.2% |
| 0.50 | 6.25% |

At k/N = 0.50: the adversary has a 1-in-16 chance per cell. Over many cells, at least one false crystallization is likely. **Cost c is not optional for large k/N.**

**Limitation under collusion**: The bound assumes dimension assignment is independent. If 4 adversarial validators collude and coordinate which dimension each covers, they achieve σ = 4 deterministically (not probabilistically). The bound becomes:

    P(false | 4 colluding validators) = 1 · exp(-4c) = exp(-4c)

With c = 0: P = 1 (certain false crystallization). The (k/N)^4 factor vanishes entirely under perfect collusion.

**Mitigation**: σ_eff penalizes causal correlation. If the 4 colluding validators share causal history (Jaccard > 0.5), σ_eff < 4 and the effective bound strengthens. But causal correlation detection requires the causal graph, which adds complexity and is not foolproof.

### From T6 (Liveness Bound)

**Conditional claim**: Crystallization in ⌈Θ / R(4)⌉ = 9 steps within the seed region.

**Limitation on σ**: The bound requires σ(x) = 4 at the cell. If σ < 4 (insufficient attestations), R(σ) < R(4) and A(σ) < A(4):

| σ | R(σ) | T_max = ⌈Θ/R(σ)⌉ |
|---|------|-------------------|
| 4 | 0.10 | 9 |
| 3 | 0.05 | 17 |
| 2 | 0.02 | 43 |
| 0-1 | 0.00 | ∞ (never crystallizes by residual alone) |

At σ ≤ 1: R = 0, so crystallization depends entirely on diffusion from neighbors. If no neighbors are crystallized, the cell never reaches Θ. This is correct behavior (insufficient evidence), not a bug.

**Limitation on field size**: At S = 2·SEED_RADIUS = 6, the toroidal wrapping causes all cells to overlap. The seed region covers the entire field. At S ≤ 2·R, the "local" event becomes global — every cell receives probability from every attestation. This is not harmful but changes the semantics of "local event."

### From T4' (SEC)

**Conditional claim**: Convergence in d anti-entropy rounds where d = graph diameter.

**Limitation on anti-entropy frequency**: If anti-entropy runs every I ticks, convergence takes d · I ticks. For I = 10, d = 5: 50 ticks. During this window, nodes may have inconsistent crystallized cores.

**Limitation under Byzantine withholding**: A Byzantine node that participates in anti-entropy but selectively omits attestations from its seen-set can cause correct nodes to have incomplete information. The correct node reconciles with the Byzantine node and receives a subset of attestations. SEC holds among correct-to-correct pairs only.

**Counterexample**: Node B is Byzantine. It has seen attestations {A1, A2, A3, A4} but reports only {A1, A2} during anti-entropy. Node C reconciles with B and gets S_C = S_C ∪ {A1, A2}. Node D reconciles with the honest Node A and gets the full set. Now S_C ≠ S_D. SEC fails for the C-B pair.

**Mitigation**: Correct nodes that reconcile with enough correct peers (bypassing the Byzantine node) will eventually receive the full set. SEC holds on the subgraph of correct-to-correct anti-entropy connections.

---

## 3. Security Claims as Conditional Properties

Rewriting all claims in "if-then" form with explicit conditions:

**Safety** (no false crystallization):

> IF A6 holds (cryptographic integrity)
> AND the attestation structure at cell x has σ(x) < 4
> THEN x does not crystallize.

> Equivalently: x crystallizes ONLY IF σ(x) ≥ 4.

> This holds unconditionally given A6 — no network, timing, or adversary assumptions needed for safety. Safety is a local property.

**Liveness** (valid events crystallize):

> IF A3 holds (4 honest exclusive validators deliver attestations)
> AND A4 holds (attestations eventually delivered)
> AND the target cell has σ(x) = 4
> THEN x crystallizes at the center in 0 steps, and within the seed region in ≤ 9 steps.

> Fails when: A3 is violated (dimension starvation), A4 is violated (permanent partition prevents delivery), or σ < 4 at the target cell.

**Strong Eventual Consistency**:

> IF A2 holds (deterministic application)
> AND A4 holds (partitions are finite)
> AND anti-entropy runs between all correct pairs within d rounds
> THEN all correct reachable nodes have identical crystallized cores after d rounds.

> Fails when: A4 is violated (permanent partition), anti-entropy is disabled, or Byzantine nodes withhold attestations in anti-entropy.

**Convergence** (field stabilizes):

> IF A1 holds (finite field)
> THEN the field state P_n converges to a fixed point P* in sup-norm.

> This holds unconditionally given A1 — no network or adversary assumptions needed. Convergence is a local property of the evolution operator.

**Security bound** (adversarial resistance):

> IF A5 holds (append-only causal graph)
> AND A6 holds (cryptographic integrity)
> AND the adversary controls k of N validators
> THEN P(false crystallization) ≤ (k/N)^4 · exp(-4c).

> Degrades when: c = 0 (no cost), k/N is large, dimensions collapse, or 4 validators collude perfectly.

---

## 4. Parameter Sensitivity

The security properties depend on the parameter set (Θ, α, A, R, κ, c). Here we trace how changes in each parameter affect each theorem.

| Parameter | Increase effect | Decrease effect |
|-----------|----------------|-----------------|
| Θ (threshold) | Harder to crystallize → more safety, less liveness. T_max ↑. | Easier to crystallize → less safety, more liveness. |
| α (influence) | Faster diffusion → faster convergence, higher L. | Slower diffusion → slower convergence, lower L. |
| A(4) (amplification) | Higher L (may exceed 1 if α·A ≥ 1 — contraction breaks). | Lower L → stronger contraction but slower convergence. |
| R(4) (residual) | Faster liveness (T_max ↓). More perturbation budget. | Slower liveness (T_max ↑). |
| κ (cascade) | Faster propagation beyond seed radius. More perturbation. | Slower propagation. Less perturbation. |
| c (cost) | Exponentially harder for adversary. | Easier for adversary. At c=0, only (k/N)^4 protection. |

**Critical constraint**: α · A_max < 1 is required for contraction (T1). Currently 0.15 × 4.0 = 0.60 < 1. If A(4) were increased to 7 or α to 0.26, contraction would break and convergence is no longer guaranteed.

---

## 5. Irreducible Limitations

These limitations cannot be removed by any protocol modification. They are fundamental to the problem domain.

1. **CAP theorem**: During a partition, correct nodes on different sides cannot maintain consistent crystallized cores. Tesseract chooses A (availability) during partition and C (consistency) after heal. No protocol achieves both simultaneously.

2. **No truth oracle**: Crystallization measures structural convergence of evidence, not correspondence with physical reality. A perfectly coordinated set of 4 liars with distinct keys and deep causal histories will produce a false crystallization that is indistinguishable from a true one. This is not a protocol bug — it is the fundamental limit of any evidence-based system without a trusted oracle.

3. **Cost floor**: Without attestation cost (c = 0), security degrades to (k/N)^4. At k/N = 0.5 this is 6.25%. Making c > 0 requires some form of resource expenditure (computation, stake, or real-world cost). The protocol defines the interface for cost but does not prescribe the cost mechanism.

4. **Dimension independence is an infrastructure property**: The protocol enforces σ = 4 (4 exclusive validators on 4 dimensions). Whether these dimensions are genuinely independent depends on the physical infrastructure backing them. The protocol cannot distinguish "4 truly independent sources" from "4 sources that happen to share a common dependency." This is analogous to BFT systems assuming independent node operators — an assumption that must be enforced operationally, not cryptographically.

5. **Anti-entropy requires connectivity**: SEC convergence takes d rounds where d = network diameter. If d = ∞ (permanent partition), convergence never occurs. If the anti-entropy graph is sparse (few peers per node), d can be large, increasing the inconsistency window. The protocol does not enforce anti-entropy topology.
