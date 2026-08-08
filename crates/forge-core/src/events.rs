//! In-process event bus. Transports (SSE, WebSocket, Tauri IPC) fan out from
//! the same channel. Live-telemetry semantics: bounded buffer, slow consumers
//! lag.

use std::sync::Arc;

use serde::Serialize;
use tokio::sync::broadcast;

/// A published event: a free-form topic plus the JSON-encoded payload.
#[derive(Debug, Clone)]
pub struct Event {
    pub topic: String,
    /// Payload, already serialized to a JSON string.
    pub json: String,
}

/// Cloneable handle to the in-process event bus (capacity 256).
///
/// All subscribers fan out from the same [`tokio::sync::broadcast`] channel;
/// publishing when nobody listens is fine.
#[derive(Clone)]
pub struct EventBus {
    tx: broadcast::Sender<Arc<Event>>,
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// Events a subscriber may fall behind by before it is told it lagged.
pub const DEFAULT_CAPACITY: usize = 256;

impl EventBus {
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }

    /// A bus whose subscribers buffer `capacity` events. Smaller means a slow
    /// consumer is told sooner; the contract fixes only that the buffer is
    /// bounded.
    pub fn with_capacity(capacity: usize) -> Self {
        let (tx, _rx) = broadcast::channel(capacity.max(1));
        Self { tx }
    }

    /// Publish `data` (any `Serialize`) on `topic`. Serialization failures are
    /// logged and dropped — this is a fire-and-forget telemetry channel.
    pub fn publish<T: Serialize>(&self, topic: impl Into<String>, data: T) {
        let topic = topic.into();
        match serde_json::to_string(&data) {
            Ok(json) => {
                // Err just means no subscribers right now.
                let _ = self.tx.send(Arc::new(Event { topic, json }));
            }
            Err(e) => tracing::error!(topic, error = %e, "failed to serialize event payload"),
        }
    }

    /// Publish a payload that is already a JSON string.
    pub fn publish_json(&self, topic: impl Into<String>, json: impl Into<String>) {
        let _ = self.tx.send(Arc::new(Event {
            topic: topic.into(),
            json: json.into(),
        }));
    }

    /// Subscribe to the raw broadcast stream.
    pub fn subscribe(&self) -> broadcast::Receiver<Arc<Event>> {
        self.tx.subscribe()
    }

    /// Number of active subscribers.
    pub fn receiver_count(&self) -> usize {
        self.tx.receiver_count()
    }
}

#[cfg(test)]
mod tests {
    use super::EventBus;
    use serde_json::json;
    use tokio::sync::broadcast::error::RecvError;

    #[tokio::test]
    async fn publish_serializes_and_delivers() {
        let bus = EventBus::new();
        let mut rx = bus.subscribe();
        bus.publish("tick", json!({"n": 1}));
        let ev = rx.recv().await.unwrap();
        assert_eq!(ev.topic, "tick");
        assert_eq!(ev.json, r#"{"n":1}"#);
    }

    #[tokio::test]
    async fn publish_json_passes_the_payload_through_verbatim() {
        let bus = EventBus::new();
        let mut rx = bus.subscribe();
        bus.publish_json("raw", r#"{"pre": "encoded"}"#);
        let ev = rx.recv().await.unwrap();
        assert_eq!(ev.topic, "raw");
        assert_eq!(ev.json, r#"{"pre": "encoded"}"#);
    }

    #[test]
    fn publish_without_subscribers_is_fine() {
        let bus = EventBus::new();
        assert_eq!(bus.receiver_count(), 0);
        // Fire-and-forget: no subscriber, no panic, no error surfaced.
        bus.publish("nobody-listens", json!(1));
        bus.publish_json("still-nobody", "{}");
    }

    #[tokio::test]
    async fn every_subscriber_receives_every_event() {
        let bus = EventBus::new();
        let mut a = bus.subscribe();
        let mut b = bus.subscribe();
        assert_eq!(bus.receiver_count(), 2);
        bus.publish("t", json!("x"));
        assert_eq!(a.recv().await.unwrap().json, r#""x""#);
        assert_eq!(b.recv().await.unwrap().json, r#""x""#);
    }

    #[tokio::test]
    async fn slow_subscriber_lags_instead_of_blocking_publishers() {
        let bus = EventBus::new();
        let mut rx = bus.subscribe();
        // Overrun the capacity-256 buffer without draining the subscriber.
        for n in 0..300 {
            bus.publish("tick", json!(n));
        }
        let missed = match rx.recv().await {
            Err(RecvError::Lagged(missed)) => missed,
            other => panic!("expected a lag report, got {other:?}"),
        };
        assert!(missed > 0);
        // After the lag report the subscriber resumes at the oldest retained
        // event — the `missed` count says exactly which one that is.
        let ev = rx.recv().await.unwrap();
        assert_eq!(ev.json, missed.to_string());
    }

    #[tokio::test]
    async fn unserializable_payload_is_dropped_not_published() {
        struct Boom;
        impl serde::Serialize for Boom {
            fn serialize<S: serde::Serializer>(&self, _s: S) -> Result<S::Ok, S::Error> {
                Err(serde::ser::Error::custom("boom"))
            }
        }
        let bus = EventBus::new();
        let mut rx = bus.subscribe();
        bus.publish("bad", Boom);
        bus.publish("good", json!(1));
        // The bad payload never entered the channel; the next frame is the
        // good one.
        assert_eq!(rx.recv().await.unwrap().topic, "good");
    }

    /// The contract fixes only that the buffer is bounded, so how deep it is
    /// belongs to the caller. This is the seam under the corpus case
    /// `ws-lagged-tells-a-consumer-it-missed-events`, whose fixture asks for a
    /// one-deep buffer so that a small flood overruns it.
    #[tokio::test]
    async fn buffer_depth_is_the_callers_choice() {
        let bus = EventBus::with_capacity(1);
        let mut rx = bus.subscribe();

        bus.publish("t", 1);
        bus.publish("t", 2);

        assert!(matches!(rx.recv().await, Err(RecvError::Lagged(1))));
        assert_eq!(rx.recv().await.expect("the surviving event").json, "2");
    }

    /// A zero-deep buffer would panic the channel, so it is a one-deep one.
    #[tokio::test]
    async fn a_buffer_is_always_at_least_one_deep() {
        let bus = EventBus::with_capacity(0);
        let mut rx = bus.subscribe();
        bus.publish("t", 1);
        assert_eq!(rx.recv().await.expect("event").topic, "t");
    }
}
