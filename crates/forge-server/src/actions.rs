//! Action dispatch over HTTP: `POST /api/actions/{name}` — JSON payload in,
//! JSON out. Registry types live in [`forge_core::actions`].

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Response;
use serde_json::Value;

use crate::auth::extract::RequireClaims;
use crate::envelope::{err, ok};
use crate::error::error_response;
use crate::state::ForgeState;

pub use forge_core::actions::{
    box_action, unknown_action_error, ActionCtx, ActionFuture, BoxedAction,
};

pub(crate) async fn run_action(
    State(state): State<ForgeState>,
    Path(name): Path<String>,
    RequireClaims(claims): RequireClaims,
    body: Bytes,
) -> Response {
    let Some(action) = state.inner.actions.get(&name) else {
        return error_response(unknown_action_error(&name, &state.action_names()));
    };

    let payload: Value = if body.is_empty() {
        Value::Object(serde_json::Map::new())
    } else {
        match serde_json::from_slice(&body) {
            Ok(v) => v,
            Err(e) => {
                return err(
                    StatusCode::BAD_REQUEST,
                    format!("body is not valid JSON: {e}"),
                )
            }
        }
    };

    let ctx = ActionCtx {
        claims,
        events: state.events().clone(),
    };
    match action(payload, ctx).await {
        Ok(data) => ok(data),
        Err(e) => error_response(e),
    }
}
