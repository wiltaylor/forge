//! Event bus (re-exported from forge-core) fanned out over SSE
//! (`/api/events`) and WebSocket (`/api/ws`). Live-telemetry semantics:
//! bounded buffer, slow consumers lag.

pub mod sse;
pub mod ws;

use std::time::Duration;

pub use forge_core::events::{Event, EventBus};

/// Contract: a comment heartbeat every 15 s on `/api/events`.
pub const DEFAULT_HEARTBEAT: Duration = Duration::from_secs(15);

/// How the two event endpoints behave.
///
/// Both members default to the values the contract states. They are knobs
/// because the contract fixes the behaviour, not the numbers: a bounded
/// buffer and a periodic heartbeat. A deployment that wants a slow consumer
/// told sooner, or a caller that will not wait a quarter of a minute to see
/// the heartbeat arrive, sets its own.
#[derive(Debug, Clone)]
pub struct EventsConfig {
    /// Events a subscriber may fall behind by before it is told it lagged.
    pub buffer: usize,
    /// Time between heartbeat comments on the event stream.
    pub heartbeat: Duration,
}

impl Default for EventsConfig {
    fn default() -> Self {
        Self {
            buffer: forge_core::events::DEFAULT_CAPACITY,
            heartbeat: DEFAULT_HEARTBEAT,
        }
    }
}

impl EventsConfig {
    /// Events a subscriber may fall behind by before it lags.
    pub fn buffer(mut self, events: usize) -> Self {
        self.buffer = events;
        self
    }

    /// Time between heartbeat comments on the event stream.
    pub fn heartbeat(mut self, every: Duration) -> Self {
        self.heartbeat = every;
        self
    }
}
