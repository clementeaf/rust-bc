# 06: GitHub Issues Template & Tracking System

**Phase 1 Day 4 - Task 1**  
**Status**: Template Complete  
**Purpose**: Standardized issue tracking for Phase 2 implementation  
**Scope**: Issue types, templates, severity/priority, automation

---

## 1. Issue Types & Labels System

### Core Issue Types

```
Type: Feature
├─ Description: New capability or enhancement
├─ Label: type/feature
├─ Priority: Can be HIGH, MEDIUM, LOW
└─ Examples: Identity registration, DAG consensus, GDPR audit logging

Type: Bug
├─ Description: Defect in existing functionality
├─ Label: type/bug
├─ Priority: Can be CRITICAL, HIGH, MEDIUM, LOW
└─ Examples: Double-spend vulnerability, memory leak, parsing error

Type: Technical Debt
├─ Description: Refactoring, cleanup, infrastructure improvement
├─ Label: type/tech-debt
├─ Priority: Can be MEDIUM, LOW
└─ Examples: Add unit tests, optimize query, update dependencies

Type: Documentation
├─ Description: Update docs, README, architecture
├─ Label: type/docs
├─ Priority: Can be MEDIUM, LOW
└─ Examples: API documentation, deployment runbook

Type: Security
├─ Description: Security vulnerability or hardening
├─ Label: type/security
├─ Priority: Always CRITICAL or HIGH
└─ Examples: Key rotation, certificate validation, access control

Type: Compliance
├─ Description: Regulatory or audit requirement
├─ Label: type/compliance
├─ Priority: Can be CRITICAL, HIGH, MEDIUM
└─ Examples: GDPR audit trail, encryption validation, data retention
```

### Area Labels (for categorization)

```
Backend:
├─ area/blockchain: Consensus, mining, DAG
├─ area/identity: DID, credentials, verification
├─ area/storage: Database, persistence, RocksDB
└─ area/api: REST gateway, serialization, protocols

Frontend:
├─ area/ui: XAML views, layout
├─ area/viewmodel: MVVM logic
├─ area/services: Business logic, HTTP client
└─ area/persistence: Local storage, sync

Infrastructure:
├─ area/testing: Unit, integration, contract tests
├─ area/ci-cd: Deployment, builds, automation
├─ area/security: Encryption, audit, hardening
└─ area/compliance: GDPR, eIDAS, legal
```

### Priority & Severity Matrix

```
CRITICAL (P0):
├─ Must be fixed immediately
├─ Blocks other work
├─ Affects production security or data integrity
├─ Examples: Key compromise, data corruption, zero-day exploit
└─ SLA: 24 hours to fix

HIGH (P1):
├─ Needs fixing in current sprint
├─ Significant impact on functionality
├─ Blocks feature delivery
├─ Examples: Authentication failure, consensus bug, API downtime
└─ SLA: 1 week to fix

MEDIUM (P2):
├─ Plan for upcoming sprint
├─ Noticeable impact but workaround exists
├─ Can delay other features slightly
├─ Examples: Slow UI, missing edge case, incomplete logging
└─ SLA: 2 weeks to fix

LOW (P3):
├─ Plan for future sprints
├─ Nice-to-have improvements
├─ No user impact
├─ Examples: Code cleanup, documentation update, minor optimization
└─ SLA: No deadline
```

### Status Workflow

```
Status: Backlog
├─ Initial state: Issues not yet triaged
├─ Action: Assign to milestone, set priority
└─ Transition: → Ready

Status: Ready
├─ Issue is refined, acceptance criteria clear
├─ Action: Waiting for sprint planning
└─ Transition: → In Progress

Status: In Progress
├─ Issue is being worked on
├─ Action: Assign to developer, move to current milestone
└─ Transition: → Review or → Blocked

Status: Review
├─ Code/work complete, pending approval
├─ Action: Assign reviewer, request changes if needed
└─ Transition: → Approved or → In Progress

Status: Approved
├─ Review passed, QA in progress
├─ Action: Run tests, validate functionality
└─ Transition: → Merged or → In Progress

Status: Merged
├─ Code merged to main, deployed to staging
├─ Action: Verify in staging, document changes
└─ Transition: → Done or → Reopened (if regression)

Status: Done
├─ Issue resolved and deployed to production
├─ Action: Close issue, document lessons learned
└─ Final state

Status: Blocked
├─ Issue blocked by dependency
├─ Action: Document blocker, link to dependent issue
└─ Transition: → In Progress (when unblocked)

Status: Wontfix
├─ Issue decided not to implement
├─ Action: Document reason, close
└─ Final state

Status: Duplicate
├─ Issue is duplicate of another
├─ Action: Link to original, close
└─ Final state
```

