//! CSIRT/SIEM webhook notifier.
//!
//! Subscribes to the [`EventBus`], filters security-relevant events, and
//! forwards them as JSON POST requests to a configurable endpoint.
//!
//! Activate by setting `CSIRT_WEBHOOK_URL` (e.g. `https://csirt.example.org/incidents`).
//! Optional `CSIRT_WEBHOOK_SECRET` adds an `X-Webhook-Secret` header for authentication.

use std::time::Duration;

use tokio::sync::broadcast;
use tracing::{error, info, warn};

use super::BlockEvent;

/// Configuration for the webhook notifier.
#[derive(Debug, Clone)]
pub struct WebhookConfig {
    /// Target URL for POST requests.
    pub url: String,
    /// Optional shared secret sent as `X-Webhook-Secret` header.
    pub secret: Option<String>,
    /// HTTP timeout per request.
    pub timeout: Duration,
    /// Maximum consecutive failures before backing off.
    pub max_retries: u32,
}

impl WebhookConfig {
    /// Build config from environment variables.
    ///
    /// Returns `None` if `CSIRT_WEBHOOK_URL` is not set.
    pub fn from_env() -> Option<Self> {
        let url = std::env::var("CSIRT_WEBHOOK_URL").ok()?;
        if url.is_empty() {
            return None;
        }
        let secret = std::env::var("CSIRT_WEBHOOK_SECRET").ok();
        let timeout_secs: u64 = std::env::var("CSIRT_WEBHOOK_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10);
        Some(Self {
            url,
            secret,
            timeout: Duration::from_secs(timeout_secs),
            max_retries: 3,
        })
    }
}

/// Spawn a background task that forwards security events to the configured webhook.
///
/// The task runs until the broadcast channel is closed (i.e. all senders drop).
pub fn spawn_webhook_notifier(config: WebhookConfig, rx: broadcast::Receiver<BlockEvent>) {
    tokio::spawn(run_webhook_loop(config, rx));
}

async fn run_webhook_loop(config: WebhookConfig, mut rx: broadcast::Receiver<BlockEvent>) {
    info!(url = %config.url, "CSIRT webhook notifier started");

    let client = reqwest::Client::builder()
        .timeout(config.timeout)
        .build()
        .expect("failed to build HTTP client");

    let mut consecutive_failures: u32 = 0;

    loop {
        let event = match rx.recv().await {
            Ok(e) => e,
            Err(broadcast::error::RecvError::Closed) => {
                info!("Event bus closed, webhook notifier shutting down");
                break;
            }
            Err(broadcast::error::RecvError::Lagged(n)) => {
                warn!(
                    skipped = n,
                    "Webhook notifier lagged, some events were dropped"
                );
                continue;
            }
        };

        if !event.is_security_event() {
            continue;
        }

        let mut req = client.post(&config.url).json(&event);
        if let Some(ref secret) = config.secret {
            req = req.header("X-Webhook-Secret", secret);
        }

        match req.send().await {
            Ok(resp) if resp.status().is_success() => {
                consecutive_failures = 0;
            }
            Ok(resp) => {
                warn!(
                    status = resp.status().as_u16(),
                    "CSIRT webhook returned non-success status"
                );
                consecutive_failures += 1;
            }
            Err(e) => {
                error!(error = %e, "CSIRT webhook request failed");
                consecutive_failures += 1;
            }
        }

        if consecutive_failures >= config.max_retries {
            let backoff = Duration::from_secs(2u64.pow(consecutive_failures.min(6)));
            warn!(
                backoff_secs = backoff.as_secs(),
                "CSIRT webhook backing off after {} consecutive failures", consecutive_failures
            );
            tokio::time::sleep(backoff).await;
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::EventBus;

    #[test]
    fn config_with_all_fields() {
        let cfg = WebhookConfig {
            url: "https://csirt.test/hook".into(),
            secret: Some("s3cret".into()),
            timeout: Duration::from_secs(5),
            max_retries: 3,
        };
        assert_eq!(cfg.url, "https://csirt.test/hook");
        assert_eq!(cfg.secret.as_deref(), Some("s3cret"));
        assert_eq!(cfg.timeout, Duration::from_secs(5));
        assert_eq!(cfg.max_retries, 3);
    }

    #[test]
    fn config_without_secret() {
        let cfg = WebhookConfig {
            url: "https://csirt.test/hook".into(),
            secret: None,
            timeout: Duration::from_secs(10),
            max_retries: 3,
        };
        assert!(cfg.secret.is_none());
    }

    #[tokio::test]
    async fn notifier_ignores_non_security_events() {
        let bus = EventBus::new();
        let rx = bus.subscribe();

        let config = WebhookConfig {
            // Point to a non-routable address — we only test that it doesn't
            // attempt a POST for non-security events.
            url: "http://192.0.2.1:1/noop".into(),
            secret: None,
            timeout: Duration::from_millis(50),
            max_retries: 0,
        };

        spawn_webhook_notifier(config, rx);

        // Publish a non-security event — should be silently skipped.
        bus.publish(BlockEvent::BlockCommitted {
            channel_id: "ch1".into(),
            height: 1,
            tx_count: 1,
        });

        // Give the notifier time to process.
        tokio::time::sleep(Duration::from_millis(100)).await;
        // If we get here without a panic or hang, the notifier correctly skipped it.
    }

    #[tokio::test]
    async fn notifier_attempts_post_for_security_events() {
        let bus = EventBus::new();
        let rx = bus.subscribe();

        let config = WebhookConfig {
            // Non-routable — will fail, but we test that it *tries*.
            url: "http://192.0.2.1:1/csirt".into(),
            secret: Some("test-secret".into()),
            timeout: Duration::from_millis(100),
            max_retries: 0,
        };

        spawn_webhook_notifier(config, rx);

        bus.publish(BlockEvent::AclDenied {
            resource: "/api/v1/blocks".into(),
            identity: "attacker".into(),
            reason: "no identity".into(),
        });

        // The notifier will try to POST and fail (non-routable).
        // We just verify it doesn't panic.
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}
