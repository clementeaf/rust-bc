# Branching Strategy — rust-bc Digital ID System

**Status:** Active for Phase 2  
**Last Updated:** December 19, 2025

---

## Overview

This document defines the Git branching model, naming conventions, protection rules, and merge procedures for the rust-bc project.

**Model:** Git Flow with simplified structure  
**Goal:** Parallel development, clear release process, minimal conflicts

---

## Branch Structure

### 1. Main Branches (Protected)

#### `main` — Production Release Branch
- **Purpose:** Production-ready code only
- **Source:** Merge from `release/*` branches
- **Protection Rules:**
  - Require pull request reviews (≥2 approvals)
  - Require CI/CD to pass (all workflows green)
  - Require CODEOWNERS approval
  - Require status checks passing
  - Dismiss stale PR reviews on push
  - Require branches up to date before merge
- **Tag Pattern:** `v*.*.* ` (semantic versioning)
- **Deployment:** Automatic to production (Week 20)

#### `develop` — Staging Integration Branch
- **Purpose:** Integration point for features
- **Source:** Merge from `feature/*` and `bugfix/*` branches
- **Protection Rules:**
  - Require pull request reviews (≥1 approval)
  - Require CI/CD to pass
  - Require CODEOWNERS approval for critical paths
  - Require branches up to date before merge
- **Deployment:** Automatic to staging (post-merge)
- **Cleanup:** Delete feature branches after merge

---

### 2. Supporting Branches (Temporary)

#### `feature/*` — Feature Development
- **Naming:** `feature/backend-tier1-storage`, `feature/frontend-persistence-layer`
- **Source:** Branch from `develop`
- **Merge Back To:** `develop` (via PR)
- **Lifecycle:** Delete after merge
- **Scope:** One feature per branch (tight scope)
- **Max Age:** 2 weeks
- **PR Requirements:**
  - Linked to GitHub issue
  - Tests added (>80% coverage delta)
  - Documentation updated
  - ≥1 approval

#### `bugfix/*` — Bug Fixes
- **Naming:** `bugfix/consensus-fork-resolution-edge-case`
- **Source:** Branch from `develop`
- **Merge Back To:** `develop` (via PR)
- **Scope:** Bug fix + test reproduction
- **PR Requirements:**
  - Issue reference required
  - Tests that reproduce bug
  - Root cause documented

#### `hotfix/*` — Production Hotfixes
- **Naming:** `hotfix/critical-api-latency-regression`
- **Source:** Branch from `main`
- **Merge Back To:** `main` + `develop` (2 PRs)
- **Scope:** Minimal, production-blocking only
- **Approval:** ≥2 approvals, tech lead required
- **Deployment:** Expedited (skip staging if approved)
- **Tag:** Pre-release tag (`v*.*.* -rc.1`) for testing

#### `release/*` — Release Preparation
- **Naming:** `release/v1.0.0` (matches tag)
- **Source:** Branch from `develop`
- **Merge Back To:** `main` + `develop`
- **Scope:** Version bump, changelog, final fixes only
- **Duration:** 1 week max
- **PR Requirements:**
  - All tests passing
  - Coverage maintained ≥80%
  - Release notes complete
  - ≥2 approvals

---

### 3. Ephemeral Branches (Development)

#### `spike/*` — Research / Prototypes
- **Naming:** `spike/dag-consensus-algorithm-v2`
- **Source:** Any branch (typically `develop`)
- **Lifetime:** Max 3 days
- **Merge Policy:** Do NOT merge; results documented, branch deleted
- **Purpose:** Validate approaches before committed development

#### `docs/*` — Documentation Updates
- **Naming:** `docs/architecture-update`, `docs/gdpr-section-v2`
- **Source:** Branch from `develop`
- **Merge Back To:** `develop`
- **Scope:** Documentation only (no code changes)
- **Review:** Single approval sufficient

---

## Naming Conventions

