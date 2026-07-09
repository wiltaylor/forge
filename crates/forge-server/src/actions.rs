//! Action registry: named async handlers dispatched via
//! `POST /api/actions/{name}` — JSON payload in, JSON out.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::Value;

use crate::auth::jwt::Claims;
use crate::envelope::{err, ok};
use crate::error::ForgeError;
use crate::events::EventBus;
use crate::state::ForgeState;

/// Context handed to every action: the caller's claims and the event bus.
#[derive(Clone)]
pub struct ActionCtx {
    pub claims: Claims,
    pub events: EventBus,
}

pub(crate) type ActionFuture = Pin<Box<dyn Future<Output = Result<Value, ForgeError>> + Send>>;
pub(crate) type BoxedAction = Arc<dyn Fn(Value, ActionCtx) -> ActionFuture + Send + Sync>;

/// Box a user handler into the registry shape.
pub(crate) fn box_action<F, Fut>(handler: F) -> BoxedAction
where
    F: Fn(Value, ActionCtx) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<Value, ForgeError>> + Send + 'static,
{
    Arc::new(move |payload, ctx| Box::pin(handler(payload, ctx)))
}

pub(crate) async fn run_action(
    State(state): State<ForgeState>,
    Path(name): Path<String>,
    claims: Claims,
    body: Bytes,
) -> Response {
    let Some(action) = state.inner.actions.get(&name) else {
        let names = state.action_names().join(", ");
        return err(
            StatusCode::NOT_FOUND,
            format!("unknown action {name:?} (have: [{names}])"),
        );
    };

    let payload: Value = if body.is_empty() {
        Value::Object(serde_json::Map::new())
    } else {
        match serde_json::from_slice(&body) {
            Ok(v) => v,
            Err(e) => {
                return err(StatusCode::BAD_REQUEST, format!("body is not valid JSON: {e}"))
            }
        }
    };

    let ctx = ActionCtx {
        claims,
        events: state.events().clone(),
    };
    match action(payload, ctx).await {
        Ok(data) => ok(data),
        Err(e) => e.into_response(),
    }
}
