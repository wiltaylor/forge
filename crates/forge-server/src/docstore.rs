//! JSON document store (playpen lineage).
//!
//! One file per doc: `<data-dir>/<name>.json`. Names must match
//! `^[a-z0-9][a-z0-9_-]{0,63}$` — the regex doubles as the path-traversal
//! guard. Writes are atomic (tmp + rename). DELETE is idempotent.

use std::path::{Path, PathBuf};

use axum::body::Bytes;
use axum::extract::{Path as UrlPath, State};
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use serde_json::{json, Value};

use crate::envelope::{err, ok, ok_empty};
use crate::error::ForgeError;
use crate::state::ForgeState;

pub(crate) const NAME_PATTERN: &str = "^[a-z0-9][a-z0-9_-]{0,63}$";

/// Validate a doc name against `^[a-z0-9][a-z0-9_-]{0,63}$` (hand-rolled,
/// no regex crate).
pub fn valid_doc_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    if bytes.is_empty() || bytes.len() > 64 {
        return false;
    }
    if !matches!(bytes[0], b'a'..=b'z' | b'0'..=b'9') {
        return false;
    }
    bytes[1..]
        .iter()
        .all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-'))
}

/// Filesystem-backed JSON document store.
#[derive(Debug, Clone)]
pub struct DocStore {
    dir: PathBuf,
}

impl DocStore {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// Directory holding the `<name>.json` files.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn doc_path(&self, name: &str) -> Result<PathBuf, ForgeError> {
        if !valid_doc_name(name) {
            return Err(ForgeError::BadRequest(format!(
                "invalid document name: {name:?} (must match {NAME_PATTERN})"
            )));
        }
        Ok(self.dir.join(format!("{name}.json")))
    }

    /// List docs as `[{name, bytes, modified}]` (modified = unix secs, float).
    pub async fn list(&self) -> Result<Vec<Value>, ForgeError> {
        let mut docs = Vec::new();
        let mut rd = match tokio::fs::read_dir(&self.dir).await {
            Ok(rd) => rd,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(docs),
            Err(e) => return Err(e.into()),
        };
        while let Some(entry) = rd.next_entry().await? {
            let path = entry.path();
            let Some(name) = path
                .file_name()
                .and_then(|f| f.to_str())
                .and_then(|f| f.strip_suffix(".json"))
            else {
                continue;
            };
            let meta = entry.metadata().await?;
            if !meta.is_file() {
                continue;
            }
            let modified = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs_f64())
                .unwrap_or(0.0);
            docs.push(json!({
                "name": name,
                "bytes": meta.len(),
                "modified": modified,
            }));
        }
        docs.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
        Ok(docs)
    }

    /// Read a doc. 404 when missing, 400 on invalid name.
    pub async fn get(&self, name: &str) -> Result<Value, ForgeError> {
        let path = self.doc_path(name)?;
        let raw = match tokio::fs::read(&path).await {
            Ok(raw) => raw,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(ForgeError::NotFound(format!("no document {name:?}")))
            }
            Err(e) => return Err(e.into()),
        };
        serde_json::from_slice(&raw)
            .map_err(|e| ForgeError::Internal(format!("document {name:?} is corrupt: {e}")))
    }

    /// Create/replace a doc atomically (write `<name>.json.tmp`, then rename).
    pub async fn put(&self, name: &str, value: &Value) -> Result<(), ForgeError> {
        let path = self.doc_path(name)?;
        tokio::fs::create_dir_all(&self.dir).await?;
        let tmp = self.dir.join(format!("{name}.json.tmp"));
        let body = serde_json::to_vec_pretty(value)?;
        tokio::fs::write(&tmp, body).await?;
        tokio::fs::rename(&tmp, &path).await?;
        Ok(())
    }

    /// Idempotent delete.
    pub async fn delete(&self, name: &str) -> Result<(), ForgeError> {
        let path = self.doc_path(name)?;
        match tokio::fs::remove_file(&path).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}

pub(crate) fn routes() -> Router<ForgeState> {
    Router::new()
        .route("/api/data", get(list_docs))
        .route(
            "/api/data/{name}",
            get(get_doc).put(put_doc).delete(delete_doc),
        )
}

fn store(state: &ForgeState) -> &DocStore {
    // Routes are only mounted when the doc store is configured.
    state.docstore().expect("docstore routes mounted without a store")
}

async fn list_docs(State(state): State<ForgeState>) -> Response {
    match store(&state).list().await {
        Ok(docs) => ok(docs),
        Err(e) => e.into_response(),
    }
}

async fn get_doc(State(state): State<ForgeState>, UrlPath(name): UrlPath<String>) -> Response {
    match store(&state).get(&name).await {
        Ok(doc) => ok(doc),
        Err(e) => e.into_response(),
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
                return err(StatusCode::BAD_REQUEST, format!("body is not valid JSON: {e}"))
            }
        }
    };
    match store(&state).put(&name, &value).await {
        Ok(()) => ok_empty(),
        Err(e) => e.into_response(),
    }
}

async fn delete_doc(State(state): State<ForgeState>, UrlPath(name): UrlPath<String>) -> Response {
    match store(&state).delete(&name).await {
        Ok(()) => ok_empty(),
        Err(e) => e.into_response(),
    }
}

use axum::response::IntoResponse;

#[cfg(test)]
mod tests {
    use super::valid_doc_name;

    #[test]
    fn name_validation() {
        assert!(valid_doc_name("a"));
        assert!(valid_doc_name("a0_b-c"));
        assert!(valid_doc_name(&"a".repeat(64)));
        assert!(!valid_doc_name(""));
        assert!(!valid_doc_name(&"a".repeat(65)));
        assert!(!valid_doc_name("_leading"));
        assert!(!valid_doc_name("-leading"));
        assert!(!valid_doc_name("UPPER"));
        assert!(!valid_doc_name("has.dot"));
        assert!(!valid_doc_name("../etc/passwd"));
    }
}
