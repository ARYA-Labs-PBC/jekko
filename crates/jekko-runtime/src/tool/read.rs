//! `read` tool — read a file (with offset/limit and line-truncation).

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::{RuntimeError, RuntimeResult};

use super::{Tool, ToolContext, ToolOutput};

const DEFAULT_LIMIT: usize = 2000;
const MAX_LINE_LENGTH: usize = 2000;

const SCHEMA: &str = r#"{
  "type": "object",
  "properties": {
    "filePath": { "type": "string", "description": "Absolute path to the file" },
    "offset": { "type": "number", "description": "1-indexed start line" },
    "limit": { "type": "number", "description": "Max lines to read (default 2000)" }
  },
  "required": ["filePath"]
}"#;

/// Input schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadInput {
    /// File path.
    #[serde(rename = "filePath")]
    pub file_path: String,
    /// 1-indexed start line.
    #[serde(default)]
    pub offset: Option<usize>,
    /// Max lines to read.
    #[serde(default)]
    pub limit: Option<usize>,
}

/// `read` tool.
#[derive(Debug, Clone, Copy, Default)]
pub struct ReadTool;

#[async_trait]
impl Tool for ReadTool {
    fn id(&self) -> &'static str {
        "read"
    }

    fn description(&self) -> &'static str {
        "Read a file from disk, returning a line-numbered slice."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::from_str(SCHEMA).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _ctx: ToolContext,
    ) -> RuntimeResult<ToolOutput> {
        let parsed: ReadInput =
            serde_json::from_value(input).map_err(|e| RuntimeError::invalid(e.to_string()))?;

        let path = parsed.file_path.clone();
        let bytes = tokio::fs::read(&path).await?;
        let text = String::from_utf8_lossy(&bytes).into_owned();

        let offset = parsed.offset.unwrap_or(1).max(1);
        let limit = parsed.limit.unwrap_or(DEFAULT_LIMIT);
        let mut out = String::new();
        let mut count = 0usize;
        for (idx, line) in text.lines().enumerate() {
            let lineno = idx + 1;
            if lineno < offset {
                continue;
            }
            if count >= limit {
                break;
            }
            let mut line_str = line.to_string();
            if line_str.len() > MAX_LINE_LENGTH {
                line_str.truncate(MAX_LINE_LENGTH);
                line_str.push_str("... (line truncated)");
            }
            out.push_str(&format!("{:>5}\t{}\n", lineno, line_str));
            count += 1;
        }

        Ok(ToolOutput {
            title: format!("read {path}"),
            output: out,
            metadata: serde_json::json!({
                "lines": count,
                "path": path,
            }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn reads_with_line_numbers() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("x.txt");
        std::fs::write(&path, "alpha\nbeta\ngamma\n").unwrap();
        let tool = ReadTool;
        let out = tool
            .execute(
                serde_json::json!({ "filePath": path.to_string_lossy() }),
                ToolContext::bare(dir.path()),
            )
            .await
            .unwrap();
        assert!(out.output.contains("    1\talpha"));
        assert!(out.output.contains("    3\tgamma"));
    }

    #[tokio::test]
    async fn applies_offset_and_limit() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("x.txt");
        std::fs::write(&path, "a\nb\nc\nd\n").unwrap();
        let tool = ReadTool;
        let out = tool
            .execute(
                serde_json::json!({ "filePath": path.to_string_lossy(), "offset": 2, "limit": 2 }),
                ToolContext::bare(dir.path()),
            )
            .await
            .unwrap();
        assert!(out.output.contains("    2\tb"));
        assert!(out.output.contains("    3\tc"));
        assert!(!out.output.contains("    4\td"));
    }
}
