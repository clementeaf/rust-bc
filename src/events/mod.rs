//! Event bus for block and transaction notifications.
//!
//! Uses `tokio::sync::broadcast` so multiple subscribers each receive every
//! event independently (fan-out semantics).

use tokio::sync::broadcast;

pub mod filtered;
pub mod private_delivery;
pub mod types;
pub use types::BlockEvent;

/// Default channel capacity: how many events can be buffered before the
/// slowest receiver starts lagging.
const DEFAULT_CAPACITY: usize = 128;

/// Fan-out event bus backed by a `tokio::sync::broadcast` channel.
///
/// Clone the bus to share it across threads; all clones share the same sender.
#[derive(Clone)]
pub struct EventBus {
    tx: broadcast::Sender<BlockEvent>,
}

impl EventBus {
    /// Create a new `EventBus` with the default buffer capacity.
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(DEFAULT_CAPACITY);
        Self { tx }
    }

    /// Create a new `EventBus` with a custom buffer capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }

    /// Publish an event to all active subscribers.
    ///
    /// Returns the number of receivers that received the event.
    /// Returns `0` (not an error) when there are no active subscribers.
    pub fn publish(&self, event: BlockEvent) -> usize {
        self.tx.send(event).unwrap_or(0)
    }

    /// Subscribe to the event stream.
    ///
    /// Each call returns an independent receiver — every subscriber sees every
    /// event published after the subscription was created.
    pub fn subscribe(&self) -> broadcast::Receiver<BlockEvent> {
        self.tx.subscribe()
    }

    /// Number of active receivers.
    pub fn receiver_count(&self) -> usize {
        self.tx.receiver_count()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::types::BlockEvent;

    #[tokio::test]
    async fn three_receivers_all_get_the_event() {
        let bus = EventBus::new();

        let mut rx1 = bus.subscribe();
        let mut rx2 = bus.subscribe();
        let mut rx3 = bus.subscribe();

        let event = BlockEvent::BlockCommitted { channel_id: "ch1".to_string(), height: 42, tx_count: 3 };
        let sent = bus.publish(event.clone());
        assert_eq!(sent, 3, "should report 3 receivers");

        let e1 = rx1.recv().await.unwrap();
        let e2 = rx2.recv().await.unwrap();
        let e3 = rx3.recv().await.unwrap();

        assert_eq!(e1, event);
        assert_eq!(e2, event);
        assert_eq!(e3, event);
    }

    #[tokio::test]
    async fn publish_with_no_subscribers_returns_zero() {
        let bus = EventBus::new();
        let sent = bus.publish(BlockEvent::BlockCommitted { channel_id: "".to_string(), height: 1, tx_count: 0 });
        assert_eq!(sent, 0);
    }

    #[tokio::test]
    async fn late_subscriber_misses_earlier_events() {
        let bus = EventBus::new();

        let mut rx_early = bus.subscribe();
        bus.publish(BlockEvent::BlockCommitted { channel_id: "ch".to_string(), height: 1, tx_count: 1 });

        // Subscribe after the first event was published
        let mut rx_late = bus.subscribe();
        bus.publish(BlockEvent::BlockCommitted { channel_id: "ch".to_string(), height: 2, tx_count: 2 });

        // Early subscriber sees both events
        let e1 = rx_early.recv().await.unwrap();
        let e2 = rx_early.recv().await.unwrap();
        assert_eq!(e1, BlockEvent::BlockCommitted { channel_id: "ch".to_string(), height: 1, tx_count: 1 });
        assert_eq!(e2, BlockEvent::BlockCommitted { channel_id: "ch".to_string(), height: 2, tx_count: 2 });

        // Late subscriber sees only the second event
        let e3 = rx_late.recv().await.unwrap();
        assert_eq!(e3, BlockEvent::BlockCommitted { channel_id: "ch".to_string(), height: 2, tx_count: 2 });
    }

    #[test]
    fn receiver_count_tracks_live_subscriptions() {
        let bus = EventBus::new();
        assert_eq!(bus.receiver_count(), 0);

        let _r1 = bus.subscribe();
        assert_eq!(bus.receiver_count(), 1);

        let _r2 = bus.subscribe();
        assert_eq!(bus.receiver_count(), 2);

        drop(_r1);
        assert_eq!(bus.receiver_count(), 1);
    }
}
