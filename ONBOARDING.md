# Team Onboarding Guide - Phase 2 Week 1

Welcome to the rust-bc Digital ID System Phase 2 development team! 🚀

This guide will get you set up and ready to submit your first PR within 30 minutes.

---

## 1. Prerequisites

Verify you have:
- [ ] Git installed (`git --version`)
- [ ] Homebrew installed (macOS only)
- [ ] Terminal/Shell access
- [ ] GitHub account (@your-username)
- [ ] SSH key configured for GitHub
- [ ] ~30 minutes of time

---

## 2. Local Setup (25 minutes)

### Step 1: Clone Repository (2 min)

```bash
git clone git@github.com:your-org/rust-bc.git
cd rust-bc
```

### Step 2: Install Rust (5 min)

The project uses Rust nightly (1.85.0+) for `edition2024` support.

```bash
# macOS
brew install rustup
rustup update

# Linux
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Verify:
```bash
rustc --version  # Should be 1.85.0-nightly or newer
cargo --version
```

### Step 3: Install .NET (8 min)

The project uses .NET SDK 8.0.0+.

```bash
# macOS
brew install dotnet

# Linux / Manual
# Download from https://dotnet.microsoft.com/download

# Verify
dotnet --version  # Should be 8.0.0+
```

### Step 4: Install Pre-commit Hooks (3 min)

```bash
# Install pre-commit
brew install pre-commit  # or: pip install pre-commit

# Setup hooks in repository
cd rust-bc
pre-commit install

# Verify installation
pre-commit run --all-files
```

This runs automated checks before every commit.

### Step 5: First Build (8 min)

```bash
# Backend
cargo build --debug
cargo test --lib  # Should pass 112/112 tests

# Frontend (if assigned to C# team)
dotnet build --configuration Debug
dotnet test --configuration Debug
```

**Expected result:** All builds succeed, zero errors.

---

## 3. GitHub Setup (3 min)

### Configure Git User

```bash
git config --global user.name "Your Full Name"
git config --global user.email "your.email@example.com"
```

### Set Default Branch

```bash
cd rust-bc
git checkout develop  # Week 1-2 development is on develop
```

---

## 4. First PR Workflow (15 min including review time)

### Scenario: You're assigned to WS1 (Storage Tier)

#### Step 1: Create Feature Branch (1 min)

```bash
git checkout -b feature/ws1-storage-initialization develop
```

**Branch naming:** `feature/ws#-component-description`

#### Step 2: Make Changes (5 min example)

Let's say you add a unit test:

```bash
# Edit a file
vim src/storage/lib.rs

# Add your changes...
```

#### Step 3: Commit Changes (2 min)

```bash
git add .

git commit -m "feat(storage): add block serialization unit tests

Add 10 unit tests for block creation and serialization:
- test_block_creation_success
- test_block_serialization
- test_block_deserialization
- ... (7 more)

Acceptance criteria from WK1-5:
- [ ] 80+ unit tests (partial: +10)
- [ ] Passes all CI
- [ ] ≥80% coverage

Fixes #123
Co-Authored-By: Warp <agent@warp.dev>"
```

