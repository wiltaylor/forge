//! Event bus (re-exported from forge-core) fanned out over SSE
//! (`/api/events`) and WebSocket (`/api/ws`). Live-telemetry semantics:
//! bounded buffer, slow consumers lag.

pub mod sse;
pub mod ws;

pub use forge_core::events::{Event, EventBus};
