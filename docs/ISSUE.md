# Architectural Decision: Layered Conflict Resolution

> Date: April 2026
> Status: Implemented

## Decision

The tesseract field (L1) accepts ALL events, including contradictory ones. Economic conflict resolution (e.g., double-spend) is the wallet layer's responsibility (L2).

## Layers

| Layer | Responsibility | Does NOT do |
|---|---|---|
| **Field (L1)** | Convergence, crystallization, self-healing | Balances, economic rules, rejection |
| **Wallet (L2)** | Balances, double-spend detection, scarcity | Consensus, field evolution |
| **Network** | Propagation, boundary sync | Validation, ordering |

## Rationale

Separating physics from economics keeps the core simple and agnostic:
- The field is a convergent state machine (CRDT-like)
- Economic rules can change without modifying the core
- No global consensus needed for rule validation

## Double-spend handling

1. **Both events crystallize** in the field — no rejection at L1
2. **Temporal ordering** — which crystallized first at a coordinate
3. **Curvature budget** — regions have finite capacity, weakest deformations decay

The wallet layer checks `balance >= amount` before accepting a transfer. The field provides the crystallization ordering and curvature constraints; the wallet interprets them economically.

## Trade-offs

| Gained | Lost |
|---|---|
| Core simplicity | Uniqueness guarantee at L1 |
| Brutal scalability | Deterministic economics at base layer |
| No heavy consensus | Must define L2 conflict rules |

## References

- Whitepaper section 3.8 (Layered Architecture)
- `tesseract/src/wallet.rs` — ledger with balance-based double-spend detection
- `tesseract/src/economics.rs` — curvature economy with conservation invariant
- `tesseract/tests/monetary.rs::distributed_simultaneous_conflict` — open design test