### Consistent Naming Pattern

```
<type>/<workstream>-<component>-<description>

Examples:
feature/ws1-storage-rocksdb-adapters
feature/ws3-frontend-mvvm-viewmodels
bugfix/consensus-fork-resolution-race-condition
hotfix/api-response-latency-regression
release/v1.0.0
spike/parallel-mining-performance-test
```

### Component Shorthand

**Backend:**
- `storage`, `consensus`, `identity`, `api`

**Frontend:**
- `persistence`, `models`, `services`, `viewmodels`, `ui`

**Cross-cutting:**
- `devex`, `compliance`, `security`, `ci-cd`

---

## Workflow: Feature Development → Production

### Phase 1: Feature Development

```
1. Create issue on GitHub (describe feature)
2. Create feature branch:
   git checkout -b feature/ws1-storage-rocksdb-adapters develop

3. Develop & commit:
   git add .
   git commit -m "feat(storage): add RocksDB adapter with error handling
   
   - Implement RocksDbBlockStore trait
   - Add retry logic for transient failures
   - Test coverage: 92%
   
   Fixes #42"

4. Push to origin:
   git push origin feature/ws1-storage-rocksdb-adapters

5. Create Pull Request on GitHub:
   - Title: "feat(storage): add RocksDB adapter"
   - Description: Link issue, explain changes, testing
   - Assign reviewers: @backend-storage-owner, @tech-lead
```

### Phase 2: Code Review

```
Reviewers check:
✓ Code quality (no clippy warnings)
✓ Test coverage (Δ >= 80%)
✓ Documentation updated
✓ No hardcoded secrets
✓ Performance baseline met
✓ CODEOWNERS rules satisfied

GitHub Actions checks:
✓ Build passes
✓ All tests passing
✓ Linting clean
✓ Security scan passing
```

### Phase 3: Merge to Develop

```
After ≥1 approval + CI green:
- Squash or rebase merge (keep history clean)
- Delete feature branch
- GitHub Actions: Deploy to staging

Commit message:
"Merge branch 'feature/ws1-storage-rocksdb-adapters' into develop

- RocksDB adapter implementation
- 92% test coverage
- Fixes #42

Co-Authored-By: Warp <agent@warp.dev>"
```

### Phase 4: Release Cycle

```
When ready for production (every 2 weeks):

1. Create release branch:
   git checkout -b release/v1.0.0 develop

2. Update version:
   - Cargo.toml: version = "1.0.0"
   - *.csproj: <Version>1.0.0</Version>

3. Update CHANGELOG.md:
   - List features, bugfixes, breaking changes
   - Credit contributors

4. Create PR: release/v1.0.0 → main
   - Title: "Release v1.0.0"
   - Description: Release notes
   - Require ≥2 approvals

5. After merge to main:
   - Tag: git tag -a v1.0.0 -m "Release v1.0.0"
   - Push tag: git push origin v1.0.0
   - GitHub Actions: Deploy to production (canary)

6. Merge back to develop:
   - Ensures version is synced
```

---

## Commit Message Standards

### Format

```
<type>(<scope>): <subject>

<body>

<footer>
```

### Type
- `feat` — New feature
- `fix` — Bug fix
- `docs` — Documentation
- `style` — Code style (format, whitespace)
- `refactor` — Refactoring without feature change
- `perf` — Performance improvement
- `test` — Adding/updating tests
- `chore` — Dependency update, config change

### Scope
- `storage`, `consensus`, `identity`, `api`, `persistence`, `models`, `services`, `viewmodels`, `ui`, `ci-cd`, `compliance`

### Subject
- Imperative mood ("add" not "added")
- Lowercase first letter
- Max 50 characters
- No period

### Body
- Explain **what** and **why** (not how)
- Wrap at 72 characters
- Optional but recommended

### Footer
- Reference issues: `Fixes #42`, `Refs #123`
- Co-author: `Co-Authored-By: Name <email>`

