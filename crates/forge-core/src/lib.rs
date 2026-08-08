//! forge-core — the transport-agnostic core of the Forge backend.
//!
//! Holds everything the frozen Forge API contract (docs/api-contract.md)
//! needs that is independent of how requests arrive: the JSON document
//! store, the action registry types, the event bus, authentication (claims,
//! token validation and the login user store), component federation, the
//! response-envelope shapes and the health payload. `forge-server` serves
//! these over HTTP/SSE/WebSocket; `forge-tauri` serves the same semantics
//! over Tauri IPC.

pub mod actions;
pub mod auth;
pub mod claims;
pub mod components;
pub mod docstore;
pub mod envelope;
pub mod error;
pub mod events;
pub mod health;
#[cfg(any(feature = "term", feature = "vnc", feature = "rdp"))]
pub mod widgets;

pub use actions::{box_action, unknown_action_error, ActionCtx, ActionFuture, BoxedAction};
pub use auth::{
    Auth, AuthConfig, AuthUser, Hs256Validator, LoginResponse, MeResponse, TokenValidator,
};
pub use claims::{unix_now, Claims};
pub use components::{valid_component_file, Components, ALLOWED_EXTENSIONS, FILE_PATTERN};
pub use docstore::{valid_doc_name, DocStore, NAME_PATTERN};
pub use envelope::{err_value, ok_empty_value, ok_value};
pub use error::ForgeError;
pub use events::{Event, EventBus};
pub use health::health_payload;
