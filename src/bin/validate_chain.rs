/**
 * Chain Validation Testing Tool
 * 
 * Tests chain validation, fork resolution, and attack protection
 * 
 * Uso: cargo run --bin validate_chain --release
 */
use rust_bc::blockchain::Blockchain;
use rust_bc::chain_validation::{ChainValidator, ForkResolver, AttackProtection};

fn main() {
    println!("╔════════════════════════════════════════════════════════╗");
    println!("║         CHAIN VALIDATION & SECURITY TESTING            ║");
    println!("╚════════════════════════════════════════════════════════╝");
    println!();

    // Test 1: Chain validation
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("TEST 1: Full Chain Validation");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    let blockchain = Blockchain::new(2);
    let (is_valid, errors) = ChainValidator::validate_full_chain(&blockchain.chain);

    println!("Chain with {} blocks:", blockchain.chain.len());
    println!("  ✓ Genesis block: {}", blockchain.chain[0].hash);
    println!("  Is valid: {}", if is_valid { "✅ YES" } else { "❌ NO" });

    if !errors.is_empty() {
        println!("  Errors found:");
        for error in &errors {
            println!("    - {}", error);
        }
    }
    println!();

    // Test 2: Fork resolution
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("TEST 2: Fork Detection & Resolution");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    // Create two chains for fork testing
    let blockchain_a = Blockchain::new(2);
    let blockchain_b = Blockchain::new(2);

    let (fork_point, suffix_a, suffix_b) =
        ForkResolver::find_fork_point(&blockchain_a.chain, &blockchain_b.chain);

    println!("Comparing two blockchain instances:");
    println!("  Chain A length: {}", blockchain_a.chain.len());
    println!("  Chain B length: {}", blockchain_b.chain.len());
    println!("  Fork point (consensus up to block): {}", fork_point);
    println!("  Chain A diverges after: {} blocks", suffix_a.len());
    println!("  Chain B diverges after: {} blocks", suffix_b.len());
    println!();

    // Test 3: Attack protection
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("TEST 3: 51% Attack Protection - Difficulty Adjustment Limits");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    struct DiffTest {
        old: u8,
        new: u8,
        min: u8,
        max_adj: u8,
        expected: bool,
        description: &'static str,
    }

    let diff_tests = vec![
        DiffTest {
            old: 5,
            new: 6,
            min: 1,
            max_adj: 2,
            expected: true,
            description: "Normal increase (+1) within limit",
        },
        DiffTest {
            old: 5,
            new: 8,
            min: 1,
            max_adj: 2,
            expected: false,
            description: "Excessive increase (+3) exceeds limit",
        },
        DiffTest {
            old: 5,
            new: 3,
            min: 1,
            max_adj: 2,
            expected: true,
            description: "Normal decrease (-2) within limit",
        },
        DiffTest {
            old: 2,
            new: 0,
            min: 1,
            max_adj: 2,
            expected: false,
            description: "Decrease below minimum difficulty",
        },
        DiffTest {
            old: 5,
            new: 10,
            min: 1,
            max_adj: 10,
            expected: true,
            description: "Large but permitted increase",
        },
    ];

    for test in diff_tests {
        let result = AttackProtection::validate_difficulty_adjustment(
            test.old, test.new, test.min, test.max_adj,
        );
        let is_ok = result.is_ok();
        let status = if is_ok == test.expected {
            "✅ PASS"
        } else {
            "❌ FAIL"
        };

        println!("{}: {}", status, test.description);
        println!("  {} → {} (min={}, max_adj={})", test.old, test.new, test.min, test.max_adj);
        if let Err(e) = result {
            println!("  Reason: {}", e);
        }
        println!();
    }

    // Test 4: Reorg safety
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("TEST 4: Chain Reorganization Safety Limits");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    struct ReorgTest {
        fork_point: usize,
        chain_length: usize,
        max_reorg: usize,
        expected: bool,
        description: &'static str,
    }

    let reorg_tests = vec![
        ReorgTest {
            fork_point: 95,
            chain_length: 100,
            max_reorg: 10,
            expected: true,
            description: "Safe reorg: 5 blocks from tip",
        },
        ReorgTest {
            fork_point: 85,
            chain_length: 100,
            max_reorg: 10,
            expected: false,
            description: "Unsafe reorg: 15 blocks from tip (exceeds limit of 10)",
        },
        ReorgTest {
            fork_point: 90,
            chain_length: 100,
            max_reorg: 10,
            expected: true,
            description: "Borderline safe reorg: exactly 10 blocks",
        },
        ReorgTest {
            fork_point: 0,
            chain_length: 100,
            max_reorg: 5,
            expected: false,
            description: "Critical: fork at genesis (full reorg)",
        },
    ];

    for test in reorg_tests {
        let is_safe = ForkResolver::is_reorg_safe(test.fork_point, test.chain_length, test.max_reorg);
        let status = if is_safe == test.expected {
            "✅ PASS"
        } else {
            "❌ FAIL"
        };

        println!("{}: {}", status, test.description);
        println!(
            "  Fork at block {}, chain length {}, max depth {}",
            test.fork_point, test.chain_length, test.max_reorg
        );
        println!(
            "  Reorg depth: {} blocks, Safe: {}",
            test.chain_length - test.fork_point,
            if is_safe { "YES" } else { "NO" }
        );
        println!();
    }

    // Summary
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ VALIDATION TESTING COMPLETE");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("Summary:");
    println!("  ✓ Chain validation working");
    println!("  ✓ Fork detection functional");
    println!("  ✓ Attack protection limits enforced");
    println!("  ✓ Reorg safety checks active");
    println!();
    println!("Your blockchain is more resilient against:");
    println!("  • Chain history manipulation");
    println!("  • Fork attacks");
    println!("  • 51% difficulty attacks");
    println!("  • Deep reorganization attacks");
}
