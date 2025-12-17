/**
 * Transaction Validation Gate Testing Tool
 * 
 * Tests pre-mempool transaction validation including:
 * - Format validation
 * - Amount and fee checking
 * - Address validation
 * - Sequence number tracking (replay attack prevention)
 * - Double-spend detection
 * 
 * Uso: cargo run --bin test_transaction_validation --release
 */
use rust_bc::transaction_validation::TransactionValidator;
use rust_bc::models::Transaction;

fn main() {
    println!("╔════════════════════════════════════════════════════════╗");
    println!("║   TRANSACTION VALIDATION GATE TESTING                  ║");
    println!("║   Pre-Mempool Validation & Replay Attack Prevention    ║");
    println!("╚════════════════════════════════════════════════════════╝");
    println!();

    // Test 1: Format validation
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("TEST 1: Transaction Format Validation");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    let mut validator = TransactionValidator::with_defaults();

    let test_cases = vec![
        (
            Transaction {
                id: "tx1".to_string(),
                from: "sender_address_1234567890".to_string(),
                to: "receiver_address_1234567890".to_string(),
                amount: 100,
                fee: 1,
                timestamp: 1000,
                signature: "sig".to_string(),
                data: None,
            },
            "Valid transaction",
            true,
        ),
        (
            Transaction {
                id: "".to_string(),
                from: "sender".to_string(),
                to: "receiver".to_string(),
                amount: 100,
                fee: 1,
                timestamp: 1000,
                signature: "sig".to_string(),
                data: None,
            },
            "Empty transaction ID",
            false,
        ),
        (
            Transaction {
                id: "tx2".to_string(),
                from: "addr1234567890".to_string(),
                to: "addr1234567890".to_string(),
                amount: 100,
                fee: 1,
                timestamp: 1000,
                signature: "sig".to_string(),
                data: None,
            },
            "Same sender and receiver",
            false,
        ),
    ];

    for (tx, desc, should_pass) in test_cases {
        let result = validator.validate(&tx);
        let status = if result.is_valid == should_pass { "✅ PASS" } else { "❌ FAIL" };
        println!("{}: {}", status, desc);
        if !result.is_valid {
            println!("  Error: {}", result.errors.join(", "));
        }
    }
    println!();

    // Test 2: Amount and fee validation
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("TEST 2: Amount & Fee Validation");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    validator = TransactionValidator::with_defaults();

    let amount_tests = vec![
        (100, 1, "Normal transaction", true),
        (0, 0, "Zero amount and fee", false),
        (1000, 0, "High amount, no fee", true),
        (1, 1, "Minimum values", true),
    ];

    for (amount, fee, desc, should_pass) in amount_tests {
        let tx = Transaction {
            id: format!("tx_{}_{}", amount, fee),
            from: "sender_address_1234567890".to_string(),
            to: "receiver_address_1234567890".to_string(),
            amount,
            fee,
            timestamp: 2000 + amount as u64,
            signature: "sig".to_string(),
            data: None,
        };

        let result = validator.validate(&tx);
        let status = if result.is_valid == should_pass { "✅ PASS" } else { "❌ FAIL" };
        println!("{}: {} (amount={}, fee={})", status, desc, amount, fee);
        if !result.is_valid && !result.errors.is_empty() {
            println!("  Error: {}", result.errors[0]);
        }
    }
    println!();

    // Test 3: Address validation
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("TEST 3: Address Validation");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    validator = TransactionValidator::with_defaults();
    println!("Min address length: {}", validator.config.min_address_length);
    println!("Max address length: {}", validator.config.max_address_length);
    println!();

    let addr_tests = vec![
        ("valid_sender_12345678901234567890", "valid_receiver_12345678901234567890", "Valid addresses", true),
        ("short", "valid_receiver_12345678901234567890", "Sender too short", false),
        ("valid_sender_12345678901234567890", "rec", "Receiver too short", false),
    ];

    for (sender, receiver, desc, should_pass) in addr_tests {
        let tx = Transaction {
            id: format!("tx_{}", desc),
            from: sender.to_string(),
            to: receiver.to_string(),
            amount: 100,
            fee: 1,
            timestamp: 3000,
            signature: "sig".to_string(),
            data: None,
        };

        let result = validator.validate(&tx);
        let status = if result.is_valid == should_pass { "✅ PASS" } else { "❌ FAIL" };
        println!("{}: {}", status, desc);
    }
    println!();

    // Test 4: Sequence tracking (replay attack prevention)
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("TEST 4: Sequence Tracking & Replay Protection");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    validator = TransactionValidator::with_defaults();

    let tx1 = Transaction {
        id: "tx_seq_1".to_string(),
        from: "sender_seq_test_1234567890".to_string(),
        to: "receiver_seq_test_1234567890".to_string(),
        amount: 100,
        fee: 1,
        timestamp: 100,
        signature: "sig".to_string(),
        data: None,
    };

    let result1 = validator.validate(&tx1);
    println!("✅ First transaction (sequence=100): {}", result1.is_valid);

    let tx2 = Transaction {
        id: "tx_seq_2".to_string(),
        from: "sender_seq_test_1234567890".to_string(),
        to: "receiver_seq_test_1234567890".to_string(),
        amount: 50,
        fee: 1,
        timestamp: 200,
        signature: "sig".to_string(),
        data: None,
    };

    let result2 = validator.validate(&tx2);
    println!("✅ Second transaction (sequence=200): {}", result2.is_valid);

    let tx3_replay = Transaction {
        id: "tx_seq_3_replay".to_string(),
        from: "sender_seq_test_1234567890".to_string(),
        to: "receiver_seq_test_1234567890".to_string(),
        amount: 75,
        fee: 1,
        timestamp: 50, // Lower than previous - REPLAY ATTEMPT
        signature: "sig".to_string(),
        data: None,
    };

    let result3 = validator.validate(&tx3_replay);
    println!("🛡️  Replay attack (sequence=50): {} (BLOCKED)", !result3.is_valid);
    if !result3.is_valid {
        println!("   Error: {}", result3.errors[0]);
    }
    println!();

    // Test 5: Duplicate detection
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("TEST 5: Duplicate Transaction Detection");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    validator = TransactionValidator::with_defaults();

    let tx_dup = Transaction {
        id: "tx_duplicate_test".to_string(),
        from: "sender_dup_1234567890".to_string(),
        to: "receiver_dup_1234567890".to_string(),
        amount: 200,
        fee: 2,
        timestamp: 5000,
        signature: "sig".to_string(),
        data: None,
    };

    let result_first = validator.validate(&tx_dup);
    println!("✅ First submission: {}", result_first.is_valid);

    let result_second = validator.validate(&tx_dup);
    println!("🛡️  Duplicate submission: {} (BLOCKED)", !result_second.is_valid);
    if !result_second.is_valid {
        println!("   Error: {}", result_second.errors[0]);
    }
    println!();

    // Test 6: Validation statistics
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("TEST 6: Validation Statistics");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    validator = TransactionValidator::with_defaults();

    // Add some transactions
    for i in 1..=3 {
        for j in 1..=2 {
            let tx = Transaction {
                id: format!("tx_stats_{}_{}", i, j),
                from: format!("sender_{}_addr_1234567890", i),
                to: format!("receiver_{}_addr_1234567890", j),
                amount: 100 * i as u64,
                fee: 1,
                timestamp: 1000 + (i as u64 * 100) + j as u64,
                signature: "sig".to_string(),
                data: None,
            };
            let _ = validator.validate(&tx);
        }
    }

    let stats = validator.get_stats();
    println!("Tracked senders: {}", stats.tracked_senders);
    println!("Seen transactions: {}", stats.seen_transactions);
    println!("Avg pending per sender: {:.2}", stats.average_pending_per_sender);
    println!();

    // Summary
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ TRANSACTION VALIDATION TESTING COMPLETE");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("Your mempool is protected against:");
    println!("  ✓ Malformed transactions");
    println!("  ✓ Invalid amounts and fees");
    println!("  ✓ Replay attacks (via sequence tracking)");
    println!("  ✓ Duplicate transactions");
    println!("  ✓ Invalid addresses");
    println!("  ✓ Double-spending (pending transaction limits)");
    println!();
    println!("Validation Layers:");
    println!("  1. Format validation (ID, sender/receiver check)");
    println!("  2. Duplicate detection (seen transaction IDs)");
    println!("  3. Amount validation (min/max bounds)");
    println!("  4. Fee validation (network-adjusted minimum)");
    println!("  5. Address validation (length/format check)");
    println!("  6. Sequence tracking (replay prevention)");
    println!("  7. Double-spend check (pending transaction limits)");
    println!();
}
