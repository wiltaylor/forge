//! Static frontend serving with SPA fallback.
//!
//! Two modes: a directory on disk (tower-http `ServeDir` with an
//! `index.html` fallback) or assets embedded at compile time via
//! `rust-embed` (feature `embed`). Unknown non-`/api` GET paths fall back to
//! `index.html`; `/api/*` misses stay JSON 404 envelopes.

use std::path::PathBuf;

use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{header, HeaderValue, Method, StatusCode};
use axum::response::Response;
use tower::ServiceExt;
use tower_http::services::{ServeDir, ServeFile};

use crate::envelope::err;
use crate::state::ForgeState;

#[cfg(feature = "embed")]
pub(crate) type EmbeddedLookup =
    std::sync::Arc<dyn Fn(&str) -> Option<std::borrow::Cow<'static, [u8]>> + Send + Sync>;

/// Frontend serving mode.
#[derive(Clone, Default)]
pub(crate) enum Frontend {
    #[default]
    None,
    Dir(PathBuf),
    #[cfg(feature = "embed")]
    Embedded(EmbeddedLookup),
}

const IMMUTABLE: &str = "public, max-age=31536000, immutable";
const NO_CACHE: &str = "no-cache";

/// Hashed build assets (Vite puts them under /assets/) get immutable caching;
/// everything else is revalidated.
fn cache_control(path: &str) -> &'static str {
    if path.starts_with("/assets/") || path.starts_with("assets/") {
        IMMUTABLE
    } else {
        NO_CACHE
    }
}

/// Router fallback: JSON 404 envelope for `/api/*` misses, static frontend
/// with SPA fallback for everything else.
pub(crate) async fn fallback(State(state): State<ForgeState>, req: Request) -> Response {
    let path = req.uri().path().to_owned();
    if path == "/api" || path.starts_with("/api/") {
        return err(StatusCode::NOT_FOUND, format!("not found: {path}"));
    }
    if req.method() != Method::GET && req.method() != Method::HEAD {
        return err(StatusCode::NOT_FOUND, format!("not found: {path}"));
    }
    match &state.inner.frontend {
        Frontend::None => err(StatusCode::NOT_FOUND, format!("not found: {path}")),
        Frontend::Dir(dir) => serve_dir(dir.clone(), &path, req).await,
        #[cfg(feature = "embed")]
        Frontend::Embedded(lookup) => serve_embedded(lookup, &path),
    }
}

async fn serve_dir(dir: PathBuf, path: &str, req: Request) -> Response {
    let index = dir.join("index.html");
    let service = ServeDir::new(&dir)
        .append_index_html_on_directories(true)
        .fallback(ServeFile::new(index));
    match service.oneshot(req).await {
        Ok(res) => {
            let mut res = res.map(Body::new);
            if res.status().is_success() {
                res.headers_mut().insert(
                    header::CACHE_CONTROL,
                    HeaderValue::from_static(cache_control(path)),
                );
            }
            res
        }
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

#[cfg(feature = "embed")]
fn serve_embedded(lookup: &EmbeddedLookup, path: &str) -> Response {
    let key = path.trim_start_matches('/');
    let key = if key.is_empty() { "index.html" } else { key };
    if let Some(data) = lookup(key) {
        return embedded_response(key, data);
    }
    // SPA fallback for unknown non-/api paths.
    match lookup("index.html") {
        Some(data) => embedded_response("index.html", data),
        None => err(StatusCode::NOT_FOUND, format!("not found: {path}")),
    }
}

#[cfg(feature = "embed")]
fn embedded_response(key: &str, data: std::borrow::Cow<'static, [u8]>) -> Response {
    use axum::response::IntoResponse;
    let mime = mime_guess::from_path(key).first_or_octet_stream();
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, mime.as_ref().to_string()),
            (header::CACHE_CONTROL, cache_control(key).to_string()),
        ],
        data.into_owned(),
    )
        .into_response()
}
