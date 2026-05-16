//! `write` tool — write a file (atomic).

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::{RuntimeError, RuntimeResult};
use crate::file::write_file;

use super::{Tool, ToolContext, ToolOutput};

const SCHEMA: &str = r#"{
  "type": "object",
  "properties": {
    "filePath": { "type": "string", "description": "Absolute path to the file" },
    "content": { "type": "string", "description": "Contents to write" }
  },
  "required": ["filePath", "content"]
}"#;

/// Input schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteInput {
    /// File path.
    #[serde(rename = "filePath")]
    pub file_path: String,
    /// Contents.
    pub content: String,
}

/// `write` tool.
#[derive(Debug, Clone, Copy, Default)]
pub struct WriteTool;

#[async_trait]
impl Tool for WriteTool {
    fn id(&self) -> &'static str {
        "write"
    }

    fn description(&self) -> &'static str {
        "Atomically write a file (creates the parent dir if needed)."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::from_str(SCHEMA).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _ctx: ToolContext,
    ) -> RuntimeResult<ToolOutput> {
        let parsed: WriteInput =
            serde_json::from_value(input).map_err(|e| RuntimeError::invalid(e.to_string()))?;
        write_file(&parsed.file_path, parsed.content.as_bytes()).await?;
        let bytes = parsed.content.len() as u64;
        Ok(ToolOutput {
            title: format!("wrote {}", parsed.file_path),
            output: format!("Wrote {bytes} bytes to {}", parsed.file_path),
            metadata: serde_json::json!({ "bytes": bytes }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn writes_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("a/b/c.txt");
        let out = WriteTool
            .execute(
                serde_json::json!({ "filePath": path.to_string_lossy(), "content": "hello" }),
                ToolContext::bare(dir.path()),
            )
            .await
            .unwrap();
        assert!(out.output.contains("Wrote 5"));
        let read = std::fs::read_to_string(&path).unwrap();
        assert_eq!(read, "hello");
    }
}
