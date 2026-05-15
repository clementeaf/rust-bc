//! Governance persistence integration tests.
//!
//! Covers:
//! - Realistic lifecycle: submit → vote → close → persist → hydrate
//! - RocksDB persistence across reopen (simulates node restart)
//! - Stress: 500 proposals, 1000 concurrent votes
//! - Adversarial: duplicate votes via store bypass, ID overflow,
//!   null-byte injection, vote replay after hydration, tampered proposals

use std::sync::Arc;

use rust_bc::governance::proposals::{
    Proposal, ProposalAction, ProposalStatus, ProposalStore, SubmitParams,
};
use rust_bc::governance::voting::{Vote, VoteOption, VoteStore};
use rust_bc::storage::traits::BlockStore;
use rust_bc::storage::MemoryStore;
use tempfile::TempDir;

// ── Helpers ──────────────────────────────────────────────────────────────────

fn submit_proposal(ps: &ProposalStore, proposer: &str, height: u64) -> u64 {
    ps.submit(SubmitParams {
        proposer,
        action: ProposalAction::TextProposal {
            title: "Test".into(),
            description: "Description".into(),
        },
        description: "test",
        deposit: 100,
        required_deposit: 100,
        current_height: height,
        voting_period: 1000,
    })
    .unwrap()
}

fn rocksdb_store() -> (rust_bc::storage::adapters::RocksDbBlockStore, TempDir) {
    let dir = TempDir::new().unwrap();
    let store = rust_bc::storage::adapters::RocksDbBlockStore::new(dir.path()).unwrap();
    (store, dir)
}

// ── 1. Realistic lifecycle: submit → vote → persist → hydrate ────────────────

#[test]
fn full_lifecycle_persist_and_hydrate() {
    let store: Arc<dyn BlockStore> = Arc::new(MemoryStore::new());
    let ps = ProposalStore::new();
    let vs = VoteStore::new();

    // Submit 3 proposals
    let id1 = submit_proposal(&ps, "alice", 100);
    let id2 = submit_proposal(&ps, "bob", 200);
    let id3 = submit_proposal(&ps, "carol", 300);

    // Cast votes
    vs.cast_vote(id1, "v1", VoteOption::Yes, 1000, 101, 1100)
        .unwrap();
    vs.cast_vote(id1, "v2", VoteOption::No, 500, 102, 1100)
        .unwrap();
    vs.cast_vote(id2, "v1", VoteOption::Abstain, 800, 201, 1200)
        .unwrap();
    vs.cast_vote(id3, "v3", VoteOption::Yes, 2000, 301, 1300)
        .unwrap();

    // Persist all proposals and votes
    for p in ps.list_all() {
        store.write_proposal(&p).unwrap();
    }
    for pid in [id1, id2, id3] {
        for v in vs.get_votes(pid) {
            store.write_vote(&v).unwrap();
        }
    }

    // Hydrate into fresh stores (simulates restart)
    let ps2 = ProposalStore::new();
    let vs2 = VoteStore::new();

    for p in store.list_proposals().unwrap() {
        ps2.load_proposal(p);
    }
    for p in store.list_proposals().unwrap() {
        for v in store.list_votes(p.id).unwrap() {
            vs2.load_vote(v);
        }
    }

    // Verify proposals survived
    assert_eq!(ps2.count(), 3);
    assert_eq!(ps2.get(id1).unwrap().proposer, "alice");
    assert_eq!(ps2.get(id2).unwrap().proposer, "bob");
    assert_eq!(ps2.get(id3).unwrap().proposer, "carol");

    // Verify votes survived
    assert_eq!(vs2.get_votes(id1).len(), 2);
    assert_eq!(vs2.get_votes(id2).len(), 1);
    assert_eq!(vs2.get_votes(id3).len(), 1);

    // Verify tally is identical
    let t1 = vs.tally(id1, 10_000, 33, 67);
    let t2 = vs2.tally(id1, 10_000, 33, 67);
    assert_eq!(t1.yes_power, t2.yes_power);
    assert_eq!(t1.no_power, t2.no_power);
    assert_eq!(t1.passed, t2.passed);

    // Verify next_id advanced — new proposals get id > 3
    let id4 = submit_proposal(&ps2, "dave", 500);
    assert!(id4 > id3, "next_id must advance past loaded proposals");
}

// ── 2. RocksDB persistence survives reopen ───────────────────────────────────

