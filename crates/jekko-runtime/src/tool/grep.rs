//! `grep` tool — wraps [`crate::ripgrep`].

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::{RuntimeError, RuntimeResult};
use crate::ripgrep;

use super::{Tool, ToolContext, ToolOutput};

const SCHEMA: &str = r#"{
  "type": "object",
  "properties": {
    "pattern": { "type": "string", "description": "Regex pattern" },
    "base": { "type": "string", "description": "Base directory (defaults to cwd)" }
  },
  "required": ["pattern"]
}"#;

/// Input schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrepInput {
    /// Regex pattern.
    pub pattern: String,
    /// Optional base.
    #[serde(default)]
    pub base: Option<String>,
}

/// `grep` tool.
#[derive(Debug, Clone, Copy, Default)]
pub struct GrepTool;

#[async_trait]
impl Tool for GrepTool {
    fn id(&self) -> &'static str {
        "grep"
    }

    fn description(&self) -> &'static str {
        "Search file contents for a regex (uses ripgrep when available)."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::from_str(SCHEMA).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        ctx: ToolContext,
    ) -> RuntimeResult<ToolOutput> {
        let parsed: GrepInput =
            serde_json::from_value(input).map_err(|e| RuntimeError::invalid(e.to_string()))?;
        let base = match parsed.base {
            Some(b) => b,
            None => ctx.cwd.to_string_lossy().into_owned(),
        };
        let matches = ripgrep::grep(&base, &parsed.pattern).await?;
        let body = matches
            .iter()
            .map(|m| format!("{}:{}:{}", m.path.display(), m.line, m.text))
            .collect::<Vec<_>>()
            .join("\n");
        Ok(ToolOutput {
            title: format!("grep {} (in {})", parsed.pattern, base),
            output: body,
            metadata: serde_json::json!({ "count": matches.len() }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn finds_match() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("x.txt"), b"hello world\nfoo bar\n").unwrap();
        let out = GrepTool
            .execute(
                serde_json::json!({ "pattern": "foo", "base": dir.path().to_string_lossy() }),
                ToolContext::bare(dir.path()),
            )
            .await
            .unwrap();
        assert!(out.output.contains("foo"));
    }
}