---

## 2. Issue Templates

### Feature Template

```markdown
## Feature: [Brief Title]

### Description
[Clear description of the feature and why it's needed]

### Acceptance Criteria
- [ ] Criterion 1
- [ ] Criterion 2
- [ ] Criterion 3

### Technical Details
[Implementation notes, architecture decisions, dependencies]

### Testing Strategy
[How to test this feature]

### Area
[area/backend, area/frontend, etc.]

### Effort Estimate
[Small (1-3 days), Medium (3-5 days), Large (>5 days)]

### Dependencies
[List blocking issues or features]

### Related Issues
[Link to related issues]
```

**Example Issue**:
```markdown
## Feature: Identity Registration with Email Verification

### Description
Users need to register new digital identities with email verification. This is a core feature for the identity layer.

### Acceptance Criteria
- [ ] User can register with username, email, public key
- [ ] Email verification code sent within 5 seconds
- [ ] Code expires after 1 hour
- [ ] Registration fails if email already registered
- [ ] User receives confirmation email after successful registration

### Technical Details
- Use IIdentityService.RegisterAsync()
- Implement EmailVerificationService
- Store verification codes in Redis (5-minute expiry)
- Log all registration attempts for audit

### Testing Strategy
- Unit test: Registration validation
- Integration test: Full registration flow
- Contract test: API response format

### Area
area/identity

### Effort Estimate
Medium (3-5 days)

### Dependencies
None

### Related Issues
#42 (Identity verification)
```

### Bug Template

```markdown
## Bug: [Title]

### Description
[What happened vs. what should have happened]

### Reproduction Steps
1. [First step]
2. [Second step]
3. ...

### Expected Behavior
[What should happen]

### Actual Behavior
[What actually happened]

### Error Message
[Stack trace or error details]

### Environment
- Platform: [macOS/Linux/Windows]
- Version: [version number]
- Configuration: [relevant config]

### Severity
[CRITICAL/HIGH/MEDIUM/LOW]

### Workaround
[Temporary workaround if available]

### Related Issues
[Link to related issues]
```

**Example Issue**:
```markdown
## Bug: Double-spend detection fails on rapid transactions

### Description
If user submits two transactions with same UTXO within 100ms, both are accepted (should reject second)

### Reproduction Steps
1. Create transaction A using UTXO_123
2. Immediately (within 100ms) create transaction B using same UTXO_123
3. Submit both transactions to API endpoint
4. Check transaction log

### Expected Behavior
Second transaction should be rejected with TRANSACTION_DOUBLE_SPEND error

### Actual Behavior
Both transactions accepted, ledger is corrupted

### Error Message
None (no error thrown)

### Environment
- Platform: Linux
- Version: v0.1.0
- Configuration: Default mining difficulty

### Severity
CRITICAL (data integrity issue)

### Workaround
Add 1-second delay between transactions

### Related Issues
#156 (Memory pool race condition)
```

### Security Template

```markdown
## Security: [Vulnerability Title]

### Description
[Clear description of the security issue]

### Severity
[CRITICAL/HIGH]

### Attack Scenario
[How could attacker exploit this?]

### Current Impact
[What data/systems are at risk?]

### Root Cause
[Why does this vulnerability exist?]

### Recommended Fix
[Proposed solution]

### Verification
[How to verify the fix works]

### Related Issues
[Link to related security issues]

### Disclosure Timeline
[If reported externally, when can this be disclosed?]
```

