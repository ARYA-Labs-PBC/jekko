//! `task` tool — spawn a child agent for a focused task.
//!
//! Ported from `packages/jekko/src/tool/task.ts`. The TS task tool forks
//! a child session and waits for its completion. The actual child agent
//! is dispatched by the processor pipeline (packet D); here we expose the
//! schema and a stubbed execute path that records the request without
//! running the child.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::{RuntimeError, RuntimeResult};

use super::{Tool, ToolContext, ToolOutput};

const SCHEMA: &str = r#"{
  "type": "object",
  "properties": {
    "name": { "type": "string", "description": "Subtask name" },
    "prompt": { "type": "string", "description": "Subtask prompt" }
  },
  "required": ["name", "prompt"]
}"#;

/// Input schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInput {
    /// Subtask name.
    pub name: String,
    /// Subtask prompt.
    pub prompt: String,
}

/// `task` tool.
#[derive(Debug, Clone, Copy, Default)]
pub struct TaskTool;

#[async_trait]
impl Tool for TaskTool {
    fn id(&self) -> &'static str {
        "task"
    }

    fn description(&self) -> &'static str {
        "Spawn a focused subtask agent."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::from_str(SCHEMA).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _ctx: ToolContext,
    ) -> RuntimeResult<ToolOutput> {
        let parsed: TaskInput =
            serde_json::from_value(input).map_err(|e| RuntimeError::invalid(e.to_string()))?;
        Ok(ToolOutput {
            title: format!("task {}", parsed.name),
            output: format!(
                "Task '{}' queued. Subtask dispatcher not yet wired in this runtime.",
                parsed.name
            ),
            metadata: serde_json::json!({
                "name": parsed.name,
                "prompt": parsed.prompt,
                "queued": true,
            }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn returns_queued_metadata() {
        let out = TaskTool
            .execute(
                serde_json::json!({ "name": "test", "prompt": "do thing" }),
                ToolContext::bare("."),
            )
            .await
            .unwrap();
        assert!(out.output.contains("queued"));
        assert_eq!(out.metadata["queued"], serde_json::Value::Bool(true));
    }
}
