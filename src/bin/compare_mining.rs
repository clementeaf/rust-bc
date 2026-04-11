/**
 * Mining Comparison Benchmark
 *
 * Compares sequential vs parallel mining performance across difficulty levels
 * Measures speedup and efficiency
 *
 * Uso: cargo run --bin compare_mining --release -- [difficulty]
 */
use rust_bc::blockchain::{Block, Blockchain};
use rust_bc::models::WalletManager;
use std::time::Instant;

#[derive(Debug)]
struct ComparisonResult {
    difficulty: u8,
    sequential_time: f64,
    parallel_time: f64,
    speedup: f64,
}

fn main() {
    let difficulty: u8 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(3);

    println!("╔════════════════════════════════════════════════════════╗");
    println!("║   SEQUENTIAL vs PARALLEL MINING COMPARISON             ║");
    println!("║   Comparing performance across difficulty levels       ║");
    println!("╚════════════════════════════════════════════════════════╝");
    println!();

    let mut results = Vec::new();

    for diff in 1..=difficulty {
        println!("Testing difficulty {diff}...");

        // Sequential test
        let seq_time = benchmark_sequential(diff);
        println!("  ✓ Sequential: {seq_time:.4}s");

        // Parallel test
        let par_time = benchmark_parallel(diff);
        println!("  ✓ Parallel:   {par_time:.4}s");

        let speedup = seq_time / par_time;
        println!("  ⚡ Speedup:   {speedup:.2}x");
        println!();

        results.push(ComparisonResult {
            difficulty: diff,
            sequential_time: seq_time,
            parallel_time: par_time,
            speedup,
        });
    }

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("SUMMARY TABLE:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!(
        "{:>4} {:>15} {:>15} {:>12}",
        "Diff", "Sequential(s)", "Parallel(s)", "Speedup"
    );
    println!("{}", "─".repeat(50));

    let mut total_speedup = 0.0;
    for result in &results {
        println!(
            "{:>4} {:>15.4} {:>15.4} {:>12.2}x",
            result.difficulty, result.sequential_time, result.parallel_time, result.speedup
        );
        total_speedup += result.speedup;
    }

    let avg_speedup = total_speedup / results.len() as f64;
    println!("{}", "─".repeat(50));
    println!("Average Speedup: {avg_speedup:.2}x");
    println!();

    // Recommendations
    println!("📊 ANALYSIS:");
    println!();

    let num_cores = num_cpus::get();
    println!("System Cores: {num_cores}");

    if avg_speedup >= (num_cores as f64 * 0.8) {
        println!("✓ Excellent: Near-linear scaling achieved");
        println!("  → Parallel mining is highly efficient for this system");
    } else if avg_speedup >= 2.0 {
        println!("✓ Good: Significant speedup from parallelization");
        println!("  → Parallel mining provides substantial benefits");
    } else {
        println!("⚠ Note: Limited speedup. May have contention or overhead");
        println!("  → Sequential mining may be preferable for this workload");
    }

    println!();
    println!("💡 RECOMMENDATIONS:");

    // Find optimal difficulty with parallel mining
    let target_time = 30.0; // seconds
    let mut best_diff = results[0].difficulty;
    let mut best_diff_error = (results[0].parallel_time - target_time).abs();

    for result in &results {
        let error = (result.parallel_time - target_time).abs();
        if error < best_diff_error {
            best_diff_error = error;
            best_diff = result.difficulty;
        }
    }

    if let Some(result) = results.iter().find(|r| r.difficulty == best_diff) {
        println!("For 30-second target block time:");
        println!("  • Use difficulty: {best_diff}");
        println!("  • Expected time: {:.2}s", result.parallel_time);
        println!(
            "  • Expected speedup from parallelization: {:.2}x",
            result.speedup
        );
    }

    println!();
    println!("✅ COMPARISON COMPLETE!");
}

fn benchmark_sequential(difficulty: u8) -> f64 {
    let blockchain = Blockchain::new(difficulty);
    let mut wallet_manager = WalletManager::new();
    let wallet = wallet_manager.create_wallet();
    let miner_address = wallet.address.clone();

    let previous_hash = blockchain.get_latest_block().hash.clone();
    let index = blockchain.chain.len() as u64;
    let coinbase = Blockchain::create_coinbase_transaction(&miner_address, Some(50));
    let mut test_block = Block::new(index, vec![coinbase], previous_hash, difficulty);

    let start = Instant::now();
    test_block.mine_sequential();
    start.elapsed().as_secs_f64()
}

fn benchmark_parallel(difficulty: u8) -> f64 {
    let blockchain = Blockchain::new(difficulty);
    let mut wallet_manager = WalletManager::new();
    let wallet = wallet_manager.create_wallet();
    let miner_address = wallet.address.clone();

    let previous_hash = blockchain.get_latest_block().hash.clone();
    let index = blockchain.chain.len() as u64;
    let coinbase = Blockchain::create_coinbase_transaction(&miner_address, Some(50));
    let mut test_block = Block::new(index, vec![coinbase], previous_hash, difficulty);

    let start = Instant::now();
    test_block.mine_parallel();
    start.elapsed().as_secs_f64()
}
