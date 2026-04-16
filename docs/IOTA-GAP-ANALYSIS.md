# rust-bc vs IOTA Rebased — Gap Analysis

> Last updated: 2026-04-16
>
> Objective: Track competitive gaps between rust-bc and IOTA Rebased, document progress, and guide the next implementation phases.

---

## IOTA Rebased Summary

IOTA pivoted in late 2024 from its original Tangle (feeless DAG) and abandoned IOTA 2.0 (7 years of development) after a community governance vote. **IOTA Rebased** launched on mainnet May 6, 2025.

| Component | Detail |
|-----------|--------|
| **VM** | MoveVM (object-oriented, resource-based, shared lineage with Sui) |
| **Consensus** | Mysticeti — BFT with uncertified DAG, Delegated Proof-of-Stake |
| **Throughput** | 50,000+ TPS, sub-second finality |
| **Execution** | Parallel (non-conflicting transactions execute concurrently) |
| **Fees** | Low (~0.005 IOTA/tx), burned (deflationary) |
| **EVM** | Available on L2; L1 integration planned |
| **Interoperability** | 150+ blockchain networks connected |
| **Token supply** | 4.6B initial, no hard cap, ~6% annual inflation, fee burning |
| **Staking** | DPoS, 10-15% APY, 150 max validators, 2M IOTA min stake |
| **Next consensus** | Starfish — reduces latency degradation under Byzantine behavior |

---

## Current Status — Gaps Closed

### Phase 1 — BFT Consensus ✅ DONE

Replaced Raft-only consensus with a selectable Raft/BFT backend.

**Implemented (`src/consensus/bft/`):**
- `types.rs` — `VoteMessage`, `QuorumCertificate`, `BftPhase` (Prepare→PreCommit→Commit→Decide) with phase-aware signing payload (domain separation)
- `quorum.rs` — `QuorumValidator` (2f+1 threshold), `SignatureVerifier` trait, `ensure_bft_viable()` guard (min 4 validators), `HashSet`-backed validator registry
- `vote_collector.rs` — accumulates votes per (phase, round, block_hash), signals quorum
- `round.rs` — event-driven state machine: AwaitingProposal→Preparing→PreCommitting→Committing→Decided/Failed
- `round_manager.rs` — orchestrates rounds with round-robin leader rotation, exponential backoff timeouts (3s base, 30s cap), highest QC tracking
- `backend.rs` — `ConsensusBackend` trait, `ConsensusMode` enum (Raft/Bft), `CONSENSUS_MODE` env var
- `engine.rs` — `with_bft()` builder, CommitQC validation on block acceptance (phase, hash, quorum)
- `dag.rs` — `DagBlock.commit_qc` field
- `network/mod.rs` — P2P message types: `BftProposal`, `BftVote`, `BftQuorumCertificate`, `BftViewChange`

**Verified (`tests/bft_e2e.rs`):**
- 147 unit tests + 16 adversarial E2E integration tests
- Scenarios: 4/7/10-node networks, equivocation attacks, crash faults, silent leaders with view change, network partitions (minority/majority), partition healing, alternating partitions, 100-round stress tests, mixed faults across rounds
- Safety verified: no two honest nodes decide different blocks
- Liveness verified: progress with up to f faults, stall below threshold

**Design choice:** HotStuff-inspired (leader-based, 3-phase) rather than Mysticeti (leaderless DAG). Simpler, well-understood, and Raft remains as `ConsensusMode::Raft` for permissioned deployments.

---

## Remaining Gaps — Ordered by Impact

### Phase 2 — Parallel Transaction Execution (next priority)

| Aspect | IOTA | rust-bc |
|--------|------|---------|
| **Model** | Object-centric (like Sui) — automatic parallel execution | Sequential chaincode executor |
| **TPS impact** | 50,000+ | ~18.7K (single-node ordering) |

