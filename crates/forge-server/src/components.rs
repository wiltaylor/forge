//! HTTP routes for component federation. The filename rule and the manifest
//! live in [`forge_core::components`]; this module mounts them at
//! `/api/components` and `/api/components/{file}`.

use axum::extract::{Path as UrlPath, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;

use crate::envelope::{err, ok};
use crate::error::error_response;
use crate::state::ForgeState;

pub use forge_core::components::{
    valid_component_file, Components, ALLOWED_EXTENSIONS, FILE_PATTERN,
};

pub(crate) fn routes() -> Router<ForgeState> {
    Router::new()
        .route("/api/components", get(manifest))
        .route("/api/components/{file}", get(bundle))
}

fn components(state: &ForgeState) -> &Components {
    // Routes are only mounted when the components directory is configured.
    state
        .components()
        .expect("components routes mounted without a components dir")
}

async fn manifest(State(state): State<ForgeState>) -> Response {
    match components(&state).manifest(state.app()).await {
        Ok(manifest) => ok(manifest),
        Err(e) => error_response(e),
    }
}

async fn bundle(State(state): State<ForgeState>, UrlPath(file): UrlPath<String>) -> Response {
    let path = match components(&state).file_path(&file) {
        Ok(path) => path,
        Err(e) => return error_response(e),
    };
    match tokio::fs::read(&path).await {
        Ok(bytes) => {
            let mime = mime_guess::from_path(&file).first_or_octet_stream();
            (
                StatusCode::OK,
                [
                    (header::CONTENT_TYPE, mime.as_ref().to_string()),
                    (header::CACHE_CONTROL, "no-cache".to_string()),
                ],
                bytes,
            )
                .into_response()
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            err(StatusCode::NOT_FOUND, format!("no component file {file:?}"))
        }
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}
