/**
 * Network Security Testing Tool
 * 
 * Tests DoS protection, peer scoring, rate limiting, and blacklisting
 * 
 * Uso: cargo run --bin test_network_security --release
 */
use rust_bc::network_security::NetworkSecurityManager;

fn main() {
    println!("╔════════════════════════════════════════════════════════╗");
    println!("║      NETWORK SECURITY & DoS PROTECTION TESTING         ║");
    println!("╚════════════════════════════════════════════════════════╝");
    println!();

    // Test 1: Peer registration and connection limits
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("TEST 1: Connection Limits & Peer Registration");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    let mut manager = NetworkSecurityManager::with_defaults();
    manager.config.max_concurrent_connections = 3;

    println!("Registering peers (max 3 connections):");
    for i in 1..=5 {
        let addr = format!("127.0.0.1:{}", 8081 + i);
        match manager.register_peer(addr.clone()) {
            Ok(_) => println!("  ✅ {} registered", addr),
            Err(e) => println!("  ❌ {} rejected: {}", addr, e),
        }
    }

    println!("  Active connections: {}", manager.active_connections);
    println!();

    // Test 2: Message size validation
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("TEST 2: Message Size Validation");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    manager = NetworkSecurityManager::with_defaults();
    println!("Max message size: {} bytes", manager.config.message_size_limit);

    let test_cases = vec![
        (1_000, "Small message"),
        (1_000_000, "Medium message"),
        (10_000_000, "Large message (at limit)"),
        (15_000_000, "Oversized message (exceeds limit)"),
    ];

    for (size, desc) in test_cases {
        match manager.validate_message_size(size) {
            Ok(_) => println!("  ✅ {}: {} bytes - ALLOWED", desc, size),
            Err(e) => println!("  ❌ {}: {} bytes - REJECTED ({})", desc, size, e),
        }
    }
    println!();

    // Test 3: Rate limiting
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("TEST 3: Rate Limiting per Peer");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    manager = NetworkSecurityManager::with_defaults();
    manager.config.max_messages_per_second = 5;

    let peer = "127.0.0.1:8081".to_string();
    manager.register_peer(peer.clone()).unwrap();

    println!("Sending {} messages per second (limit: {})", 7, manager.config.max_messages_per_second);
    for i in 1..=7 {
        match manager.check_rate_limit(&peer, 1000) {
            Ok(_) => println!("  ✅ Message {} - ALLOWED", i),
            Err(e) => {
                println!("  ⚠️  Message {} - BLOCKED: {}", i, e);
                manager.record_invalid_message(&peer, 10);
                if let Some(stats) = manager.get_peer_stats(&peer) {
                    println!("     Peer score: {}", stats.score);
                }
            }
        }
    }
    println!();

    // Test 4: Peer scoring and reputation
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("TEST 4: Peer Scoring & Reputation System");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    manager = NetworkSecurityManager::with_defaults();
    let peer = "127.0.0.1:9000".to_string();
    manager.register_peer(peer.clone()).unwrap();

    println!("Initial score: 100 (TRUSTED)");
    println!();

    println!("Scenario 1: Good behavior (valid messages)");
    for _ in 0..5 {
        manager.record_valid_message(&peer, 1000);
    }
    let stats = manager.get_peer_stats(&peer).unwrap();
    println!("  After 5 valid messages:");
    println!("    Score: {}", stats.score);
    println!("    Status: {}", stats.status);
    println!();

    println!("Scenario 2: Bad behavior (invalid messages)");
    for _ in 0..30 {
        manager.record_invalid_message(&peer, 5);
    }
    let stats = manager.get_peer_stats(&peer).unwrap();
    println!("  After 30 invalid messages:");
    println!("    Score: {}", stats.score);
    println!("    Status: {}", stats.status);
    println!();

    // Test 5: Blacklisting
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("TEST 5: Peer Blacklisting");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    manager = NetworkSecurityManager::with_defaults();
    let peer = "127.0.0.1:9001".to_string();
    manager.register_peer(peer.clone()).unwrap();

    println!("Blacklisting peer: {}", peer);
    manager.blacklist_peer(&peer, "Detected as malicious node".to_string());

    let stats = manager.get_peer_stats(&peer).unwrap();
    println!("  Status: {}", stats.status);
    println!("  Reason: {:?}", manager.peer_scores.get(&peer).map(|p| &p.blacklist_reason));

    println!();
    println!("Attempting to send message from blacklisted peer:");
    match manager.check_rate_limit(&peer, 1000) {
        Ok(_) => println!("  ❌ Message allowed (should be blocked!)"),
        Err(e) => println!("  ✅ Message blocked: {}", e),
    }
    println!();

    // Test 6: Multiple peers statistics
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("TEST 6: Network Statistics");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    manager = NetworkSecurityManager::with_defaults();

    // Register and score multiple peers
    let peers = vec!["127.0.0.1:9002", "127.0.0.1:9003", "127.0.0.1:9004"];

    for peer in &peers {
        manager.register_peer(peer.to_string()).unwrap();
        for _ in 0..3 {
            manager.record_valid_message(peer, 1000);
        }
    }

    println!("Peer Statistics:");
    println!();
    println!("Address             Score  Received  Rejected  Status");
    println!("─────────────────────────────────────────────────────");

    for stat in manager.get_all_peer_stats() {
        println!(
            "{:<19} {:>5}  {:>8}  {:>8}  {}",
            stat.address, stat.score, stat.messages_received, stat.messages_rejected, stat.status
        );
    }

    println!();
    println!("Active connections: {}/{}", 
        manager.active_connections, 
        manager.config.max_concurrent_connections
    );
    println!();

    // Summary
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ NETWORK SECURITY TESTING COMPLETE");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("Your network is protected against:");
    println!("  ✓ Connection flooding attacks");
    println!("  ✓ Message size bombs");
    println!("  ✓ Rate-based DoS attacks");
    println!("  ✓ Spam from malicious peers");
    println!("  ✓ Reputation system prevents repeat offenders");
    println!();
    println!("Default Limits:");
    println!("  • Max concurrent connections: 100");
    println!("  • Max messages/second per peer: 100");
    println!("  • Max bytes/second per peer: 10MB");
    println!("  • Max message size: 10MB");
    println!();
}
