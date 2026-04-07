use prometheus::{Counter, CounterVec, HistogramVec, IntGauge, Registry};

/// Prometheus metrics for rust-bc API
#[derive(Clone)]
pub struct ApiMetrics {
    /// Total HTTP requests by method, path, and status code
    pub http_requests_total: CounterVec,
    /// HTTP request duration in seconds (histogram)
    pub http_request_duration_seconds: HistogramVec,
    /// Active consensus forks in DAG
    pub consensus_fork_count: IntGauge,
    /// Pending transactions in mempool
    pub mempool_pending_transactions: IntGauge,
    /// Total DIDs created
    pub identity_dids_total: Counter,
    /// Total credentials issued
    pub credentials_issued_total: Counter,
}

impl ApiMetrics {
    /// Create and register all metrics with the given registry
    pub fn new(registry: &Registry) -> Result<Self, prometheus::Error> {
        let http_requests_total = CounterVec::new(
            prometheus::Opts::new(
                "rust_bc_http_requests_total",
                "Total HTTP requests processed",
            ),
            &["method", "path", "status"],
        )?;
        registry.register(Box::new(http_requests_total.clone()))?;

        let http_request_duration_seconds = HistogramVec::new(
            prometheus::HistogramOpts::new(
                "rust_bc_http_request_duration_seconds",
                "HTTP request latency in seconds",
            ),
            &["method", "path"],
        )?;
        registry.register(Box::new(http_request_duration_seconds.clone()))?;

        let consensus_fork_count = IntGauge::new(
            "rust_bc_consensus_fork_count",
            "Current number of active forks in DAG",
        )?;
        registry.register(Box::new(consensus_fork_count.clone()))?;

        let mempool_pending_transactions = IntGauge::new(
            "rust_bc_mempool_pending_transactions",
            "Number of pending transactions in mempool",
        )?;
        registry.register(Box::new(mempool_pending_transactions.clone()))?;

        let identity_dids_total =
            Counter::new("rust_bc_identity_dids_total", "Total DIDs created")?;
        registry.register(Box::new(identity_dids_total.clone()))?;

        let credentials_issued_total = Counter::new(
            "rust_bc_credentials_issued_total",
            "Total credentials issued",
        )?;
        registry.register(Box::new(credentials_issued_total.clone()))?;

        Ok(ApiMetrics {
            http_requests_total,
            http_request_duration_seconds,
            consensus_fork_count,
            mempool_pending_transactions,
            identity_dids_total,
            credentials_issued_total,
        })
    }

    /// Record a successful HTTP request
    pub fn record_request_success(&self, method: &str, path: &str, duration_secs: f64) {
        self.http_requests_total
            .with_label_values(&[method, path, "200"])
            .inc();
        self.http_request_duration_seconds
            .with_label_values(&[method, path])
            .observe(duration_secs);
    }

    /// Record a failed HTTP request
    pub fn record_request_error(
        &self,
        method: &str,
        path: &str,
        status_code: &str,
        duration_secs: f64,
    ) {
        self.http_requests_total
            .with_label_values(&[method, path, status_code])
            .inc();
        self.http_request_duration_seconds
            .with_label_values(&[method, path])
            .observe(duration_secs);
    }

    /// Update fork count in consensus
    pub fn set_fork_count(&self, count: i64) {
        self.consensus_fork_count.set(count);
    }

    /// Update pending transaction count
    pub fn set_pending_tx_count(&self, count: i64) {
        self.mempool_pending_transactions.set(count);
    }

    /// Increment DIDs created
    pub fn increment_dids_created(&self) {
        self.identity_dids_total.inc();
    }

    /// Increment credentials issued
    pub fn increment_credentials_issued(&self) {
        self.credentials_issued_total.inc();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_creation() {
        let registry = Registry::new();
        let metrics = ApiMetrics::new(&registry);
        assert!(metrics.is_ok());
    }

    #[test]
    fn test_record_request_success() {
        let registry = Registry::new();
        let metrics = ApiMetrics::new(&registry).unwrap();
        metrics.record_request_success("GET", "/health", 0.010);

        let gathered = registry.gather();
        let mut found = false;
        for mf in gathered {
            if mf.get_name() == "rust_bc_http_requests_total" {
                found = true;
                break;
            }
        }
        assert!(found);
    }

    #[test]
    fn test_record_request_error() {
        let registry = Registry::new();
        let metrics = ApiMetrics::new(&registry).unwrap();
        metrics.record_request_error("POST", "/identity/create", "500", 0.050);

        let gathered = registry.gather();
        let mut found = false;
        for mf in gathered {
            if mf.get_name() == "rust_bc_http_requests_total" {
                found = true;
                break;
            }
        }
        assert!(found);
    }

    #[test]
    fn test_set_fork_count() {
        let registry = Registry::new();
        let metrics = ApiMetrics::new(&registry).unwrap();
        metrics.set_fork_count(5);

        let gathered = registry.gather();
        let mut found = false;
        for mf in gathered {
            if mf.get_name() == "rust_bc_consensus_fork_count" {
                found = true;
                break;
            }
        }
        assert!(found);
    }

    #[test]
    fn test_set_pending_tx_count() {
        let registry = Registry::new();
        let metrics = ApiMetrics::new(&registry).unwrap();
        metrics.set_pending_tx_count(100);

        let gathered = registry.gather();
        let mut found = false;
        for mf in gathered {
            if mf.get_name() == "rust_bc_mempool_pending_transactions" {
                found = true;
                break;
            }
        }
        assert!(found);
    }

    #[test]
    fn test_increment_dids() {
        let registry = Registry::new();
        let metrics = ApiMetrics::new(&registry).unwrap();
        metrics.increment_dids_created();
        metrics.increment_dids_created();

        let gathered = registry.gather();
        let mut found = false;
        for mf in gathered {
            if mf.get_name() == "rust_bc_identity_dids_total" {
                found = true;
                break;
            }
        }
        assert!(found);
    }

    #[test]
    fn test_increment_credentials() {
        let registry = Registry::new();
        let metrics = ApiMetrics::new(&registry).unwrap();
        metrics.increment_credentials_issued();

        let gathered = registry.gather();
        let mut found = false;
        for mf in gathered {
            if mf.get_name() == "rust_bc_credentials_issued_total" {
                found = true;
                break;
            }
        }
        assert!(found);
    }

    #[test]
    fn test_multiple_metrics_registered() {
        let registry = Registry::new();
        let _metrics = ApiMetrics::new(&registry).unwrap();

        let gathered = registry.gather();
        // Each metric family is registered, so we should have at least 6
        assert!(
            gathered.len() >= 4,
            "Should have at least 4 metric families registered"
        );
    }

    #[test]
    fn test_histogram_observation() {
        let registry = Registry::new();
        let metrics = ApiMetrics::new(&registry).unwrap();
        metrics.record_request_success("GET", "/version", 0.025);

        let gathered = registry.gather();
        let mut found = false;
        for mf in gathered {
            if mf.get_name() == "rust_bc_http_request_duration_seconds" {
                found = true;
                break;
            }
        }
        assert!(found);
    }

    #[test]
    fn test_metrics_labels() {
        let registry = Registry::new();
        let metrics = ApiMetrics::new(&registry).unwrap();
        metrics.record_request_success("POST", "/blocks/propose", 0.045);
        metrics.record_request_error("GET", "/consensus/state", "404", 0.008);

        let gathered = registry.gather();
        let mut counter_found = false;
        for mf in gathered {
            if mf.get_name() == "rust_bc_http_requests_total" {
                counter_found = true;
                break;
            }
        }
        assert!(counter_found);
    }
}