**Commit format:** Follow [CONTRIBUTING.md](CONTRIBUTING.md#commit-messages)

#### Step 4: Push to Origin (1 min)

```bash
git push origin feature/ws1-storage-initialization
```

#### Step 5: Create Pull Request (3 min)

On GitHub:
1. Go to **Pull Requests** tab
2. Click **New Pull Request**
3. Select:
   - Base: `develop`
   - Compare: `feature/ws1-storage-initialization`
4. Fill in PR template:
   ```markdown
   ## Description
   Add 10 unit tests for block serialization
   
   ## Fixes
   Fixes #123
   
   ## Testing
   - [x] Unit tests added (10 new tests)
   - [x] Coverage ≥80% (verified locally)
   - [x] All CI checks passing
   
   ## Checklist
   - [x] Code follows style guidelines
   - [x] No new warnings introduced
   - [x] Documentation updated
   - [x] Tests added
   ```
5. Assign reviewers: @backend-storage-owner, @tech-lead
6. Add labels: `area/backend`, `type/test`, `priority/high`
7. Link to milestone: `Week 1`
8. Click **Create Pull Request**

#### Step 6: Wait for Code Review

Reviewers will check:
- [ ] Code quality
- [ ] Test coverage (≥80%)
- [ ] All CI checks green
- [ ] No hardcoded secrets

**Expected:** Review within 24 hours

#### Step 7: Address Feedback (if needed)

```bash
# If reviewers request changes:
git add .
git commit -m "refactor(storage): address review feedback

- Refactor block_serialization test for clarity
- Add additional edge case tests"

git push origin feature/ws1-storage-initialization

# PR auto-updates, no need to recreate
```

#### Step 8: Merge to Develop

After approval and all checks pass:
1. Reviewer clicks **Merge**
2. Select **Squash and merge**
3. Feature branch auto-deletes

---

## 5. Daily Workflow Checklist

### Every Morning
- [ ] `git fetch origin`
- [ ] `git rebase origin/develop` (if behind)
- [ ] `cargo check` or `dotnet build` (quick validation)

### Before Committing
- [ ] `pre-commit run --all-files` (runs locally)
- [ ] Tests pass locally (`cargo test` or `dotnet test`)
- [ ] No TODOs/FIXMEs left in code

### Before Creating PR
- [ ] Branch is up-to-date with develop
- [ ] All CI checks will pass (`cargo clippy`, `dotnet format`)
- [ ] Coverage ≥80%
- [ ] PR description includes acceptance criteria

### After PR Merged
- [ ] Switch back to develop: `git checkout develop`
- [ ] Pull latest: `git pull origin develop`
- [ ] Delete local branch: `git branch -d feature/...`

---

## 6. Common Issues & Fixes

### "Rust toolchain not found"
```bash
rustup update nightly-2024-12-19
```

### ".NET SDK not found"
```bash
dotnet --list-sdks
# If needed: dotnet tool update --global dotnet-format
```

### "Pre-commit hook failed"
```bash
# View which check failed
pre-commit run --all-files

# Auto-fix common issues
cargo fmt --all
dotnet format

# Re-commit
git add . && git commit -m "fix: format code"
```

### "Test failures locally but CI passed"
```bash
# Ensure you're on the right toolchain
rustc --version  # Should be nightly-2024-12-19
rustup update

# Run tests again
cargo test --lib
```

### "Can't push to remote"
```bash
# Verify branch exists on remote
git branch -r

# If blocked by branch protection, wait for approvals
# Check PR status on GitHub
```

---

## 7. Resources

- **Architecture:** [05_TARGET_ARCHITECTURE_BACKEND.md](05_TARGET_ARCHITECTURE_BACKEND.md), etc.
- **Branching:** [BRANCHING_STRATEGY.md](BRANCHING_STRATEGY.md)
- **Contributing:** [CONTRIBUTING.md](CONTRIBUTING.md)
- **Testing:** [08_TESTING_STRATEGY_PHASE2.md](ANALYSIS/08_TESTING_STRATEGY_PHASE2.md)
- **API Contract:** [05_TARGET_ARCHITECTURE_INTEGRATION.md](05_TARGET_ARCHITECTURE_INTEGRATION.md)

---

## 8. Team Contacts

- **Tech Lead:** @tech-lead
- **Backend Lead (WS1-2):** @backend-storage-owner, @backend-consensus-owner
- **Frontend Lead (WS3):** @frontend-persistence-owner
- **DevEx Lead (WS4):** @devex-lead
- **Compliance (WS5):** @compliance-officer

---

## 9. Success Criteria for Week 1

You're ready if you can:
- [ ] Clone and build locally (no errors)
- [ ] Run tests locally (112/112 passing)
- [ ] Create a feature branch
- [ ] Make a code change
- [ ] Commit with proper message format
- [ ] Push to origin
- [ ] Create a Pull Request
- [ ] Address review feedback
- [ ] Merge to develop

---

## 10. First Week Goals

**By Friday EOD:**
- [ ] Complete setup (today/tomorrow)
- [ ] Assign to a workstream (WS1-5)
- [ ] Pick a Week 1 issue from [GITHUB_ISSUES_BULK.md](GITHUB_ISSUES_BULK.md)
- [ ] Submit first PR (target: WK1-5)
- [ ] Merged to develop (all checks green)

**Next Week:** Move to Week 2 issues and start Phase 2 development.

---

## Questions?

1. Check the FAQ below
2. Ask in #rust-bc Slack channel
3. Message @tech-lead directly

---

## FAQ

### Q: Can I work on macOS/Linux/Windows?
**A:** macOS and Linux fully supported. Windows use WSL2.

### Q: Do I need a specific IDE?
**A:** Any IDE works. Popular choices:
- Rust: VS Code + Rust Analyzer, IntelliJ IDEA
- C#: Visual Studio 2022, VS Code + C# Dev Kit

### Q: How long do tests take to run locally?
**A:** ~2-3 seconds for unit tests, ~30 seconds for full test suite.

### Q: What if I make a mistake in git?
**A:** Most mistakes are reversible:
- Uncommitted changes: `git checkout -- .`
- Wrong commit: `git reset HEAD~1`
- Wrong branch: `git checkout -b new-branch && git reset origin/develop --soft`

### Q: Can I commit to main/develop directly?
**A:** No, branch protection prevents direct commits. Always use feature branches and PRs.

### Q: How many reviewers do I need?
**A:** For WK1-5 (first PR): ≥1 reviewer from your workstream + tech-lead
For subsequent PRs: ≥1 + selective CODEOWNERS approval

### Q: What if CI checks fail?
**A:** Re-run locally:
```bash
cargo clippy --all
dotnet format --verify-no-changes
cargo test --lib
```

Fix errors, commit, push — CI will re-run automatically.

---

**Status: Ready to Onboard** ✅

Start here → [README.md](README.md)

**Good luck, and welcome to the team!** 🎉
