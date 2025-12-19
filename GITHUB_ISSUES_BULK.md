# GitHub Issues - Phase 2 Week-by-Week

Use the `gh` CLI to create issues, or copy individual issues to GitHub web UI.

## Week 1: Project Boot & CI/CD Foundations

### Issue: WK1-1 Setup Repository Branch Protection Rules
```
Title: [Week 1] Setup Repository Branch Protection Rules
Labels: area/devex, type/setup, priority/critical
Milestone: Week 1
Body:
Configure branch protection rules:
- main: 2 reviews, all checks pass, CODEOWNERS approval
- develop: 1 review, all checks pass, selective CODEOWNERS
- feature/*: 1 review, all checks pass

Links: BRANCHING_STRATEGY.md
Assignee: @devex-lead
Acceptance Criteria:
- [ ] main branch protected
- [ ] develop branch protected
- [ ] feature branch naming enforced
- [ ] All CI checks required
```

### Issue: WK1-2 Setup GitHub Actions Workflows
```
Title: [Week 1] Setup GitHub Actions Workflows
Labels: area/devex, type/setup, priority/critical
Milestone: Week 1
Body:
GitHub Actions pipelines are configured:
- .github/workflows/build.yml
- .github/workflows/test.yml
- .github/workflows/lint.yml
- .github/workflows/security.yml

Links: .github/workflows/
Assignee: @devex-lead
Acceptance Criteria:
- [ ] All 4 workflows trigger on push/PR
- [ ] Build completes in <10 min
- [ ] Tests pass 100%
- [ ] Lint passes with 0 warnings
```

### Issue: WK1-3 Team Member Onboarding
```
Title: [Week 1] Team Member Onboarding
Labels: area/devex, type/setup, priority/high
Milestone: Week 1
Body:
All team members can:
- Clone and setup locally
- Build both backend & frontend
- Run tests locally
- Submit first PR

Links: README.md, ONBOARDING.md
Assignee: @tech-lead
Acceptance Criteria:
- [ ] All 6 team members cloned repo
- [ ] All passed local build verification
- [ ] All installed pre-commit hooks
- [ ] At least 1 PR per team member
```

### Issue: WK1-4 Setup Local Development Environment
```
Title: [Week 1] Setup Local Development Environment
Labels: area/devex, type/setup, priority/high
Milestone: Week 1
Body:
Create local dev environment guide:
- Rust 1.83.0+ setup (nightly support)
- .NET 8.0.0 setup
- Pre-commit hooks installation
- First build validation

Links: README.md, rust-toolchain.toml, global.json
Assignee: @devex-lead
Acceptance Criteria:
- [ ] ONBOARDING.md created
- [ ] Setup takes <30 minutes
- [ ] Zero errors on first build
```

### Issue: WK1-5 Create Tier/Layer Initialization PRs
```
Title: [Week 1] Create Tier/Layer Initialization PRs
Labels: area/backend, area/frontend, type/feature, priority/high
Milestone: Week 1
Body:
Each workstream creates first PR:
- WS1: feature/ws1-storage-initialization
- WS2: feature/ws2-consensus-dag-skeleton
- WS3: feature/ws3-identity-foundations
- WS4: feature/ws4-api-gateway-v0
- WS3: feature/ws3-frontend-persistence-layer

Each PR:
- Has 80+ unit tests
- Passes all CI checks
- ≥80% coverage
- Linked to this issue

Acceptance Criteria:
- [ ] 5 PRs created
- [ ] All PRs pass CI
- [ ] All PRs ≥80% coverage
- [ ] All PRs merged to develop
```

---

## Week 2: Storage Layer Hardening

### Issue: WK2-1 RocksDB Schema Finalization
```
Title: [Week 2] RocksDB Schema Finalization
Labels: area/backend, type/feature, priority/high
Milestone: Week 2
Body:
Finalize RocksDB persistence schema:
- Block storage column families
- Transaction indexing
- UTXO index design
- Merkle proof storage

Links: 05_TARGET_ARCHITECTURE_BACKEND.md
Assignee: @backend-storage-owner
Acceptance Criteria:
- [ ] Schema documented
- [ ] Adapters implemented
- [ ] 80+ unit tests passing
```

### Issue: WK2-2 Storage Layer Unit Tests (80+)
```
Title: [Week 2] Storage Layer Unit Tests (80+)
Labels: area/backend, type/test, priority/high
Milestone: Week 2
Body:
Complete unit test coverage for Storage Tier:
- Block creation & serialization (10 tests)
- Merkle tree proof generation (15 tests)
- Ledger operations (25 tests)
- Index operations (20 tests)
- Error handling (10 tests)

Target: 90%+ coverage
Acceptance Criteria:
- [ ] 80+ tests implemented
- [ ] Coverage ≥90%
- [ ] All tests passing
- [ ] Documentation updated
```

---

## Week 3-20: (Additional weeks follow same pattern...)

---

## CLI Commands to Create Issues

```bash
# Install gh CLI first
brew install gh

# Authenticate
gh auth login

# Create issue from template
gh issue create \
  --title "[Week 1] Setup Repository Branch Protection Rules" \
  --label "area/devex,type/setup,priority/critical" \
  --milestone "Week 1" \
  --body "Configure branch protection rules as per BRANCHING_STRATEGY.md"

# Bulk create from file (using sed/xargs or gh script)
# For each week, create 5-10 issues
```

---

## Manual Web UI Alternative

If using GitHub web UI:
1. Go to **Issues** tab
2. Click **New Issue**
3. Use template button (if configured)
4. Fill in title, labels, milestone, assignee
5. Link to acceptance criteria

---

## Tracking & Management

**Milestone: Week 1**
- Target: 5 issues
- Expected completion: Friday EOD

**Milestone: Week 2**
- Target: 5 issues
- Expected completion: Friday EOD

*Repeat for Weeks 3-20*

---

**Status: Ready for GitHub**
- Team can create issues manually or use `gh` CLI
- Each issue links to architecture docs
- Clear acceptance criteria for verification
