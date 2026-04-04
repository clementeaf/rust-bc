use prometheus::{Histogram, HistogramOpts, IntCounter, IntGauge, Opts, Registry, TextEncoder};
use std::sync::Arc;

/// Central Prometheus metrics collector.
///
/// Wraps a private [`Registry`] so each node gets isolated metric state.
/// All metric handles are `Clone` and internally `Arc`-backed, making the
/// collector itself cheaply cloneable.
#[derive(Clone)]
pub struct MetricsCollector {
    registry: Arc<Registry>,

    // ── Blockchain ────────────────────────────────────────────────────────────
    pub blocks_total: IntCounter,
    pub blockchain_transactions_total: IntCounter,
    pub chain_height: IntGauge,
    pub difficulty: IntGauge,

    // ── Transaction pipeline ──────────────────────────────────────────────────
    pub transactions_validated_total: IntCounter,
    pub transactions_rejected_total: IntCounter,
    pub transactions_total_fees: IntCounter,
    /// Histogram of per-transaction validation latency in milliseconds.
    pub transaction_validation_duration_ms: Histogram,

    // ── Mempool ───────────────────────────────────────────────────────────────
    pub mempool_pending: IntGauge,
    pub mempool_fees_pending: IntGauge,

    // ── Network / P2P ─────────────────────────────────────────────────────────
    pub network_peers: IntGauge,
    pub network_messages_received: IntCounter,
    pub network_messages_sent: IntCounter,

    // ── Gossip (Phase 12.1.1) ─────────────────────────────────────────────────
    /// Number of blocks sent to gossip fanout peers.
    pub gossip_blocks_gossiped: IntCounter,

    // ── Endorsement (Phase 12.2.1) ────────────────────────────────────────────
    /// Total endorsement policy validations performed.
    pub endorsement_validations_total: IntCounter,

    // ── Ordering (Phase 12.2.1) ───────────────────────────────────────────────
    /// Total ordered blocks cut by the ordering service.
    pub ordering_blocks_cut_total: IntCounter,

    // ── MVCC (Phase 12.2.1) ───────────────────────────────────────────────────
    /// Total MVCC read-set conflicts detected during block commit.
    pub mvcc_conflicts_total: IntCounter,

    // ── Events (Phase 12.2.1) ─────────────────────────────────────────────────
    /// Current number of active event bus subscriptions.
    pub event_subscriptions_active: IntGauge,

    // ── Discovery (Phase 12.2.1) ──────────────────────────────────────────────
    /// Current number of registered peers in the discovery service.
    pub discovery_peers_registered: IntGauge,
}

