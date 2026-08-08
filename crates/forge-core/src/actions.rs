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

#[cfg(test)]
mod tests {
    use super::{box_action, unknown_action_error, ActionCtx};
    use crate::claims::Claims;
    use crate::error::ForgeError;
    use crate::events::EventBus;
    use serde_json::{json, Value};

    fn ctx() -> ActionCtx {
        ActionCtx {
            claims: Claims::anonymous(),
            events: EventBus::new(),
        }
    }

    #[tokio::test]
    async fn boxed_action_returns_the_handler_value() {
        let action = box_action(|payload, ctx: ActionCtx| async move {
            Ok(json!({"echo": payload, "caller": ctx.claims.sub}))
        });
        let out = action(json!({"n": 1}), ctx()).await.unwrap();
        assert_eq!(out, json!({"echo": {"n": 1}, "caller": "anonymous"}));
    }

    #[tokio::test]
    async fn boxed_action_propagates_handler_errors() {
        let action = box_action(|_payload, _ctx| async {
            Err::<Value, _>(ForgeError::BadRequest("nope".into()))
        });
        let err = action(json!(null), ctx()).await.unwrap_err();
        assert!(matches!(err, ForgeError::BadRequest(_)), "got {err:?}");
        assert_eq!(err.status(), 400);
        assert_eq!(err.to_string(), "nope");
    }

    #[tokio::test]
    async fn handlers_publish_on_the_ctx_event_bus() {
        let ctx = ctx();
        let mut rx = ctx.events.subscribe();
        let action = box_action(|_payload, ctx: ActionCtx| async move {
            ctx.events.publish("side-effect", json!({"done": true}));
            Ok(json!(null))
        });
        action(json!(null), ctx).await.unwrap();
        let ev = rx.recv().await.unwrap();
        assert_eq!(ev.topic, "side-effect");
        assert_eq!(ev.json, r#"{"done":true}"#);
    }

    #[test]
    fn unknown_action_is_a_404_naming_the_registry() {
        let err = unknown_action_error("missing", &["alpha", "beta"]);
        assert_eq!(err.status(), 404);
        assert_eq!(
            err.to_string(),
            r#"unknown action "missing" (have: [alpha, beta])"#
        );
    }

    #[test]
    fn unknown_action_with_an_empty_registry() {
        let err = unknown_action_error("missing", &[]);
        assert_eq!(err.status(), 404);
        assert_eq!(err.to_string(), r#"unknown action "missing" (have: [])"#);
    }
}