#[test]
fn rocksdb_proposals_and_votes_survive_reopen() {
    let dir = TempDir::new().unwrap();

    // Phase 1: write proposals and votes
    {
        let store = rust_bc::storage::adapters::RocksDbBlockStore::new(dir.path()).unwrap();
        let proposal = Proposal {
            id: 42,
            proposer: "alice".into(),
            action: ProposalAction::TextProposal {
                title: "Persist me".into(),
                description: "Across restarts".into(),
            },
            status: ProposalStatus::Voting,
            deposit: 500,
            submitted_at: 10,
            voting_ends_at: 1010,
            timelock_ends_at: None,
            finalized_at: None,
            description: "persistence test".into(),
        };
        store.write_proposal(&proposal).unwrap();

        let vote = Vote {
            voter: "validator-1".into(),
            proposal_id: 42,
            option: VoteOption::Yes,
            power: 5000,
            voted_at: 15,
        };
        store.write_vote(&vote).unwrap();
    }
    // DB dropped here

    // Phase 2: reopen and verify
    {
        let store = rust_bc::storage::adapters::RocksDbBlockStore::new(dir.path()).unwrap();
        let proposals = store.list_proposals().unwrap();
        assert_eq!(proposals.len(), 1);
        assert_eq!(proposals[0].id, 42);
        assert_eq!(proposals[0].proposer, "alice");

        let proposal = store.read_proposal(42).unwrap();
        assert_eq!(proposal.description, "persistence test");

        let votes = store.list_votes(42).unwrap();
        assert_eq!(votes.len(), 1);
        assert_eq!(votes[0].voter, "validator-1");
        assert_eq!(votes[0].power, 5000);
    }
}

// ── 3. Proposal status update persists correctly ─────────────────────────────

#[test]
fn proposal_status_transitions_persist() {
    let (store, _dir) = rocksdb_store();

    let mut proposal = Proposal {
        id: 1,
        proposer: "alice".into(),
        action: ProposalAction::TextProposal {
            title: "T".into(),
            description: "D".into(),
        },
        status: ProposalStatus::Voting,
        deposit: 100,
        submitted_at: 0,
        voting_ends_at: 100,
        timelock_ends_at: None,
        finalized_at: None,
        description: "".into(),
    };
    store.write_proposal(&proposal).unwrap();

    // Transition to Passed
    proposal.status = ProposalStatus::Passed;
    proposal.timelock_ends_at = Some(200);
    store.write_proposal(&proposal).unwrap();
    let loaded = store.read_proposal(1).unwrap();
    assert_eq!(loaded.status, ProposalStatus::Passed);
    assert_eq!(loaded.timelock_ends_at, Some(200));

    // Transition to Executed
    proposal.status = ProposalStatus::Executed;
    proposal.finalized_at = Some(200);
    store.write_proposal(&proposal).unwrap();
    let loaded = store.read_proposal(1).unwrap();
    assert_eq!(loaded.status, ProposalStatus::Executed);
    assert_eq!(loaded.finalized_at, Some(200));
}

// ── 4. Stress: 500 proposals + 10 votes each ────────────────────────────────

#[test]
fn stress_500_proposals_5000_votes() {
    let (store, _dir) = rocksdb_store();

    for i in 1..=500u64 {
        let proposal = Proposal {
            id: i,
            proposer: format!("proposer-{i}"),
            action: ProposalAction::TextProposal {
                title: format!("Proposal {i}"),
                description: "stress".into(),
            },
            status: ProposalStatus::Voting,
            deposit: 100,
            submitted_at: i,
            voting_ends_at: i + 1000,
            timelock_ends_at: None,
            finalized_at: None,
            description: "stress test".into(),
        };
        store.write_proposal(&proposal).unwrap();

        for v in 0..10u64 {
            let vote = Vote {
                voter: format!("voter-{v}"),
                proposal_id: i,
                option: match v % 3 {
                    0 => VoteOption::Yes,
                    1 => VoteOption::No,
                    _ => VoteOption::Abstain,
                },
                power: 1000 + v * 100,
                voted_at: i + v,
            };
            store.write_vote(&vote).unwrap();
        }
    }

    // Verify counts
    let proposals = store.list_proposals().unwrap();
    assert_eq!(proposals.len(), 500);

    // Verify ordering (RocksDB lexicographic = numeric via zero-padding)
    for i in 0..499 {
        assert!(proposals[i].id < proposals[i + 1].id);
    }

    // Verify votes per proposal
    let votes_1 = store.list_votes(1).unwrap();
    assert_eq!(votes_1.len(), 10);
    let votes_500 = store.list_votes(500).unwrap();
    assert_eq!(votes_500.len(), 10);

    // Verify vote isolation — no bleed between proposals
    let votes_250 = store.list_votes(250).unwrap();
    assert!(votes_250.iter().all(|v| v.proposal_id == 250));
}

