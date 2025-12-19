# Contributing to rust-bc Digital ID System

Welcome! This document provides guidelines for contributing to the rust-bc project.

**Table of Contents**
- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Workflow](#development-workflow)
- [Coding Standards](#coding-standards)
- [Testing Requirements](#testing-requirements)
- [Pull Request Process](#pull-request-process)
- [Issue Templates](#issue-templates)

---

## Code of Conduct

We are committed to providing a welcoming and inclusive environment. All contributors must:
- Treat all individuals with respect
- Report unacceptable behavior to maintainers
- Focus on constructive feedback

---

## Getting Started

### Prerequisites
- Rust 1.75.0 (auto-enforced)
- .NET SDK 8.0.0+ (auto-enforced)
- Git with pre-commit hooks configured
- Docker (optional, for local testing)

### Initial Setup

```bash
# 1. Clone repository
git clone https://github.com/your-org/rust-bc.git
cd rust-bc

# 2. Install pre-commit hooks
pre-commit install

# 3. Create feature branch
git checkout -b feature/your-feature develop

# 4. Verify setup
cargo check
dotnet build
```

---

## Development Workflow

### 1. Issue Tracking

**Before starting work:**
- Check [Issues](https://github.com/your-org/rust-bc/issues) for existing work
- Create issue if not exists (use templates below)
- Link your PR to the issue

### 2. Branch Creation

**Naming convention:** `<type>/<workstream>-<component>-<description>`

```bash
# Feature
git checkout -b feature/ws1-storage-rocksdb-adapters develop

# Bugfix
git checkout -b bugfix/consensus-fork-resolution-race develop

# Hotfix (production only)
git checkout -b hotfix/critical-api-latency-regression main
```

### 3. Local Development

```bash
# Backend development
cargo build --debug
cargo test --lib --all
cargo clippy --all -- -D warnings

# Frontend development
dotnet build --configuration Debug
dotnet test --configuration Debug

# Run pre-commit checks
pre-commit run --all-files
```

### 4. Commit Messages

Follow **Conventional Commits** format:

```
<type>(<scope>): <subject>

<body>

<footer>
```

**Types:** `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `chore`

**Scopes:** `storage`, `consensus`, `identity`, `api`, `persistence`, `services`, `viewmodels`, `ui`, `ci-cd`, `compliance`

**Example:**

```
feat(storage): add RocksDB persistence adapter

Implement storage tier abstraction with RocksDB backend.
Supports block append, proof generation, index queries.

- Factory pattern for adapter instantiation
- Error handling with exponential backoff
- Audit logging per operation

Fixes #42
Co-Authored-By: Warp <agent@warp.dev>
```

---

## Coding Standards

### Rust

**Code Style:**
```bash
cargo fmt --all
cargo clippy --all -- -D warnings
```

**Best Practices:**
- No `unwrap()` in production code (use `Result<T, E>`)
- Use `?` operator for error propagation
- Document public APIs with `///` comments
- Write unit tests alongside code
- Minimize `unsafe` blocks with clear documentation

**Example:**
```rust
/// Appends a block to storage with retry logic.
///
/// # Errors
/// Returns `StorageError` if operation fails after max retries.
pub fn append_block(&self, block: Block) -> Result<BlockHash, StorageError> {
    // implementation
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_append_block_success() {
        // test implementation
    }
}
```

### C#

**Code Style:**
```bash
dotnet format --verify-no-changes
dotnet build /p:EnforceCodeStyleInBuild=true
```

**Best Practices:**
- Use nullable reference types (`#nullable enable`)
- Prefer `record` for data objects
- Use MVVM patterns for UI code
- Async/await for I/O operations
- XML documentation (`///`) for public APIs

**Example:**
```csharp
/// <summary>
/// Verifies a credential signature and returns the result.
/// </summary>
/// <param name="credential">The credential to verify</param>
/// <returns>True if valid; otherwise false</returns>
public async Task<bool> VerifyCredentialAsync(Credential credential)
{
    // implementation
}

[TestFixture]
public class CredentialServiceTests
{
    [Test]
    public async Task VerifyCredential_WithValidSignature_ReturnsTrue()
    {
        // test implementation
    }
}
```

---

## Testing Requirements

### Test Pyramid

**Coverage Targets:**
- Overall: 80%+
- Unit tests: 75% (600+ tests)
- Service tests: 20% (160+ tests)
- Integration tests: 5% (40+ tests)

### Running Tests

```bash
# Backend: Unit tests
cargo test --lib --all -- --nocapture

# Backend: Integration tests
cargo test --test '*' --all -- --nocapture

# Frontend: All tests
dotnet test --configuration Release

# Coverage report
cargo tarpaulin --out Html --output-dir coverage
dotnet test /p:CollectCoverage=true /p:CoverageFormat=cobertura
```

### Test Quality Checklist

- [ ] Tests have descriptive names (`test_<function>_<scenario>_<expected>`)
- [ ] Both happy path and error cases covered
- [ ] No test interdependencies (isolated)
- [ ] Deterministic (no flakiness)
- [ ] Fast execution (<100ms per unit test)

---

## Pull Request Process

### Before Submitting

```bash
# 1. Sync with latest develop
git fetch origin
git rebase origin/develop

# 2. Run all checks
cargo fmt --all
cargo clippy --all -- -D warnings
cargo test --all
dotnet format
dotnet build --configuration Release
dotnet test --configuration Release

# 3. Push
git push origin feature/your-feature
```

### PR Description Template

```markdown
## Description
Brief summary of changes

## Fixes
Fixes #42

## Changes
- [] Change 1
- [ ] Change 2

## Testing
- [ ] Unit tests added (≥80% coverage delta)
- [ ] Integration tests passing
- [ ] Manual testing done

## Checklist
- [ ] Code follows style guidelines
- [ ] No new warnings introduced
- [ ] Documentation updated
- [ ] CHANGELOG.md updated
```

### Review Process

**Requirements:**
- ✅ All CI checks passing
- ✅ ≥1 approval from code reviewer
- ✅ CODEOWNERS approval for critical paths
- ✅ No merge conflicts

**Reviewers check:**
- Code quality & maintainability
- Test coverage & quality
- Security best practices
- Performance impact
- Compliance with architecture

---

## Issue Templates

### Feature Request

```markdown
**Title:** [Feature] Brief description

## Description
What problem does this solve?

## Acceptance Criteria
- [ ] Criterion 1
- [ ] Criterion 2

## Implementation Notes
- Estimated effort: X hours
- Related workstream: WSX
```

### Bug Report

```markdown
**Title:** [Bug] Brief description

## Description
Steps to reproduce

## Expected Behavior
What should happen

## Actual Behavior
What actually happens

## Environment
- OS: macOS / Linux / Windows
- Rust version: 1.75.0
- .NET version: 8.0.0
```

### Security Vulnerability

```markdown
**Title:** [Security] Brief description

## Severity
Critical / High / Medium / Low

## Description
Detailed description of vulnerability

## Impact
What is the security impact?

## Suggested Fix
(Optional) Proposed solution
```

### Performance Issue

```markdown
**Title:** [Performance] Brief description

## Metric
What metric degraded? (latency, throughput, memory)

## Current
Current value

## Target
Target value

## Profiling Data
(Optional) Attach profiling results
```

---

## Common Development Tasks

### Add a Dependency

```bash
# Rust
cargo add <crate> --features feature1,feature2
cargo update

# C#
dotnet add package <package>
```

### Run Linter Fixes

```bash
# Rust: auto-format
cargo fmt --all

# C#: auto-format
dotnet format
```

### Debug Build Issues

```bash
# Rust verbose output
RUST_BACKTRACE=1 cargo build 2>&1 | head -50

# C# detailed build
dotnet build --verbosity detailed
```

---

## Resources

- **Architecture:** [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
- **API Contract:** [docs/API_CONTRACT.md](docs/API_CONTRACT.md)
- **Branching Strategy:** [BRANCHING_STRATEGY.md](BRANCHING_STRATEGY.md)
- **GitHub Issues:** [issues](https://github.com/your-org/rust-bc/issues)
- **Discussions:** [discussions](https://github.com/your-org/rust-bc/discussions)

---

## Questions?

- Create a GitHub Discussion
- Ask in #rust-bc Slack channel
- Contact maintainers

---

**Thank you for contributing!** 🚀
