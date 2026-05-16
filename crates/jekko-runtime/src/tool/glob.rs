//! `glob` tool — recursive glob matching.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::{RuntimeError, RuntimeResult};
use crate::file::glob;

use super::{Tool, ToolContext, ToolOutput};

const SCHEMA: &str = r#"{
  "type": "object",
  "properties": {
    "pattern": { "type": "string", "description": "Glob pattern (supports *, **, ?)" },
    "base": { "type": "string", "description": "Base directory (defaults to cwd)" }
  },
  "required": ["pattern"]
}"#;

/// Input schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobInput {
    /// Glob pattern.
    pub pattern: String,
    /// Optional base.
    #[serde(default)]
    pub base: Option<String>,
}

/// `glob` tool.
#[derive(Debug, Clone, Copy, Default)]
pub struct GlobTool;

#[async_trait]
impl Tool for GlobTool {
    fn id(&self) -> &'static str {
        "glob"
    }

    fn description(&self) -> &'static str {
        "Glob files matching a pattern, returning a newline-separated list."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::from_str(SCHEMA).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        ctx: ToolContext,
    ) -> RuntimeResult<ToolOutput> {
        let parsed: GlobInput =
            serde_json::from_value(input).map_err(|e| RuntimeError::invalid(e.to_string()))?;
        let base = match parsed.base {
            Some(b) => b,
            None => ctx.cwd.to_string_lossy().into_owned(),
        };
        let hits = glob(&base, &parsed.pattern)?;
        let body = hits
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("\n");
        Ok(ToolOutput {
            title: format!("glob {} (in {})", parsed.pattern, base),
            output: body,
            metadata: serde_json::json!({ "count": hits.len() }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn finds_rs_files() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("a")).unwrap();
        std::fs::write(dir.path().join("a/x.rs"), b"").unwrap();
        std::fs::write(dir.path().join("y.rs"), b"").unwrap();
        let out = GlobTool
            .execute(
                serde_json::json!({ "pattern": "**/*.rs", "base": dir.path().to_string_lossy() }),
                ToolContext::bare(dir.path()),
            )
            .await
            .unwrap();
        assert!(out.output.contains("x.rs"));
        assert!(out.output.contains("y.rs"));
    }
}