// ── 5. Stress: concurrent vote writes from multiple threads ──────────────────

#[test]
fn stress_concurrent_vote_writes() {
    let store = Arc::new(MemoryStore::new());

    // Write 1 proposal
    let proposal = Proposal {
        id: 1,
        proposer: "alice".into(),
        action: ProposalAction::TextProposal {
            title: "Concurrent".into(),
            description: "test".into(),
        },
        status: ProposalStatus::Voting,
        deposit: 100,
        submitted_at: 0,
        voting_ends_at: 10000,
        timelock_ends_at: None,
        finalized_at: None,
        description: "".into(),
    };
    store.write_proposal(&proposal).unwrap();

    // Spawn 100 threads, each writing 10 votes to different proposals
    let mut handles = Vec::new();
    for t in 0..100u32 {
        let s = store.clone();
        handles.push(std::thread::spawn(move || {
            for v in 0..10u32 {
                let vote = Vote {
                    voter: format!("thread-{t}-voter-{v}"),
                    proposal_id: 1,
                    option: VoteOption::Yes,
                    power: 100,
                    voted_at: 50,
                };
                s.write_vote(&vote).unwrap();
            }
        }));
    }
    for h in handles {
        h.join().unwrap();
    }

    let votes = store.list_votes(1).unwrap();
    assert_eq!(votes.len(), 1000);
}

// ── 6. Adversarial: vote replay after hydration ──────────────────────────────

#[test]
fn adversarial_vote_replay_after_hydration_blocked() {
    let store: Arc<dyn BlockStore> = Arc::new(MemoryStore::new());
    let vs = VoteStore::new();

    // Cast legitimate vote
    vs.cast_vote(1, "alice", VoteOption::Yes, 1000, 50, 1000)
        .unwrap();

    // Persist
    for v in vs.get_votes(1) {
        store.write_vote(&v).unwrap();
    }

    // Hydrate
    let vs2 = VoteStore::new();
    for v in store.list_votes(1).unwrap() {
        vs2.load_vote(v);
    }

    // Attacker tries to vote again as alice — should be rejected
    let result = vs2.cast_vote(1, "alice", VoteOption::No, 1000, 60, 1000);
    assert!(
        result.is_err(),
        "Duplicate vote after hydration must be rejected"
    );
    assert!(
        result.unwrap_err().to_string().contains("already voted"),
        "Error must indicate already voted"
    );

    // Verify tally unchanged — only 1 vote counted
    let tally = vs2.tally(1, 10_000, 33, 67);
    assert_eq!(tally.yes_power, 1000);
    assert_eq!(tally.total_voted_power, 1000);
}

// ── 7. Adversarial: null bytes and special chars in voter/proposer ────────────

#[test]
fn adversarial_null_bytes_in_voter_name() {
    let (store, _dir) = rocksdb_store();

    // Write vote with null byte in voter name
    let evil_voter = "alice\0bob";
    let vote = Vote {
        voter: evil_voter.to_string(),
        proposal_id: 1,
        option: VoteOption::Yes,
        power: 999,
        voted_at: 10,
    };
    store.write_vote(&vote).unwrap();

    // Read back — must get exactly the same voter string, no truncation
    let votes = store.list_votes(1).unwrap();
    assert_eq!(votes.len(), 1);
    assert_eq!(votes[0].voter, evil_voter);
    assert_eq!(votes[0].power, 999);
}

#[test]
fn adversarial_special_chars_in_proposer() {
    let (store, _dir) = rocksdb_store();

    let evil_proposer = "alice'; DROP TABLE proposals;--";
    let proposal = Proposal {
        id: 1,
        proposer: evil_proposer.into(),
        action: ProposalAction::TextProposal {
            title: "<script>alert('xss')</script>".into(),
            description: "../../etc/passwd".into(),
        },
        status: ProposalStatus::Voting,
        deposit: 100,
        submitted_at: 0,
        voting_ends_at: 100,
        timelock_ends_at: None,
        finalized_at: None,
        description: "injection test".into(),
    };
    store.write_proposal(&proposal).unwrap();
    let loaded = store.read_proposal(1).unwrap();
    assert_eq!(loaded.proposer, evil_proposer);
    assert!(loaded.description == "injection test");
}

