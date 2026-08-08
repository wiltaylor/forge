//! JSON document store.
//!
//! One file per doc: `<data-dir>/<name>.json`. Names must match
//! `^[a-z0-9][a-z0-9_-]{0,63}$` — the regex doubles as the path-traversal
//! guard. Writes are atomic (tmp + rename). DELETE is idempotent.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::error::ForgeError;

/// The doc-name pattern shared by every error message and validator.
pub const NAME_PATTERN: &str = "^[a-z0-9][a-z0-9_-]{0,63}$";

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

#[cfg(test)]
mod tests {
    use super::{valid_doc_name, DocStore};
    use crate::error::ForgeError;
    use serde_json::json;

    /// A store over a fresh temp dir. Keep the `TempDir` alive for the test.
    fn store() -> (tempfile::TempDir, DocStore) {
        let dir = tempfile::tempdir().unwrap();
        let store = DocStore::new(dir.path());
        (dir, store)
    }

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

    #[tokio::test]
    async fn put_get_roundtrip_and_replace() {
        let (dir, store) = store();
        let doc = json!({"title": "hello", "items": [1, 2, 3], "nested": {"a": null}});
        store.put("notes", &doc).await.unwrap();
        assert_eq!(store.get("notes").await.unwrap(), doc);

        // Replace with a different JSON shape entirely.
        let doc2 = json!(["now", "an", "array"]);
        store.put("notes", &doc2).await.unwrap();
        assert_eq!(store.get("notes").await.unwrap(), doc2);

        // Atomic write: the doc file exists, the tmp file does not survive.
        assert!(dir.path().join("notes.json").exists());
        assert!(!dir.path().join("notes.json.tmp").exists());
    }

    #[tokio::test]
    async fn bad_names_rejected_on_every_operation() {
        let (_dir, store) = store();
        let long = "a".repeat(65);
        for name in [
            "UPPER",
            "_lead",
            "-lead",
            "has.dot",
            "",
            "../etc/passwd",
            &long,
        ] {
            assert!(
                matches!(store.get(name).await, Err(ForgeError::BadRequest(_))),
                "get {name:?}"
            );
            assert!(
                matches!(
                    store.put(name, &json!({})).await,
                    Err(ForgeError::BadRequest(_))
                ),
                "put {name:?}"
            );
            assert!(
                matches!(store.delete(name).await, Err(ForgeError::BadRequest(_))),
                "delete {name:?}"
            );
        }
    }

    #[tokio::test]
    async fn missing_doc_is_not_found() {
        let (_dir, store) = store();
        let err = store.get("nope").await.unwrap_err();
        assert!(matches!(err, ForgeError::NotFound(_)), "got {err:?}");
        assert!(err.to_string().contains("nope"));
    }

    #[tokio::test]
    async fn delete_is_idempotent() {
        let (_dir, store) = store();
        store.put("tmp", &json!(1)).await.unwrap();
        store.delete("tmp").await.unwrap();
        // Deleting again (missing) still succeeds.
        store.delete("tmp").await.unwrap();
        assert!(matches!(
            store.get("tmp").await,
            Err(ForgeError::NotFound(_))
        ));
    }

    #[tokio::test]
    async fn list_is_sorted_with_metadata() {
        let dir = tempfile::tempdir().unwrap();
        // Point at a subdirectory that does not exist yet: list = [].
        let store = DocStore::new(dir.path().join("docs"));
        assert!(store.list().await.unwrap().is_empty());

        store.put("beta", &json!({"b": 2})).await.unwrap();
        store.put("alpha", &json!({"a": 1})).await.unwrap();
        let docs = store.list().await.unwrap();
        assert_eq!(docs.len(), 2);
        // Sorted by name.
        assert_eq!(docs[0]["name"], json!("alpha"));
        assert_eq!(docs[1]["name"], json!("beta"));
        for doc in &docs {
            assert!(doc["bytes"].as_u64().unwrap() > 0);
            // modified = unix seconds as float, recent.
            let modified = doc["modified"].as_f64().unwrap();
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs_f64();
            assert!(modified > now - 60.0 && modified <= now + 1.0);
        }
    }

    #[tokio::test]
    async fn list_skips_entries_that_are_not_json_files() {
        let (dir, store) = store();
        store.put("real", &json!(1)).await.unwrap();
        std::fs::write(dir.path().join("notes.txt"), "not a doc").unwrap();
        std::fs::create_dir(dir.path().join("subdir.json")).unwrap();
        let docs = store.list().await.unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0]["name"], json!("real"));
    }

    #[tokio::test]
    async fn corrupt_doc_is_internal() {
        let (dir, store) = store();
        std::fs::write(dir.path().join("bad.json"), "{not json").unwrap();
        let err = store.get("bad").await.unwrap_err();
        assert!(matches!(err, ForgeError::Internal(_)), "got {err:?}");
        assert!(err.to_string().contains("corrupt"));
    }
}
