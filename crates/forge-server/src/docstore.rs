//! HTTP routes for the JSON document store. The store itself lives in
//! [`forge_core::docstore`]; this module mounts it at `/api/data`.

use axum::body::Bytes;
use axum::extract::{Path as UrlPath, State};
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use serde_json::Value;

use crate::envelope::{err, ok, ok_empty};
use crate::error::error_response;
use crate::state::ForgeState;

pub use forge_core::docstore::{valid_doc_name, DocStore, NAME_PATTERN};

pub(crate) fn routes() -> Router<ForgeState> {
    Router::new().route("/api/data", get(list_docs)).route(
        "/api/data/{name}",
        get(get_doc).put(put_doc).delete(delete_doc),
    )
}

fn store(state: &ForgeState) -> &DocStore {
    // Routes are only mounted when the doc store is configured.
    state
        .docstore()
        .expect("docstore routes mounted without a store")
}

async fn list_docs(State(state): State<ForgeState>) -> Response {
    match store(&state).list().await {
        Ok(docs) => ok(docs),
        Err(e) => error_response(e),
    }
}

async fn get_doc(State(state): State<ForgeState>, UrlPath(name): UrlPath<String>) -> Response {
    match store(&state).get(&name).await {
        Ok(doc) => ok(doc),
        Err(e) => error_response(e),
    }
}

async fn put_doc(
    State(state): State<ForgeState>,
    UrlPath(name): UrlPath<String>,
    body: Bytes,
) -> Response {
    let value: Value = if body.is_empty() {
        Value::Null
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
    match store(&state).put(&name, &value).await {
        Ok(()) => ok_empty(),
        Err(e) => error_response(e),
    }
}

async fn delete_doc(State(state): State<ForgeState>, UrlPath(name): UrlPath<String>) -> Response {
    match store(&state).delete(&name).await {
        Ok(()) => ok_empty(),
        Err(e) => error_response(e),
    }
}
