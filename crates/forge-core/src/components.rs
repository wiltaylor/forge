//! Component federation: the bundle-filename rule and the manifest.
//!
//! The manifest is `manifest.json` in the components directory, served with
//! the application name injected. Bundle filenames must match
//! `^[a-zA-Z0-9][a-zA-Z0-9._-]{0,127}$`, hold no `..`, and end in one of
//! `.js .mjs .css .map` — the rule doubles as the path-traversal guard.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::error::ForgeError;

/// The bundle-filename pattern shared by every error message and validator.
pub const FILE_PATTERN: &str = "^[a-zA-Z0-9][a-zA-Z0-9._-]{0,127}$";

/// Extensions a bundle file may carry.
pub const ALLOWED_EXTENSIONS: &[&str] = &[".js", ".mjs", ".css", ".map"];

/// Validate a bundle filename per the contract: `^[a-zA-Z0-9][a-zA-Z0-9._-]{0,127}$`,
/// no `..`, extension allowlist `.js .mjs .css .map` (hand-rolled, no regex crate).
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

/// Filesystem-backed component federation directory.
#[derive(Debug, Clone)]
pub struct Components {
    dir: PathBuf,
}

impl Components {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// Directory holding `manifest.json` and the bundle files.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The federation manifest with `app` injected.
    ///
    /// No `manifest.json` is an empty catalogue — `{app, components: []}` —
    /// not a 404: the contract states one response shape for this endpoint and
    /// names no error status for it, unlike the endpoints where a miss is a
    /// 404 (`/api/data/{name}`, `/api/actions/{name}`).
    pub async fn manifest(&self, app: &str) -> Result<Value, ForgeError> {
        let path = self.dir.join("manifest.json");
        let raw = match tokio::fs::read(&path).await {
            Ok(raw) => raw,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(json!({"app": app, "components": []}))
            }
            Err(e) => return Err(e.into()),
        };
        let manifest: Value = serde_json::from_slice(&raw)
            .map_err(|e| ForgeError::Internal(format!("manifest.json is not valid JSON: {e}")))?;
        Ok(match manifest {
            Value::Object(mut map) => {
                map.insert("app".into(), Value::String(app.to_string()));
                Value::Object(map)
            }
            // An array manifest is treated as the components list.
            Value::Array(components) => json!({"app": app, "components": components}),
            // Anything else cannot carry the app name, and the contract states
            // one response shape for this endpoint.
            _ => {
                return Err(ForgeError::Internal(
                    "manifest.json must be an object or an array".into(),
                ))
            }
        })
    }

    /// Path of a bundle file, once its name passes the filename rule.
    ///
    /// The rule is the path-traversal guard, so the returned path is always
    /// inside the components directory. Existence is the caller's business.
    pub fn file_path(&self, name: &str) -> Result<PathBuf, ForgeError> {
        if !valid_component_file(name) {
            return Err(ForgeError::BadRequest(format!(
                "invalid component file name: {name:?} (must match {FILE_PATTERN}, extensions {})",
                ALLOWED_EXTENSIONS.join(" ")
            )));
        }
        Ok(self.dir.join(name))
    }
}

#[cfg(test)]
mod tests {
    use super::{valid_component_file, Components};
    use serde_json::json;

    #[test]
    fn filename_validation() {
        assert!(valid_component_file("widget.js"));
        assert!(valid_component_file("Widget-1.2.3.mjs"));
        assert!(valid_component_file("styles.css"));
        assert!(valid_component_file("bundle.js.map"));
        assert!(valid_component_file(&format!("{}.js", "a".repeat(125))));
        assert!(!valid_component_file("evil.sh"));
        assert!(!valid_component_file(".hidden.js"));
        assert!(!valid_component_file("no-ext"));
        assert!(!valid_component_file("a/../b.js"));
        assert!(!valid_component_file("a..b.js"));
        assert!(!valid_component_file(""));
        assert!(!valid_component_file(&format!("{}.js", "a".repeat(126))));
    }

    #[test]
    fn file_path_rejects_traversal_and_joins_valid_names() {
        let dir = tempfile::tempdir().expect("tempdir");
        let components = Components::new(dir.path());

        let err = components.file_path("../secret.js").expect_err("traversal");
        assert_eq!(err.status(), 400);

        assert_eq!(
            components.file_path("widget.js").expect("valid name"),
            dir.path().join("widget.js")
        );
    }

    #[tokio::test]
    async fn absent_manifest_is_an_empty_catalogue() {
        let dir = tempfile::tempdir().expect("tempdir");
        let components = Components::new(dir.path());

        assert_eq!(
            components.manifest("demo").await.expect("manifest"),
            json!({"app": "demo", "components": []})
        );
    }

    #[tokio::test]
    async fn object_manifest_gets_the_app_name_injected() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("manifest.json"),
            r#"{"app": "stale", "components": [{"name": "Widget"}]}"#,
        )
        .expect("write manifest");

        assert_eq!(
            Components::new(dir.path())
                .manifest("demo")
                .await
                .expect("manifest"),
            json!({"app": "demo", "components": [{"name": "Widget"}]})
        );
    }

    #[tokio::test]
    async fn array_manifest_is_the_components_list() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("manifest.json"), r#"[{"name": "Widget"}]"#)
            .expect("write manifest");

        assert_eq!(
            Components::new(dir.path())
                .manifest("demo")
                .await
                .expect("manifest"),
            json!({"app": "demo", "components": [{"name": "Widget"}]})
        );
    }

    #[tokio::test]
    async fn corrupt_manifest_is_a_500() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("manifest.json"), "{ not json").expect("write manifest");

        let err = Components::new(dir.path())
            .manifest("demo")
            .await
            .expect_err("corrupt manifest");
        assert_eq!(err.status(), 500);
    }

    #[tokio::test]
    async fn manifest_that_is_neither_object_nor_array_is_a_500() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("manifest.json"), r#""hello""#).expect("write manifest");

        let err = Components::new(dir.path())
            .manifest("demo")
            .await
            .expect_err("scalar manifest");
        assert_eq!(err.status(), 500);
    }
}
