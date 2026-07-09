//! Component federation: GET /api/components (manifest) and
//! GET /api/components/{file} (bundle files).

use axum::extract::{Path as UrlPath, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use serde_json::Value;

use crate::envelope::{err, ok};
use crate::state::ForgeState;

const FILE_PATTERN: &str = "^[a-zA-Z0-9][a-zA-Z0-9._-]{0,127}$";
const ALLOWED_EXTENSIONS: &[&str] = &[".js", ".mjs", ".css", ".map"];

/// Validate a bundle filename per the contract: `^[a-zA-Z0-9][a-zA-Z0-9._-]{0,127}$`,
/// no `..`, extension allowlist `.js .mjs .css .map`.
pub fn valid_component_file(name: &str) -> bool {
    let bytes = name.as_bytes();
    if bytes.is_empty() || bytes.len() > 128 {
        return false;
    }
    if !bytes[0].is_ascii_alphanumeric() {
        return false;
    }
    if !bytes[1..]
        .iter()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
    {
        return false;
    }
    if name.contains("..") {
        return false;
    }
    ALLOWED_EXTENSIONS.iter().any(|ext| name.ends_with(ext))
}

pub(crate) fn routes() -> Router<ForgeState> {
    Router::new()
        .route("/api/components", get(manifest))
        .route("/api/components/{file}", get(bundle))
}

fn components_dir(state: &ForgeState) -> &std::path::Path {
    state
        .inner
        .components_dir
        .as_deref()
        .expect("components routes mounted without a components dir")
}

async fn manifest(State(state): State<ForgeState>) -> Response {
    let path = components_dir(&state).join("manifest.json");
    let raw = match tokio::fs::read(&path).await {
        Ok(raw) => raw,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return err(StatusCode::NOT_FOUND, "no components manifest")
        }
        Err(e) => return err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };
    let manifest: Value = match serde_json::from_slice(&raw) {
        Ok(v) => v,
        Err(e) => {
            return err(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("manifest.json is not valid JSON: {e}"),
            )
        }
    };
    // Inject the app name.
    let manifest = match manifest {
        Value::Object(mut map) => {
            map.insert("app".into(), Value::String(state.app().to_string()));
            Value::Object(map)
        }
        // An array manifest is treated as the components list.
        Value::Array(components) => serde_json::json!({
            "app": state.app(),
            "components": components,
        }),
        other => other,
    };
    ok(manifest)
}

async fn bundle(State(state): State<ForgeState>, UrlPath(file): UrlPath<String>) -> Response {
    if !valid_component_file(&file) {
        return err(
            StatusCode::BAD_REQUEST,
            format!(
                "invalid component file name: {file:?} (must match {FILE_PATTERN}, extensions {})",
                ALLOWED_EXTENSIONS.join(" ")
            ),
        );
    }
    let path = components_dir(&state).join(&file);
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

#[cfg(test)]
mod tests {
    use super::valid_component_file;

    #[test]
    fn filename_validation() {
        assert!(valid_component_file("widget.js"));
        assert!(valid_component_file("Widget-1.2.3.mjs"));
        assert!(valid_component_file("styles.css"));
        assert!(valid_component_file("bundle.js.map"));
        assert!(!valid_component_file("evil.sh"));
        assert!(!valid_component_file(".hidden.js"));
        assert!(!valid_component_file("no-ext"));
        assert!(!valid_component_file("a/../b.js"));
        assert!(!valid_component_file("a..b.js"));
        assert!(!valid_component_file(""));
    }
}
