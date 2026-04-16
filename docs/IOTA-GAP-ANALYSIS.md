# rust-bc vs IOTA Rebased — Gap Analysis

> Date: 2026-04-16
>
> Objective: Identify competitive gaps between rust-bc and IOTA Rebased to define a roadmap for taking rust-bc to the next level.

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

## Critical Gaps — Block "Next Level"

| # | Area | IOTA Rebased | rust-bc | Impact |
|---|------|-------------|---------|--------|
| 1 | **Throughput** | 50,000+ TPS, sub-second finality | ~18.7K TPS ordering (single-node), Raft distributed throughput unverified | 2.5-3x less in best case. No parallel tx execution. |
| 2 | **Consensus** | Mysticeti (DAG BFT, leaderless) → Starfish | Raft 3-node (leader-based, CFT not BFT). DAG module is scaffolding with `#[allow(dead_code)]` | Raft tolerates crashes, not Byzantine faults. A single malicious node can compromise the network. |
| 3 | **Smart Contract VM** | MoveVM — formal verification, resource-oriented, native parallel execution | Wasmtime — sandboxed but no formal verification, sequential execution | Move prevents entire bug classes by design (double-spend impossible at type level). Wasm is generic but offers no such guarantees. |
| 4 | **Parallel execution** | Object model enables automatic parallel execution of independent txs | Sequential — one tx at a time in the chaincode executor | Fundamental bottleneck for scaling TPS. |
| 5 | **Horizontal scaling** | DPoS with 150 validators, designed to scale | Raft effective max ~5-7 nodes, no sharding, no L2 | Low ceiling. Raft does not scale beyond a handful of nodes. |

---

## Important Gaps — Significant Competitive Disadvantage

| # | Area | IOTA Rebased | rust-bc | Impact |
|---|------|-------------|---------|--------|
| 6 | **Tokenomics** | Fee burning (deflationary), DPoS staking 10-15% APY, storage deposits, formal economic model | NOTA token with basic staking + SaaS-style billing tiers (Free/$49/$299) | No formal economic model. Billing tiers feel SaaS, not protocol-native. No fee burning or storage deposits. |
| 7 | **Interoperability** | 150+ chains connected, native bridges | Zero cross-chain capability. Isolated network. | No bridges = no external liquidity or composability. |
| 8 | **Developer ecosystem** | Move language ecosystem, dApp Kit (React), mature TypeScript SDK, docs.iota.org | Python SDK (28 methods), basic JS SDK, Vite explorer | SDKs functional but thin. No contract language of its own. No dApp framework. |
| 9 | **On-chain governance** | On-chain voting (used to decide the Rebased pivot) | None. All decisions off-chain. | No formal governance mechanism = de facto centralization. |
| 10 | **Gas Station / Sponsored txs** | Built-in — enables gasless txs for onboarding | Not implemented | Entry barrier for new users. |

---

## Nice-to-Have — Minor but Relevant Differences

| # | Area | IOTA Rebased | rust-bc | Impact |
|---|------|-------------|---------|--------|
| 11 | **EVM compatibility** | L2 now, L1 planned | None | Limits access to the Solidity/DeFi ecosystem. |
| 12 | **Formal verification** | Move has built-in formal verification | None | Contracts cannot be formally verified. |
| 13 | **Light clients** | Supported | Not implemented | Mobile/IoT devices cannot participate without a full node. |
| 14 | **Public testnets** | Testnet + Devnet active | Docker local only | No public testnet = no community testing. |

---

## Where rust-bc Wins or Matches

| Area | rust-bc Advantage |
|------|-------------------|
| **Post-Quantum Crypto** | ML-DSA-65 (FIPS 204) integrated end-to-end. IOTA has no PQC yet. **Real differentiator.** |
| **Fabric compatibility** | Endorse→Order→Commit pipeline, private data collections, chaincode lifecycle. IOTA does not compete here. |
| **Enterprise permissioned** | mTLS, ACL, org registry, endorsement policies. IOTA is public/permissionless. **Different niche.** |
| **Security audit** | 10/10 findings closed, FIPS KAT self-tests at startup, Wasmtime v36 (15 CVEs patched). Solid posture. |
| **Resource footprint** | ~50 MB/node vs IOTA validator requires 128 GB RAM. **Orders of magnitude lighter.** |
| **Wasm chaincode** | Any language that compiles to Wasm. Move is Move-only. |

---

## Suggested Roadmap

### Phase 1 — BFT Consensus (most critical gap)

Replace Raft (CFT) with a real BFT consensus. Options:

- **HotStuff / HotStuff-2**: Linear communication complexity, pipelined, well-understood.
- **CometBFT (Tendermint)**: Battle-tested, IBC-compatible out of the box.
- **DAG-based BFT**: Build on existing `src/consensus/dag.rs` scaffolding to implement a Mysticeti-style protocol.

Raft can remain as an optional ordering service for permissioned deployments.

### Phase 2 — Parallel Transaction Execution

Analyze transaction dependencies using read/write sets (already tracked by `simulation.rs` and `invoker.rs`) and execute non-conflicting txs concurrently.

The foundation exists:
- `invoker.rs` tracks read-set/write-set per invocation
- `simulation.rs` provides read-only evaluation
- Conflict detection logic needs to be added to build a dependency graph per block

### Phase 3 — Protocol-Native Tokenomics

Migrate from SaaS billing model to protocol-native economics:

- **Fee burning**: Deflationary pressure from tx fees
- **Storage deposits**: Lock tokens when creating on-chain objects, refundable on deletion
- **Formal inflation/deflation model**: Predictable issuance curve
- **Validator economics**: Align staking rewards with network security needs

### Phase 4 — Cross-Chain Bridges

Break network isolation with at least one bridge:

- **IBC (Inter-Blockchain Communication)**: If Phase 1 uses CometBFT, IBC comes nearly free.
- **EVM bridge**: Connect to Ethereum/L2 ecosystem for liquidity access.
- **Light client verification**: Required foundation for trustless bridges.

### Phase 5 — On-Chain Governance

Implement a voting mechanism for protocol upgrades:

- Proposal submission with deposit
- Validator-weighted or stake-weighted voting
- Timelock execution of approved proposals
- Parameter change governance (block size, fees, slashing thresholds)

---

## Strategic Positioning

rust-bc's natural strength is **enterprise/permissioned blockchain** (Fabric alternative). Competing head-to-head with IOTA in the public/permissionless space requires closing the BFT, parallel execution, and tokenomics gaps.

The **PQC advantage** (ML-DSA-65) and **lightweight footprint** (~50 MB vs 128 GB) are real differentiators worth exploiting — particularly for regulated industries and IoT edge deployments where IOTA's validator requirements are prohibitive.

A pragmatic strategy: **own the enterprise niche first**, then expand to public networks as BFT and parallel execution mature.
