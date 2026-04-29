//! Faucet abuse prevention tests.

use rust_bc::testnet::faucet::{Faucet, FaucetConfig, FaucetError};

fn test_faucet() -> Faucet {
    Faucet::new(FaucetConfig {
        drip_amount: 100,
        cooldown_blocks: 10,
        max_total: 1000,
        enabled: true,
        max_drips_per_ip_per_day: 3,
        max_daily_total: 500,
    })
}

// ── Cooldown ───────────────────────────────────────────────────────────────

#[test]
fn cooldown_enforced_per_address() {
    let faucet = test_faucet();
    faucet.drip("alice", 0).unwrap();

    // Within cooldown (10 blocks)
    let err = faucet.drip("alice", 5).unwrap_err();
    assert!(matches!(err, FaucetError::Cooldown { .. }));

    // After cooldown
    faucet.drip("alice", 10).unwrap();
}

#[test]
fn cooldown_independent_per_address() {
    let faucet = test_faucet();
    faucet.drip("alice", 0).unwrap();
    // Bob unaffected by Alice's cooldown
    faucet.drip("bob", 1).unwrap();
}

// ── IP Rate Limit ──────────────────────────────────────────────────────────

#[test]
fn ip_limit_enforced() {
    let faucet = test_faucet();
    // 3 drips per IP per day
    faucet.drip_with_ip("addr1", 0, "1.2.3.4", 1).unwrap();
    faucet.drip_with_ip("addr2", 10, "1.2.3.4", 1).unwrap();
    faucet.drip_with_ip("addr3", 20, "1.2.3.4", 1).unwrap();

    // 4th from same IP → rejected
    let err = faucet.drip_with_ip("addr4", 30, "1.2.3.4", 1).unwrap_err();
    assert!(matches!(err, FaucetError::IpLimitExceeded { .. }));
}

#[test]
fn ip_limit_resets_on_new_day() {
    let faucet = test_faucet();
    for i in 0..3 {
        faucet
            .drip_with_ip(&format!("a{i}"), i * 10, "1.2.3.4", 1)
            .unwrap();
    }
    // Exceeded on day 1
    assert!(faucet.drip_with_ip("a3", 30, "1.2.3.4", 1).is_err());

    // Day 2: reset
    faucet.drip_with_ip("a4", 100, "1.2.3.4", 2).unwrap();
}

#[test]
fn different_ips_independent() {
    let faucet = test_faucet();
    for i in 0..3 {
        faucet
            .drip_with_ip(&format!("a{i}"), i * 10, "1.1.1.1", 1)
            .unwrap();
    }
    // Different IP still works
    faucet.drip_with_ip("b0", 0, "2.2.2.2", 1).unwrap();
}

// ── Daily Cap ──────────────────────────────────────────────────────────────

#[test]
fn daily_cap_enforced() {
    let faucet = Faucet::new(FaucetConfig {
        drip_amount: 200,
        cooldown_blocks: 0, // no cooldown for this test
        max_total: 0,       // no total cap
        enabled: true,
        max_drips_per_ip_per_day: 0, // no IP limit
        max_daily_total: 500,
    });

    // 200 + 200 = 400 OK
    faucet.drip_with_ip("a", 0, "ip", 1).unwrap();
    faucet.drip_with_ip("b", 1, "ip", 1).unwrap();

    // 400 + 200 = 600 > 500 → rejected
    let err = faucet.drip_with_ip("c", 2, "ip", 1).unwrap_err();
    assert!(matches!(err, FaucetError::DailyCapReached { .. }));

    // Next day OK
    faucet.drip_with_ip("d", 100, "ip", 2).unwrap();
}

// ── Total Depletion ────────────────────────────────────────────────────────

#[test]
fn total_depletion_stops_drips() {
    let faucet = Faucet::new(FaucetConfig {
        drip_amount: 100,
        cooldown_blocks: 0,
        max_total: 250, // only 2.5 drips worth
        enabled: true,
        max_drips_per_ip_per_day: 0,
        max_daily_total: 0,
    });

    faucet.drip("a", 0).unwrap();
    faucet.drip("b", 1).unwrap();

    // 200 distributed, 50 remaining < 100 drip → depleted
    let err = faucet.drip("c", 2).unwrap_err();
    assert!(matches!(err, FaucetError::Depleted { .. }));
}

// ── Disabled ───────────────────────────────────────────────────────────────

#[test]
fn disabled_faucet_rejects_all() {
    let faucet = Faucet::new(FaucetConfig {
        enabled: false,
        ..FaucetConfig::default()
    });
    let err = faucet.drip("alice", 0).unwrap_err();
    assert!(matches!(err, FaucetError::Disabled));
}

// ── Invalid Address ────────────────────────────────────────────────────────

#[test]
fn empty_address_rejected() {
    let faucet = test_faucet();
    let err = faucet.drip("", 0).unwrap_err();
    assert!(matches!(err, FaucetError::InvalidAddress));
}
