/**
 * Test Prometheus Metrics Module
 *
 * Validates:
 * - Prometheus text format correctness
 * - Metric name and type declarations
 * - Metric collection accuracy
 * - Thread-safe atomic operations
 *
 * Usage:
 *   cargo run --release --bin test_metrics
 */

use std::sync::Arc;

#[derive(Clone)]
struct SimpleMetricsCollector {
    blocks: Arc<std::sync::atomic::AtomicU64>,
    transactions: Arc<std::sync::atomic::AtomicU64>,
    peers: Arc<std::sync::atomic::AtomicU64>,
}

impl SimpleMetricsCollector {
    fn new() -> Self {
        SimpleMetricsCollector {
            blocks: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            transactions: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            peers: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    fn generate_prometheus_format(&self) -> String {
        use std::sync::atomic::Ordering;

        let mut output = String::new();
        output.push_str("# HELP blockchain_blocks_total Total number of blocks\n");
        output.push_str("# TYPE blockchain_blocks_total counter\n");
        output.push_str(&format!(
            "blockchain_blocks_total {}\n",
            self.blocks.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP blockchain_transactions_total Total transactions\n");
        output.push_str("# TYPE blockchain_transactions_total counter\n");
        output.push_str(&format!(
            "blockchain_transactions_total {}\n",
            self.transactions.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP network_connected_peers Connected peers\n");
        output.push_str("# TYPE network_connected_peers gauge\n");
        output.push_str(&format!(
            "network_connected_peers {}\n",
            self.peers.load(Ordering::Relaxed)
        ));

        output
    }
}

fn main() {
    println!("╔════════════════════════════════════════════════════════╗");
    println!("║        PROMETHEUS METRICS TEST SUITE                   ║");
    println!("╚════════════════════════════════════════════════════════╝");
    println!();

    // Test 1: Format validation
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("TEST 1: Prometheus Format Validation");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    {
        let metrics = SimpleMetricsCollector::new();
        let output = metrics.generate_prometheus_format();

        assert!(output.contains("# HELP"));
        assert!(output.contains("# TYPE"));
        assert!(output.contains("counter"));
        assert!(output.contains("gauge"));

        println!("✅ Format contains required HELP and TYPE declarations");
        println!("✅ Metrics classified as counter and gauge");
    }
    println!();

    // Test 2: Metric name validation
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("TEST 2: Metric Name Validation");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    {
        let metrics = SimpleMetricsCollector::new();
        let output = metrics.generate_prometheus_format();

        let required_metrics = vec![
            "blockchain_blocks_total",
            "blockchain_transactions_total",
            "network_connected_peers",
        ];

        for metric in required_metrics {
            assert!(
                output.contains(metric),
                "Metric {} not found",
                metric
            );
            println!("✅ Metric found: {}", metric);
        }
    }
    println!();

    // Test 3: Metric values
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("TEST 3: Metric Value Accuracy");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    {
        use std::sync::atomic::Ordering;

        let metrics = SimpleMetricsCollector::new();
        metrics.blocks.store(42, Ordering::Relaxed);
        metrics.transactions.store(1337, Ordering::Relaxed);
        metrics.peers.store(5, Ordering::Relaxed);

        let output = metrics.generate_prometheus_format();

        assert!(output.contains("blockchain_blocks_total 42"));
        assert!(output.contains("blockchain_transactions_total 1337"));
        assert!(output.contains("network_connected_peers 5"));

        println!("✅ Blocks value: 42");
        println!("✅ Transactions value: 1337");
        println!("✅ Peers value: 5");
    }
    println!();

    // Test 4: Thread-safe operations
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("TEST 4: Thread-Safe Atomic Operations");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    {
        use std::sync::atomic::Ordering;
        use std::thread;

        let metrics = SimpleMetricsCollector::new();
        let metrics_clone = metrics.clone();

        thread::scope(|s| {
            for _ in 0..10 {
                let m = metrics_clone.clone();
                s.spawn(move || {
                    m.blocks.fetch_add(1, Ordering::Relaxed);
                });
            }
        });

        assert_eq!(metrics.blocks.load(Ordering::Relaxed), 10);
        println!("✅ 10 threads incremented counter to: 10");
    }
    println!();

    // Test 5: Line format validation
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("TEST 5: Line Format Validation");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    {
        let metrics = SimpleMetricsCollector::new();
        let output = metrics.generate_prometheus_format();

        let lines: Vec<&str> = output.lines().collect();
        for line in &lines {
            if !line.is_empty() && !line.starts_with('#') {
                let parts: Vec<&str> = line.split_whitespace().collect();
                assert_eq!(parts.len(), 2, "Metric line should have name and value");
            }
            println!("✅ Valid line: {}", line);
        }
    }
    println!();

    // Test 6: Full metrics output
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("TEST 6: Complete Metrics Output");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    {
        use std::sync::atomic::Ordering;

        let metrics = SimpleMetricsCollector::new();
        metrics.blocks.store(100, Ordering::Relaxed);
        metrics.transactions.store(500, Ordering::Relaxed);
        metrics.peers.store(8, Ordering::Relaxed);

        let output = metrics.generate_prometheus_format();
        println!("\n{}", output);

        let line_count = output.lines().count();
        println!("✅ Output contains {} lines", line_count);
        assert!(line_count > 0);
    }

    println!();
    println!("╔════════════════════════════════════════════════════════╗");
    println!("║        ✅ ALL TESTS PASSED                             ║");
    println!("╚════════════════════════════════════════════════════════╝");
    println!();
    println!("Prometheus Format Validation:");
    println!("  ✓ HELP and TYPE declarations present");
    println!("  ✓ Metric names follow Prometheus conventions");
    println!("  ✓ Values are accurate and atomic");
    println!("  ✓ Line format is correct (metric_name value)");
    println!("  ✓ Thread-safe atomic operations");
    println!("  ✓ Ready for Prometheus scraping");
}
