//! `edit` tool — apply a string substitution to a file.
//!
//! Ported from `packages/jekko/src/tool/edit.ts`. The TS version supports
//! patch-based edits and "anchored" replacements; this port covers the
//! common case (`old_string` -> `new_string` with optional `replace_all`).
//! The patch-application logic lives in the `apply_patch` submodule.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::{RuntimeError, RuntimeResult};
use crate::file::{read_file, write_file, DEFAULT_MAX_BYTES};

use super::{Tool, ToolContext, ToolOutput};

pub mod apply_patch;

const SCHEMA: &str = r#"{
  "type": "object",
  "properties": {
    "filePath": { "type": "string", "description": "Absolute path to the file" },
    "old_string": { "type": "string", "description": "Exact text to replace" },
    "new_string": { "type": "string", "description": "Replacement text" },
    "replace_all": { "type": "boolean", "description": "Replace every occurrence" }
  },
  "required": ["filePath", "old_string", "new_string"]
}"#;

/// Input schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditInput {
    /// File path.
    #[serde(rename = "filePath")]
    pub file_path: String,
    /// Text to replace.
    pub old_string: String,
    /// New text to substitute.
    pub new_string: String,
    /// Whether to replace every match.
    #[serde(default)]
    pub replace_all: bool,
}

/// `edit` tool.
#[derive(Debug, Clone, Copy, Default)]
pub struct EditTool;

#[async_trait]
impl Tool for EditTool {
    fn id(&self) -> &'static str {
        "edit"
    }

    fn description(&self) -> &'static str {
        "Apply a string substitution to a file."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::from_str(SCHEMA).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _ctx: ToolContext,
    ) -> RuntimeResult<ToolOutput> {
        let parsed: EditInput =
            serde_json::from_value(input).map_err(|e| RuntimeError::invalid(e.to_string()))?;
        let body = read_file(&parsed.file_path, DEFAULT_MAX_BYTES.max(10 * 1024 * 1024)).await?;
        let updated = if parsed.replace_all {
            body.replace(&parsed.old_string, &parsed.new_string)
        } else {
            if body.matches(&parsed.old_string).count() != 1 {
                return Err(RuntimeError::invalid(format!(
                    "edit: old_string must match exactly once in {} (found {})",
                    parsed.file_path,
                    body.matches(&parsed.old_string).count()
                )));
            }
            body.replacen(&parsed.old_string, &parsed.new_string, 1)
        };
        if updated == body {
            return Err(RuntimeError::invalid(
                "edit: old_string not found in file".to_string(),
            ));
        }
        write_file(&parsed.file_path, updated.as_bytes()).await?;
        Ok(ToolOutput {
            title: format!("edit {}", parsed.file_path),
            output: format!("Applied edit to {}", parsed.file_path),
            metadata: serde_json::json!({
                "path": parsed.file_path,
                "replace_all": parsed.replace_all,
            }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn replaces_once() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("x.txt");
        std::fs::write(&path, "alpha beta gamma").unwrap();
        let out = EditTool
            .execute(
                serde_json::json!({ "filePath": path.to_string_lossy(), "old_string": "beta", "new_string": "BETA" }),
                ToolContext::bare(dir.path()),
            )
            .await
            .unwrap();
        assert!(out.output.contains("Applied"));
        let read = std::fs::read_to_string(&path).unwrap();
        assert_eq!(read, "alpha BETA gamma");
    }

    #[tokio::test]
    async fn errors_on_duplicates_without_replace_all() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("x.txt");
        std::fs::write(&path, "aa aa aa").unwrap();
        let err = EditTool
            .execute(
                serde_json::json!({ "filePath": path.to_string_lossy(), "old_string": "aa", "new_string": "bb" }),
                ToolContext::bare(dir.path()),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, RuntimeError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn replace_all_works() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("x.txt");
        std::fs::write(&path, "aa aa aa").unwrap();
        EditTool
            .execute(
                serde_json::json!({ "filePath": path.to_string_lossy(), "old_string": "aa", "new_string": "bb", "replace_all": true }),
                ToolContext::bare(dir.path()),
            )
            .await
            .unwrap();
        let read = std::fs::read_to_string(&path).unwrap();
        assert_eq!(read, "bb bb bb");
    }
}
