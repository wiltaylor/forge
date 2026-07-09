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

impl EventBus {
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(256);
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