impl MetricsCollector {
    pub fn new() -> Self {
        let registry = Registry::new();

        // ── Blockchain ────────────────────────────────────────────────────────
        let blocks_total = IntCounter::with_opts(
            Opts::new("blockchain_blocks_total", "Total number of blocks in the chain"),
        )
        .expect("metric creation failed");
        registry.register(Box::new(blocks_total.clone())).expect("register failed");

        let blockchain_transactions_total = IntCounter::with_opts(
            Opts::new("blockchain_transactions_total", "Total transactions committed to blocks"),
        )
        .expect("metric creation failed");
        registry
            .register(Box::new(blockchain_transactions_total.clone()))
            .expect("register failed");

        let chain_height = IntGauge::with_opts(
            Opts::new("blockchain_height", "Current blockchain height"),
        )
        .expect("metric creation failed");
        registry.register(Box::new(chain_height.clone())).expect("register failed");

        let difficulty = IntGauge::with_opts(
            Opts::new("blockchain_difficulty", "Current mining difficulty"),
        )
        .expect("metric creation failed");
        registry.register(Box::new(difficulty.clone())).expect("register failed");

        // ── Transaction pipeline ──────────────────────────────────────────────
        let transactions_validated_total = IntCounter::with_opts(
            Opts::new("transactions_validated_total", "Total validated transactions"),
        )
        .expect("metric creation failed");
        registry
            .register(Box::new(transactions_validated_total.clone()))
            .expect("register failed");

        let transactions_rejected_total = IntCounter::with_opts(
            Opts::new("transactions_rejected_total", "Total rejected transactions"),
        )
        .expect("metric creation failed");
        registry
            .register(Box::new(transactions_rejected_total.clone()))
            .expect("register failed");

        let transactions_total_fees = IntCounter::with_opts(
            Opts::new("transactions_total_fees_collected", "Total fees collected (in base units)"),
        )
        .expect("metric creation failed");
        registry
            .register(Box::new(transactions_total_fees.clone()))
            .expect("register failed");

        let transaction_validation_duration_ms = Histogram::with_opts(
            HistogramOpts::new(
                "transaction_validation_duration_ms",
                "Transaction validation latency in milliseconds",
            )
            .buckets(vec![0.5, 1.0, 2.5, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0]),
        )
        .expect("metric creation failed");
        registry
            .register(Box::new(transaction_validation_duration_ms.clone()))
            .expect("register failed");

        // ── Mempool ───────────────────────────────────────────────────────────
        let mempool_pending = IntGauge::with_opts(
            Opts::new("mempool_pending_transactions", "Pending transactions in mempool"),
        )
        .expect("metric creation failed");
        registry.register(Box::new(mempool_pending.clone())).expect("register failed");

        let mempool_fees_pending = IntGauge::with_opts(
            Opts::new("mempool_total_fees_pending", "Total fees of pending transactions"),
        )
        .expect("metric creation failed");
        registry
            .register(Box::new(mempool_fees_pending.clone()))
            .expect("register failed");

        // ── Network ───────────────────────────────────────────────────────────
        let network_peers = IntGauge::with_opts(
            Opts::new("network_connected_peers", "Number of connected P2P peers"),
        )
        .expect("metric creation failed");
        registry.register(Box::new(network_peers.clone())).expect("register failed");

        let network_messages_received = IntCounter::with_opts(
            Opts::new("network_messages_received_total", "Total P2P messages received"),
        )
        .expect("metric creation failed");
        registry
            .register(Box::new(network_messages_received.clone()))
            .expect("register failed");

        let network_messages_sent = IntCounter::with_opts(
            Opts::new("network_messages_sent_total", "Total P2P messages sent"),
        )
        .expect("metric creation failed");
        registry
            .register(Box::new(network_messages_sent.clone()))
            .expect("register failed");

        // ── Gossip ────────────────────────────────────────────────────────────
        let gossip_blocks_gossiped = IntCounter::with_opts(
            Opts::new("gossip_blocks_gossiped_total", "Blocks sent to gossip fanout peers"),
        )
        .expect("metric creation failed");
        registry
            .register(Box::new(gossip_blocks_gossiped.clone()))
            .expect("register failed");

        // ── Endorsement ───────────────────────────────────────────────────────
        let endorsement_validations_total = IntCounter::with_opts(
            Opts::new("endorsement_validations_total", "Total endorsement policy validations"),
        )
        .expect("metric creation failed");
        registry
            .register(Box::new(endorsement_validations_total.clone()))
            .expect("register failed");

        // ── Ordering ─────────────────────────────────────────────────────────
        let ordering_blocks_cut_total = IntCounter::with_opts(
            Opts::new("ordering_blocks_cut_total", "Total ordered blocks cut"),
        )
        .expect("metric creation failed");
        registry
            .register(Box::new(ordering_blocks_cut_total.clone()))
            .expect("register failed");

        // ── MVCC ──────────────────────────────────────────────────────────────
        let mvcc_conflicts_total = IntCounter::with_opts(
            Opts::new("mvcc_conflicts_total", "Total MVCC read-set conflicts"),
        )
        .expect("metric creation failed");
        registry
            .register(Box::new(mvcc_conflicts_total.clone()))
            .expect("register failed");

        // ── Events ────────────────────────────────────────────────────────────
        let event_subscriptions_active = IntGauge::with_opts(
            Opts::new("event_subscriptions_active", "Active event bus subscriptions"),
        )
        .expect("metric creation failed");
        registry
            .register(Box::new(event_subscriptions_active.clone()))
            .expect("register failed");

        // ── Discovery ─────────────────────────────────────────────────────────
        let discovery_peers_registered = IntGauge::with_opts(
            Opts::new("discovery_peers_registered", "Registered peers in discovery service"),
        )
        .expect("metric creation failed");
        registry
            .register(Box::new(discovery_peers_registered.clone()))
            .expect("register failed");

        MetricsCollector {
            registry: Arc::new(registry),
            blocks_total,
            blockchain_transactions_total,
            chain_height,
            difficulty,
            transactions_validated_total,
            transactions_rejected_total,
            transactions_total_fees,
            transaction_validation_duration_ms,
            mempool_pending,
            mempool_fees_pending,
            network_peers,
            network_messages_received,
            network_messages_sent,
            gossip_blocks_gossiped,
            endorsement_validations_total,
            ordering_blocks_cut_total,
            mvcc_conflicts_total,
            event_subscriptions_active,
            discovery_peers_registered,
        }
    }

    // ── Blockchain helpers ────────────────────────────────────────────────────

    pub fn record_block(&self, tx_count: u64, difficulty: u8, chain_height: u64) {
        self.blocks_total.inc();
        self.blockchain_transactions_total.inc_by(tx_count);
        self.chain_height.set(chain_height as i64);
        self.difficulty.set(difficulty as i64);
    }

    pub fn update_height(&self, height: u64) {
        self.chain_height.set(height as i64);
    }

    pub fn update_difficulty(&self, difficulty: u8) {
        self.difficulty.set(difficulty as i64);
    }

    // ── Transaction helpers ───────────────────────────────────────────────────

    pub fn record_validated(&self, fee: u64, validation_time_ms: f64) {
        self.transactions_validated_total.inc();
        self.transactions_total_fees.inc_by(fee);
        self.transaction_validation_duration_ms.observe(validation_time_ms);
    }

