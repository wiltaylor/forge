//! Action registry types: named async handlers — JSON payload in, JSON out.
//! Transports dispatch by name (`POST /api/actions/{name}` over HTTP, the
//! `request` command over Tauri IPC).

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use serde_json::Value;

use crate::claims::Claims;
use crate::error::ForgeError;
use crate::events::EventBus;

/// Context handed to every action: the caller's claims and the event bus.
#[derive(Clone)]
pub struct ActionCtx {
    pub claims: Claims,
    pub events: EventBus,
}

pub type ActionFuture = Pin<Box<dyn Future<Output = Result<Value, ForgeError>> + Send>>;
pub type BoxedAction = Arc<dyn Fn(Value, ActionCtx) -> ActionFuture + Send + Sync>;

/// Box a user handler into the registry shape.
pub fn box_action<F, Fut>(handler: F) -> BoxedAction
where
    F: Fn(Value, ActionCtx) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<Value, ForgeError>> + Send + 'static,
{
    Arc::new(move |payload, ctx| Box::pin(handler(payload, ctx)))
}

/// The contract-mandated 404 for an unknown action: names the miss and lists
/// what is registered. Shared by every transport so the message shape never
/// drifts.
pub fn unknown_action_error(name: &str, known: &[&str]) -> ForgeError {
    let names = known.join(", ");
    ForgeError::NotFound(format!("unknown action {name:?} (have: [{names}])"))
}
