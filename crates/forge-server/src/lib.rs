//! forge-server — the batteries-included backend layer for Forge apps.
//!
//! Implements the frozen Forge API contract (docs/api-contract.md):
//! envelope responses, HS256 JWT auth (with first-class auth-disabled mode),
//! a JSON document store, registered actions, an SSE/WebSocket event bus,
//! component federation and static frontend serving.
//!
//! ```no_run
//! use forge_server::ForgeApp;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), forge_server::ForgeError> {
//!     ForgeApp::new("my-app")
//!         .auth_from_env()
//!         .with_docstore_from_env()
//!         .with_events()
//!         .action("echo", |payload, _ctx| async move { Ok(payload) })
//!         .serve()
//!         .await
//! }
//! ```

pub mod actions;
pub mod app;
pub mod auth;
pub mod components;
pub mod docstore;
pub mod envelope;
pub mod error;
pub mod events;
pub mod frontend;
pub mod health;
pub mod state;

pub use actions::ActionCtx;
pub use app::ForgeApp;
pub use auth::extract::OptionalClaims;
pub use auth::jwt::Claims;
pub use auth::{AuthConfig, AuthUser, Hs256Validator, TokenValidator};
pub use docstore::DocStore;
pub use envelope::{err, ok, ok_empty};
pub use error::ForgeError;
pub use events::{Event, EventBus};
pub use state::ForgeState;