    pub fn record_rejected(&self, _reason: &str) {
        self.transactions_rejected_total.inc();
    }

    // ── Mempool helpers ───────────────────────────────────────────────────────

    pub fn update_mempool(&self, pending_count: u64, total_fees: u64) {
        self.mempool_pending.set(pending_count as i64);
        self.mempool_fees_pending.set(total_fees as i64);
    }

    // ── Network helpers ───────────────────────────────────────────────────────

    pub fn update_peers(&self, count: u64) {
        self.network_peers.set(count as i64);
    }

    pub fn record_message_received(&self) {
        self.network_messages_received.inc();
    }

    pub fn record_message_sent(&self) {
        self.network_messages_sent.inc();
    }

    // ── Gossip helpers ────────────────────────────────────────────────────────

    pub fn record_gossip_block(&self) {
        self.gossip_blocks_gossiped.inc();
    }

    // ── Endorsement helpers ───────────────────────────────────────────────────

    /// Increment after each call to `validate_endorsements`.
    pub fn record_endorsement_validation(&self) {
        self.endorsement_validations_total.inc();
    }

    // ── Ordering helpers ──────────────────────────────────────────────────────

    /// Increment each time the ordering service cuts a new block.
    pub fn record_ordering_block_cut(&self) {
        self.ordering_blocks_cut_total.inc();
    }

    // ── MVCC helpers ──────────────────────────────────────────────────────────

    /// Increment each time a transaction is rejected due to an MVCC conflict.
    pub fn record_mvcc_conflict(&self) {
        self.mvcc_conflicts_total.inc();
    }

    // ── Event bus helpers ─────────────────────────────────────────────────────

    /// Set the current active subscription count (call after subscribe/unsubscribe).
    pub fn set_event_subscriptions(&self, count: usize) {
        self.event_subscriptions_active.set(count as i64);
    }

    // ── Discovery helpers ─────────────────────────────────────────────────────

    /// Set the current registered peer count (call after register/unregister).
    pub fn set_discovery_peers(&self, count: usize) {
        self.discovery_peers_registered.set(count as i64);
    }

    // ── Rendering ─────────────────────────────────────────────────────────────

    /// Render all metrics in Prometheus text exposition format (0.0.4).
    pub fn collect_metrics(&self) -> String {
        let mf = self.registry.gather();
        let encoder = TextEncoder::new();
        encoder.encode_to_string(&mf).unwrap_or_default()
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_metrics_returns_prometheus_text() {
        let m = MetricsCollector::new();
        m.record_block(5, 2, 10);
        let output = m.collect_metrics();
        assert!(output.contains("blockchain_blocks_total 1"));
        assert!(output.contains("blockchain_height 10"));
    }

    #[test]
    fn transaction_metrics_increment() {
        let m = MetricsCollector::new();
        m.record_validated(100, 3.5);
        m.record_rejected("bad sig");
        let output = m.collect_metrics();
        assert!(output.contains("transactions_validated_total 1"));
        assert!(output.contains("transactions_rejected_total 1"));
    }

    #[test]
    fn gossip_counter_increments() {
        let m = MetricsCollector::new();
        m.record_gossip_block();
        m.record_gossip_block();
        let output = m.collect_metrics();
        assert!(output.contains("gossip_blocks_gossiped_total 2"));
    }

    #[test]
    fn network_peer_gauge() {
        let m = MetricsCollector::new();
        m.update_peers(7);
        let output = m.collect_metrics();
        assert!(output.contains("network_connected_peers 7"));
    }

    #[test]
    fn endorsement_validation_counter() {
        let m = MetricsCollector::new();
        m.record_endorsement_validation();
        m.record_endorsement_validation();
        m.record_endorsement_validation();
        let output = m.collect_metrics();
        assert!(output.contains("endorsement_validations_total 3"));
    }

    #[test]
    fn ordering_blocks_cut_counter() {
        let m = MetricsCollector::new();
        m.record_ordering_block_cut();
        m.record_ordering_block_cut();
        let output = m.collect_metrics();
        assert!(output.contains("ordering_blocks_cut_total 2"));
    }

    #[test]
    fn mvcc_conflicts_counter() {
        let m = MetricsCollector::new();
        m.record_mvcc_conflict();
        let output = m.collect_metrics();
        assert!(output.contains("mvcc_conflicts_total 1"));
    }

    #[test]
    fn event_subscriptions_gauge() {
        let m = MetricsCollector::new();
        m.set_event_subscriptions(5);
        let out = m.collect_metrics();
        assert!(out.contains("event_subscriptions_active 5"));
        m.set_event_subscriptions(3);
        let out2 = m.collect_metrics();
        assert!(out2.contains("event_subscriptions_active 3"));
    }

    #[test]
    fn discovery_peers_gauge() {
        let m = MetricsCollector::new();
        m.set_discovery_peers(12);
        let out = m.collect_metrics();
        assert!(out.contains("discovery_peers_registered 12"));
    }
}