**Example Issue**:
```markdown
## Security: Private Key Vulnerable to Memory Dump

### Description
User's Ed25519 private key stored in unencrypted memory. If process crashes with debugger attached, key can be read from memory dump.

### Severity
CRITICAL

### Attack Scenario
1. Attacker gets local access to device
2. Attaches debugger while app running
3. Memory dump contains unencrypted private key
4. Attacker can now sign transactions on behalf of user

### Current Impact
All user transactions, credential issuance, identity management

### Root Cause
Private key loaded into memory without encryption, not zeroed after use

### Recommended Fix
- Use memory-safe structures (zeroize crate)
- Store private key only in OS Keychain (locked behind biometric)
- Never load unencrypted to memory

### Verification
- Memory dump test (attach debugger, verify key not readable)
- Biometric auth test (verify unlock required)

### Disclosure Timeline
Fix by 2026-01-15, disclosure 48 hours after fix deployed
```

### Compliance Template

```markdown
## Compliance: [Requirement Title]

### Regulation
[GDPR Article 25, eIDAS Article 3, etc.]

### Requirement
[What does the regulation require?]

### Current Status
[What's implemented, what's missing?]

### Implementation Plan
- [ ] Step 1
- [ ] Step 2
- [ ] Step 3

### Success Criteria
[How will we verify compliance?]

### Auditor Notes
[Any specific auditor feedback]

### Related Issues
[Link to related compliance issues]
```

**Example Issue**:
```markdown
## Compliance: GDPR Article 17 - Right to Be Forgotten

### Regulation
GDPR Article 17 (Right to erasure)

### Requirement
User can request deletion of all personal data. System must delete within 30 days and confirm in writing.

### Current Status
- ✅ Deletion API endpoint exists
- ❌ Grace period not enforced (deletes immediately)
- ❌ No confirmation email sent
- ❌ Audit log not retained

### Implementation Plan
- [ ] Add 30-day grace period to DeletionService
- [ ] Send confirmation email after deletion completes
- [ ] Keep audit logs for 3 years per data retention policy
- [ ] Add deletion status UI in app settings
- [ ] Test with external auditor

### Success Criteria
- [ ] External GDPR auditor approves deletion workflow
- [ ] User can verify deletion request in app
- [ ] User receives confirmation email
- [ ] Deletion completes within 30 days

### Related Issues
#234 (Data retention policy)
#235 (Audit logging)
```

---

## 3. Sprint Planning Template

### Milestone (Sprint)

```markdown
## Phase 2 Sprint 1: Identity Layer Foundation

### Duration
Weeks 1-4 (January 6 - February 2, 2026)

### Goals
- [x] DID registration system
- [x] Email verification
- [x] JWT authentication
- [x] 80% test coverage

### Issues Included
- #101: DID system design
- #102: Identity registration API
- #103: Email verification service
- #104: JWT token generation
- #105: Unit tests for identity

### Success Criteria
- [ ] All issues in "Done" status
- [ ] Test coverage ≥ 80%
- [ ] No CRITICAL bugs
- [ ] Documentation complete
- [ ] Demo to stakeholders

### Blockers
[None initially]

### Burndown Chart
[Managed in GitHub Projects]
```

---

## 4. Automated Issue Labeling & Actions

### GitHub Actions Workflow

```yaml
# File: .github/workflows/issue-triage.yml

name: Issue Triage

on:
  issues:
    types: [opened, edited]

jobs:
  triage:
    runs-on: ubuntu-latest
    steps:
      # Auto-label based on title keywords
      - name: Label by keyword
        uses: actions/github-script@v6
        with:
          script: |
            const title = context.payload.issue.title.toLowerCase();
            const labels = [];
            
            if (title.includes('security')) labels.push('type/security');
            if (title.includes('bug')) labels.push('type/bug');
            if (title.includes('feature')) labels.push('type/feature');
            if (title.includes('gdpr')) labels.push('type/compliance');
            
            if (labels.length > 0) {
              github.rest.issues.addLabels({
                issue_number: context.issue.number,
                owner: context.repo.owner,
                repo: context.repo.repo,
                labels: labels
              });
            }

      # Require description
      - name: Check description
        uses: actions/github-script@v6
        with:
          script: |
            const body = context.payload.issue.body || '';
            if (body.length < 50) {
              github.rest.issues.createComment({
                issue_number: context.issue.number,
                owner: context.repo.owner,
                repo: context.repo.repo,
                body: 'Please provide more detailed description (min 50 characters)'
              });
            }

      # Assign to area label
      - name: Assign area
        if: contains(context.payload.issue.labels.*.name, 'area/*')
        uses: actions/github-script@v6
        with:
          script: |
            const labels = context.payload.issue.labels.map(l => l.name);
            const areaLabel = labels.find(l => l.startsWith('area/'));
            
            if (areaLabel) {
              const areaOwners = {
                'area/blockchain': ['@alice', '@bob'],
                'area/identity': ['@charlie', '@diana'],
                'area/frontend': ['@eve', '@frank'],
                'area/compliance': ['@grace']
              };
              
              const assignees = areaOwners[areaLabel] || [];
              // Auto-assign based on area
            }
```

