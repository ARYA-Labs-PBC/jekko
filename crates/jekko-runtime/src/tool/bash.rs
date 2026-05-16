//! `bash` tool — run a shell command.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::{RuntimeError, RuntimeResult};
use crate::permission::{new_request_id, PermissionRequest};
use crate::shell;

use super::{Tool, ToolContext, ToolOutput};

const SCHEMA: &str = r#"{
  "type": "object",
  "properties": {
    "command": { "type": "string", "description": "Shell command to run (passed to sh -c)" },
    "timeout_ms": { "type": "number", "description": "Optional timeout in ms" }
  },
  "required": ["command"]
}"#;

/// Input schema for the bash tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BashInput {
    /// Command to run.
    pub command: String,
    /// Optional timeout.
    #[serde(default, rename = "timeout_ms")]
    pub timeout_ms: Option<u64>,
}

/// `bash` tool.
#[derive(Debug, Clone, Copy, Default)]
pub struct BashTool;

#[async_trait]
impl Tool for BashTool {
    fn id(&self) -> &'static str {
        "bash"
    }

    fn description(&self) -> &'static str {
        "Run a shell command inside the project worktree."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::from_str(SCHEMA).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        ctx: ToolContext,
    ) -> RuntimeResult<ToolOutput> {
        let parsed: BashInput =
            serde_json::from_value(input).map_err(|e| RuntimeError::invalid(e.to_string()))?;

        if let Some(perm) = &ctx.permissions {
            perm.ask(
                PermissionRequest {
                    id: new_request_id(),
                    session_id: ctx.session_id.clone(),
                    permission: "bash".into(),
                    patterns: vec![parsed.command.clone()],
                    metadata: serde_json::json!({ "cwd": ctx.cwd }),
                    always: vec![parsed.command.clone()],
                },
                vec![],
            )
            .await?;
        }

        let out = match parsed.timeout_ms {
            Some(ms) => {
                let fut = shell::run(&parsed.command, &ctx.cwd);
                tokio::time::timeout(std::time::Duration::from_millis(ms), fut)
                    .await
                    .map_err(|_| RuntimeError::Command("timeout".into()))??
            }
            None => shell::run(&parsed.command, &ctx.cwd).await?,
        };

        Ok(ToolOutput {
            title: format!("$ {}", parsed.command),
            output: format_output(&out),
            metadata: serde_json::json!({
                "code": out.code,
                "stderr": out.stderr,
            }),
        })
    }
}

fn format_output(out: &shell::ShellOutput) -> String {
    let mut s = String::new();
    if !out.stdout.is_empty() {
        s.push_str(&out.stdout);
    }
    if !out.stderr.is_empty() {
        if !s.is_empty() {
            s.push('\n');
        }
        s.push_str("[stderr] ");
        s.push_str(&out.stderr);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn echo_runs() {
        let tool = BashTool;
        let ctx = ToolContext::bare(".");
        let out = tool
            .execute(serde_json::json!({ "command": "printf hi" }), ctx)
            .await
            .unwrap();
        assert_eq!(out.output, "hi");
    }

    #[test]
    fn schema_parseable() {
        let _ = BashTool.schema();
    }
}