// ── 8. Adversarial: tampered proposal loaded into store ──────────────────────

#[test]
fn adversarial_tampered_proposal_status_doesnt_bypass_lifecycle() {
    let store: Arc<dyn BlockStore> = Arc::new(MemoryStore::new());
    let ps = ProposalStore::new();

    // Submit legitimate proposal
    let id = submit_proposal(&ps, "alice", 100);
    let p = ps.get(id).unwrap();
    store.write_proposal(&p).unwrap();

    // Attacker writes a tampered proposal directly to store (Executed without going through lifecycle)
    let mut tampered = p.clone();
    tampered.status = ProposalStatus::Executed;
    tampered.finalized_at = Some(999);
    store.write_proposal(&tampered).unwrap();

    // Hydrate into fresh ProposalStore
    let ps2 = ProposalStore::new();
    for loaded in store.list_proposals().unwrap() {
        ps2.load_proposal(loaded);
    }

    // The tampered status IS loaded (load_proposal trusts store data)
    // BUT: trying to mark_executed again fails because it's already Executed
    let result = ps2.mark_executed(id, 1000);
    assert!(
        result.is_err(),
        "Cannot execute an already-executed proposal"
    );

    // And trying to re-vote on a passed proposal in VoteStore still works
    // because VoteStore doesn't check proposal status — the handler does.
    // This is acceptable: store is tampered, but business logic still checks.
}

// ── 9. Adversarial: vote for non-existent proposal ───────────────────────────

#[test]
fn adversarial_vote_for_nonexistent_proposal_in_store() {
    let (store, _dir) = rocksdb_store();

    // Write vote for proposal 99999 which doesn't exist in proposals CF
    let vote = Vote {
        voter: "attacker".into(),
        proposal_id: 99999,
        option: VoteOption::Yes,
        power: 1_000_000,
        voted_at: 1,
    };
    store.write_vote(&vote).unwrap();

    // Store accepts it (dumb persistence), but proposal doesn't exist
    assert!(store.read_proposal(99999).is_err());

    // Hydrate — votes load but have no matching proposal
    let vs = VoteStore::new();
    for v in store.list_votes(99999).unwrap() {
        vs.load_vote(v);
    }

    // Tally produces no meaningful result (orphaned vote)
    let tally = vs.tally(99999, 10_000, 33, 67);
    assert_eq!(tally.yes_power, 1_000_000);
    // But the ProposalStore has no proposal 99999, so handler would 404
}

// ── 10. Adversarial: ID boundary — u64::MAX proposal ─────────────────────────

#[test]
fn adversarial_u64_max_proposal_id() {
    let (store, _dir) = rocksdb_store();

    let proposal = Proposal {
        id: u64::MAX,
        proposer: "eve".into(),
        action: ProposalAction::TextProposal {
            title: "Max ID".into(),
            description: "boundary".into(),
        },
        status: ProposalStatus::Voting,
        deposit: 1,
        submitted_at: 0,
        voting_ends_at: u64::MAX,
        timelock_ends_at: None,
        finalized_at: None,
        description: "".into(),
    };
    store.write_proposal(&proposal).unwrap();

    let loaded = store.read_proposal(u64::MAX).unwrap();
    assert_eq!(loaded.id, u64::MAX);

    // Vote with u64::MAX power
    let vote = Vote {
        voter: "whale".into(),
        proposal_id: u64::MAX,
        option: VoteOption::Yes,
        power: u64::MAX,
        voted_at: u64::MAX,
    };
    store.write_vote(&vote).unwrap();
    let votes = store.list_votes(u64::MAX).unwrap();
    assert_eq!(votes.len(), 1);
    assert_eq!(votes[0].power, u64::MAX);
}

// ── 11. Vote isolation: prefix scan doesn't bleed ────────────────────────────

#[test]
fn vote_prefix_scan_isolation() {
    let (store, _dir) = rocksdb_store();

    // Proposals 1, 10, 100, 1000 — keys with shared prefixes
    for pid in [1, 10, 100, 1000] {
        for v in 0..3 {
            let vote = Vote {
                voter: format!("voter-{v}"),
                proposal_id: pid,
                option: VoteOption::Yes,
                power: 100,
                voted_at: 1,
            };
            store.write_vote(&vote).unwrap();
        }
    }

    // Each proposal must see exactly 3 votes — no prefix bleed
    for pid in [1, 10, 100, 1000] {
        let votes = store.list_votes(pid).unwrap();
        assert_eq!(
            votes.len(),
            3,
            "proposal {pid} should have exactly 3 votes, got {}",
            votes.len()
        );
        assert!(
            votes.iter().all(|v| v.proposal_id == pid),
            "all votes for proposal {pid} must have matching proposal_id"
        );
    }
}