**Foundation that already exists:**
- `chaincode/invoker.rs` tracks read-set/write-set per invocation
- `chaincode/simulation.rs` provides read-only evaluation with RW tracking
- `endorsement/` validates endorsements after simulation

**What needs to be built:**
1. Conflict detector: given a batch of txs, build a dependency graph from their read/write sets
2. Parallel executor: group non-conflicting txs into waves, execute each wave concurrently
3. Deterministic ordering: ensure all validators produce the same execution result regardless of parallelism

**Expected impact:** 2-4x throughput improvement, closing the gap to ~40-75K TPS.

### Phase 3 — Protocol-Native Tokenomics

| Aspect | IOTA | rust-bc |
|--------|------|---------|
| **Fee model** | Gas fees burned (deflationary) | SaaS billing tiers (Free/$49/$299) |
| **Storage** | Storage deposits (lock tokens, refund on delete) | None |
| **Inflation** | ~6% annual, offset by fee burning | No formal model |
| **Staking** | DPoS, 10-15% APY | Basic staking with fixed rewards |

**What needs to be built:**
1. Fee burning: tx fees → burn address (reduce supply)
2. Storage deposits: lock NOTA when creating on-chain objects, refund on deletion
3. Formal issuance curve: predictable block rewards with decay schedule
4. Validator economics: align staking rewards with BFT security needs

### Phase 4 — Cross-Chain Bridges

| Aspect | IOTA | rust-bc |
|--------|------|---------|
| **Connectivity** | 150+ chains, native bridges | Zero cross-chain |

**Options:**
- **IBC**: Well-defined standard, growing ecosystem
- **EVM bridge**: Access Ethereum/L2 liquidity
- **Light client verification**: Required foundation for trustless bridges

### Phase 5 — On-Chain Governance

| Aspect | IOTA | rust-bc |
|--------|------|---------|
| **Governance** | On-chain voting (decided the Rebased pivot) | Off-chain only |

**What needs to be built:**
1. Proposal submission with deposit
2. Stake-weighted voting
3. Timelock execution of approved proposals
4. Parameter change governance (block size, fees, slashing)

---

## Persistent Advantages (rust-bc over IOTA)

| Area | Detail |
|------|--------|
| **Post-Quantum Crypto** | ML-DSA-65 (FIPS 204) integrated end-to-end. IOTA has no PQC. |
| **Resource footprint** | ~50 MB/node vs 128 GB RAM per IOTA validator. Orders of magnitude lighter. |
| **Enterprise permissioned** | mTLS, ACL, org registry, endorsement policies, private data collections, Raft option. IOTA is public-only. |
| **Dual consensus** | Raft (permissioned) or BFT (semi-public) selectable at runtime. IOTA is DPoS-only. |
| **Fabric compatibility** | Endorse→Order→Commit pipeline, chaincode lifecycle. Direct migration path from Hyperledger Fabric. |
| **Security posture** | Audit 10/10 closed, FIPS KAT self-tests, Wasmtime v36 (15 CVEs patched), zero clippy warnings. |
| **Wasm chaincode** | Any language that compiles to Wasm (Rust, Go, C, AssemblyScript). Move is Move-only. |

---

## Strategic Positioning

**Enterprise/permissioned (primary):** rust-bc is competitive now — BFT + Raft selectable, PQC, mTLS, 50 MB footprint, Fabric-compatible pipeline. IOTA does not compete in this space.

**Public/semi-public (secondary):** Requires closing parallel execution and tokenomics gaps. With those two, rust-bc competes in the niche of lightweight PQC-ready blockchain for IoT and edge computing — where IOTA's 128 GB validator requirement is prohibitive.

**Differentiator to exploit:** PQC + lightweight footprint. No other L1 combines post-quantum signatures with sub-100 MB node requirements. This positions rust-bc uniquely for regulated industries (finance, government, defense) and resource-constrained deployments (IoT, edge, embedded).