---

## 5. Release Notes Template

### Release Template

```markdown
## Release v0.2.0 - Identity Layer MVP

**Release Date**: February 2, 2026

### Features
- ✅ Digital Identity (DID) registration
- ✅ Email verification
- ✅ JWT authentication
- ✅ Credential issuance (basic)
- ✅ REST API for identity operations

### Bug Fixes
- 🐛 Fixed race condition in transaction validation
- 🐛 Fixed memory leak in DAG consensus
- 🐛 Fixed incorrect error message on failed verification

### Security
- 🔒 Added private key encryption in Keychain
- 🔒 Added rate limiting to API endpoints
- 🔒 Added audit logging to all identity operations

### Compliance
- ✅ GDPR audit trail implementation
- ✅ Data retention policy enforcement
- ✅ Encryption validation by auditor

### Breaking Changes
None

### Migration Guide
N/A (new features only)

### Known Issues
- [ ] Credential revocation not yet implemented (v0.3.0)
- [ ] eIDAS qualified signatures pending (v0.4.0)
- [ ] Performance optimization needed (v0.3.0)

### Contributors
- [@alice](github.com/alice)
- [@bob](github.com/bob)
- [@charlie](github.com/charlie)

### Downloads
- [Rust Backend](https://releases/rust-bc-0.2.0.tar.gz)
- [C# Frontend](https://releases/neuroaccess-0.2.0.apk)
```

---

## 6. Metrics & Reporting

### Weekly Status Report

```markdown
## Week 1 Status Report: Phase 2 Sprint 1

### Velocity
- Issues Completed: 4/6 (67%)
- Story Points Completed: 21/30 (70%)
- Projected Sprint Completion: On Track

### Quality Metrics
- Test Coverage: 78% (target: 80%)
- Bugs Found: 2 (both MEDIUM)
- Critical Vulnerabilities: 0

### Blockers
- Issue #105 blocked by external API documentation (TAG)
- Action: Escalated to manager for vendor follow-up

### Upcoming
- Week 2: API contract testing
- Week 3: Integration testing
- Week 4: Compliance review

### Risks
- Team member out sick (1 day lost)
- External dependency delay (2 days impact if not resolved)

### Action Items
- [ ] Follow up with TAG on API docs (due: tomorrow)
- [ ] Add additional test coverage for edge cases
- [ ] Schedule compliance pre-audit
```

---

## 7. Integration with CI/CD

### GitHub Project Board Columns

```
Backlog → Ready → In Progress → Review → Approved → Merged → Done → Closed
```

### Automatic Transitions

```
Code pushed to PR
  → Issue moves to "Review"

PR approved
  → Issue moves to "Approved"

PR merged to main
  → Issue moves to "Merged"

Tests pass + deployed to prod
  → Issue moves to "Done"

Issues closed manually → Status = "Closed"
```

---

## 8. Issue Best Practices

### DO:
✅ Create one issue per task
✅ Use clear, specific titles
✅ Provide context and acceptance criteria
✅ Link related issues
✅ Keep issues small (1-5 days effort)
✅ Update status regularly
✅ Include reproduction steps for bugs
✅ Document decisions in issue comments

### DON'T:
❌ Create vague issues ("Fix stuff")
❌ Mix multiple concerns in one issue
❌ Leave issues unassigned
❌ Ignore bugs in review
❌ Close issues without verification
❌ Create duplicate issues
❌ Leave issues in "In Progress" for > 3 days without update
❌ Use issues as chat (use Slack for quick questions)

---

**End of GitHub Issues Template**

*Use this template to create consistent, trackable issues for Phase 2 implementation.*
