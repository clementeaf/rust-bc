# Phase 2 Kickoff: Week-by-Week Implementation Roadmap (20 Weeks)
Digital ID System — rust-bc Evolution
Version: 1.0  
Date: December 19, 2025  
Status: Ready for execution

---

## Overview
This roadmap breaks Phase 2 into 20 weeks, aligning with the Hybrid (Option D) strategy. Ownership, deliverables, acceptance criteria, and quality gates are specified per week. Dependencies are minimized via parallelizable workstreams.

### Workstreams
- WS1: Backend — Storage & Consensus (Rust)
- WS2: Backend — Identity & API (Rust)
- WS3: Frontend — MAUI App (C#)
- WS4: DevEx — CI/CD, Tooling, Observability
- WS5: Compliance & Security — GDPR/eIDAS

---

## Week-by-Week Plan

### Week 1 — Project Boot, CI/CD Foundations
- Deliverables:
  - Repo setup, branching strategy, CODEOWNERS
  - GitHub Actions: build, test, lint jobs
  - Pre-commit hooks (format, lint, unit tests)
- Acceptance:
  - PRs blocked unless all checks pass
  - Coverage >= 45% baseline
- Risks/Mitigations:
  - Toolchain drift → pin versions in toolchain files

### Week 2 — Storage Layer Hardening
- Deliverables:
  - RocksDB schema finalized, adapters, error model
  - Storage unit tests (80+)
- Acceptance:
  - 90%+ coverage for storage
  - Read/write latency < 2ms p95

### Week 3 — DAG Consensus Skeleton
- Deliverables:
  - DAG structures, slot scheduler, basic validator
  - 30 unit tests
- Acceptance:
  - Build green, CI time < 10m

### Week 4 — Identity Foundations (DID, Keys)
- Deliverables:
  - DID doc model, key mgmt, signature service
  - 25 unit tests
- Acceptance:
  - Keys rotate, signatures verified

### Week 5 — REST API Gateway v0
- Deliverables:
  - OpenAPI spec draft, 5 endpoints implemented
  - Serialization tests
- Acceptance:
  - Contract matches integration spec

### Week 6 — Frontend Persistence and Models
- Deliverables:
  - SQLite schemas, ORM, domain models
  - 60 unit tests
- Acceptance:
  - Data encrypted at rest

### Week 7 — Frontend Services & Sync v0
- Deliverables:
  - HTTP client, sync service, retries
  - 40 unit tests
- Acceptance:
  - Offline queue works

### Week 8 — ViewModels & Basic Views
- Deliverables:
  - 3 core flows (onboarding, tx, credentials)
  - 30 unit tests
- Acceptance:
  - MVVM best practices applied

### Week 9 — Consensus Advanced (Fork Resolution)
- Deliverables:
  - Canonical path selection, rollback
  - 40 unit tests
- Acceptance:
  - Fork handling < 1s reconciliation

### Week 10 — Identity + Credential Lifecycle
- Deliverables:
  - Issue, verify, revoke credentials
  - 30 unit tests + 10 service tests
- Acceptance:
  - eIDAS mapping validated

### Week 11 — API v1 and Contract Tests
- Deliverables:
  - 15 endpoints, error spec complete
  - Contract tests (backend/frontend)
- Acceptance:
  - Backward compatibility checks

### Week 12 — Frontend UX Polish + Accessibility
- Deliverables:
  - A11y pass, localization framework
  - 20 ViewModel tests
- Acceptance:
  - AA compliance achieved

### Week 13 — System Integration I
- Deliverables:
  - End-to-end tests: tx + credential journeys
  - Load baseline (500 TPS)
- Acceptance:
  - p99 < 120ms

### Week 14 — Security & Compliance I
- Deliverables:
  - SAST, DAST baselines, threat model
  - GDPR audit logging complete
- Acceptance:
  - 0 critical vulnerabilities

### Week 15 — Performance & Resilience I
- Deliverables:
  - 1000 TPS load tests, circuit breakers
  - Canary deploy setup
- Acceptance:
  - p99 < 100ms, error <0.5%

### Week 16 — System Integration II
- Deliverables:
  - Multi-device sync, network partitions
  - 10 integration tests
- Acceptance:
  - Automatic healing validated

### Week 17 — Security & Compliance II (eIDAS)
- Deliverables:
  - QTSP integration stub, trust list checks
  - Data retention enforcement
- Acceptance:
  - Compliance tests pass

### Week 18 — Release Readiness
- Deliverables:
  - Docs, runbooks, migration scripts
  - Beta release notes
- Acceptance:
  - DR playbook rehearsed

### Week 19 — Staging Hardening
- Deliverables:
  - Staging soak (72h), bug bash
  - Final perf tests
- Acceptance:
  - 0 critical bugs, p99 < 100ms

### Week 20 — Production Launch
- Deliverables:
  - Blue/green rollout, 5% → 100% traffic
  - Postmortem template, monitoring
- Acceptance:
  - KPIs green for 24h

---

## Ownership & RACI
- Responsible: Team leads per workstream
- Accountable: Project tech lead
- Consulted: Compliance officer (WS5)
- Informed: Stakeholders weekly

---

## Quality Gates (per week)
- All tests passing
- Coverage trend non-decreasing
- 0 critical security findings
- Performance SLOs within target

---

## Risks & Mitigations
- Fork resolution complexity → Prototype by Week 3, spike tests
- Credential revocation edge cases → Property-based tests
- Mobile performance variance → Early profiling (Week 8)

---

## Budget & Timeline Integrity
- Burn rate aligned to €183K total
- Buffers embedded in Weeks 18-20
- Scope changes require change control

---

## Next Actions
- Approve this roadmap
- Create epics/issues from weekly deliverables
- Kickoff meeting (90 minutes) with all leads