### Examples

```
feat(storage): add RocksDB persistence adapter

Implement storage tier abstraction with RocksDB backend.
Supports block append, proof generation, index queries.

- Factory pattern for adapter instantiation
- Error handling with exponential backoff
- Audit logging per operation

Fixes #42
Co-Authored-By: Warp <agent@warp.dev>

---

fix(consensus): resolve fork resolution race condition

Two threads could simultaneously update canonical path,
causing inconsistent state. Add mutex-protected write.

Fixes #87

---

docs: update API contract for credentials endpoint

Add request/response examples for issue, verify, revoke.
Align with OpenAPI spec v1.2.
```

---

## Protection Rules Summary

| Branch | Require Reviews | Require Status Checks | Require CODEOWNERS | Dismiss Stale | Require Up-to-Date |
|--------|-----------------|----------------------|-------------------|---------------|--------------------|
| `main` | ≥2 | Yes | Yes | Yes | Yes |
| `develop` | ≥1 | Yes | Selective | No | Yes |
| `release/*` | ≥2 | Yes | Yes | No | Yes |
| `feature/*` | ≥1 | Yes | No | No | Yes |
| `hotfix/*` | ≥2 | Yes | Yes | No | Yes |

---

## GitHub Actions Integration

### Auto-triggered Workflows

**On Push to any branch:**
- Build job (compile Rust + C#)
- Lint job (clippy, StyleCop)
- Security job (cargo audit)

**On PR to develop:**
- Full test suite
- Coverage check (≥80% delta)
- Conflict detection

**On PR to main:**
- All tests + security scans
- Performance baselines
- Manual approval required

**On Tag creation (v*.*.*):**
- Release build
- Generate artifacts
- Deploy to staging (auto)
- Deploy to production (canary 5%)

---

## Release Numbering (Semantic Versioning)

**Format:** `MAJOR.MINOR.PATCH`

- `MAJOR` — Breaking changes
- `MINOR` — New features (backward compatible)
- `PATCH` — Bug fixes

**Examples:**
- `v1.0.0` — Initial production release
- `v1.1.0` — Add credential revocation (new feature)
- `v1.1.1` — Fix fork resolution race (patch)
- `v2.0.0` — Breaking change (new API contract)

---

## Common Scenarios

### Scenario 1: Develop Feature in Parallel

```
Developer A:
git checkout -b feature/ws1-consensus-dag develop

Developer B:
git checkout -b feature/ws3-frontend-sync develop

Both can merge independently without conflicts
(separate components, separate paths)
```

### Scenario 2: Hotfix During Development

```
git checkout -b hotfix/api-latency-regression main

[Fix + test + PR]

Merge to: main (production) + develop (sync)

main: v1.0.1
develop: includes fix
```

### Scenario 3: Release Preparation

```
Weeks 19-20 of Phase 2:
- Create release/v1.0.0 from develop
- Bump versions, update changelog
- Final testing on staging
- Merge to main (tag: v1.0.0)
- Deploy to production (canary 5%)
- Monitor 10 minutes
- Expand to 100%
```

---

## Troubleshooting

### Q: How do I sync my feature branch with latest develop?

```
git fetch origin
git rebase origin/develop
git push origin feature/... --force-with-lease
```

### Q: I accidentally committed to main, what do I do?

1. Revert the commit: `git revert HEAD`
2. Push: `git push origin main`
3. Create PR to document the revert

### Q: How do I recover a deleted branch?

```
git reflog
git checkout -b recovered-branch <commit-hash>
```

---

## Enforcement

**Pre-commit Hook:** Validates branch name format  
**GitHub Branch Protection:** Enforces rules via GitHub UI  
**CI/CD:** Blocks merge if checks fail  
**Code Review:** CODEOWNERS prevents unreviewed changes

---

**This strategy is enforced from Phase 2 Week 1. All developers must adhere.**