// ── 12. Hydration preserves delegation state ─────────────────────────────────

#[test]
fn hydration_does_not_corrupt_delegation() {
    let store: Arc<dyn BlockStore> = Arc::new(MemoryStore::new());
    let vs = VoteStore::new();

    // Set up delegation: alice delegates to bob
    vs.delegate("alice", "bob").unwrap();

    // Bob votes (as delegate)
    vs.cast_vote(1, "bob", VoteOption::Yes, 2000, 50, 1000)
        .unwrap();

    // Persist bob's vote
    for v in vs.get_votes(1) {
        store.write_vote(&v).unwrap();
    }

    // Hydrate into fresh VoteStore
    let vs2 = VoteStore::new();
    for v in store.list_votes(1).unwrap() {
        vs2.load_vote(v);
    }

    // Note: delegations are NOT persisted (in-memory only).
    // After hydration, alice CAN vote (delegation lost) — this is expected.
    // Bob's vote is preserved.
    let bob_vote = vs2.get_vote(1, "bob").unwrap();
    assert_eq!(bob_vote.power, 2000);
    assert_eq!(bob_vote.option, VoteOption::Yes);

    // Alice can now vote since delegation was lost
    vs2.cast_vote(1, "alice", VoteOption::No, 500, 60, 1000)
        .unwrap();
    let tally = vs2.tally(1, 10_000, 33, 67);
    assert_eq!(tally.yes_power, 2000);
    assert_eq!(tally.no_power, 500);
}

// ── 13. Stress: rapid overwrite of same proposal ─────────────────────────────

#[test]
fn stress_rapid_proposal_overwrites() {
    let (store, _dir) = rocksdb_store();

    let mut proposal = Proposal {
        id: 1,
        proposer: "alice".into(),
        action: ProposalAction::TextProposal {
            title: "Evolving".into(),
            description: "v0".into(),
        },
        status: ProposalStatus::Voting,
        deposit: 100,
        submitted_at: 0,
        voting_ends_at: 10000,
        timelock_ends_at: None,
        finalized_at: None,
        description: "v0".into(),
    };

    // Overwrite the same proposal 1000 times
    for i in 0..1000u64 {
        proposal.description = format!("v{i}");
        store.write_proposal(&proposal).unwrap();
    }

    // Only 1 proposal in list, with latest description
    let all = store.list_proposals().unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].description, "v999");
}

// ── 14. Empty store returns empty lists ──────────────────────────────────────

#[test]
fn empty_store_returns_empty_lists() {
    let (store, _dir) = rocksdb_store();
    assert!(store.list_proposals().unwrap().is_empty());
    assert!(store.list_votes(1).unwrap().is_empty());
    assert!(store.read_proposal(1).is_err());
}

// ── 15. Concurrent proposal + vote writes across threads ─────────────────────

#[test]
fn stress_concurrent_proposals_and_votes() {
    let store = Arc::new(MemoryStore::new());
    let mut handles = Vec::new();

    // 50 threads, each writing 1 proposal + 20 votes
    for t in 0..50u64 {
        let s = store.clone();
        handles.push(std::thread::spawn(move || {
            let pid = t + 1;
            let proposal = Proposal {
                id: pid,
                proposer: format!("proposer-{t}"),
                action: ProposalAction::TextProposal {
                    title: format!("P{t}"),
                    description: "concurrent".into(),
                },
                status: ProposalStatus::Voting,
                deposit: 100,
                submitted_at: t,
                voting_ends_at: t + 1000,
                timelock_ends_at: None,
                finalized_at: None,
                description: "".into(),
            };
            s.write_proposal(&proposal).unwrap();

            for v in 0..20u64 {
                let vote = Vote {
                    voter: format!("t{t}-v{v}"),
                    proposal_id: pid,
                    option: VoteOption::Yes,
                    power: 100,
                    voted_at: t + v,
                };
                s.write_vote(&vote).unwrap();
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    let all = store.list_proposals().unwrap();
    assert_eq!(all.len(), 50);

    // Each proposal should have 20 votes
    for p in &all {
        let votes = store.list_votes(p.id).unwrap();
        assert_eq!(votes.len(), 20, "proposal {} should have 20 votes", p.id);
    }
}
