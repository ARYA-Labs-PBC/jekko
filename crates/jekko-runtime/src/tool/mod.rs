//! Tool trait + registry + ported tool implementations.
//!
//! Ported from `packages/jekko/src/tool/`. Each tool is a small async
//! function that takes a JSON input, runs side effects, and returns a
//! [`ToolOutput`]. Tools register themselves in the [`Registry`] so the
//! processor pipeline can hand them to the LLM as a catalog.

pub mod bash;
pub mod edit;
pub mod glob;
pub mod grep;
pub mod read;
pub mod task;
pub mod webfetch;
pub mod websearch;
pub mod write;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::RuntimeResult;
use crate::permission::PermissionService;
use crate::session::{SessionService, SessionStore};

pub use bash::BashTool;
pub use edit::EditTool;
pub use glob::GlobTool;
pub use grep::GrepTool;
pub use read::ReadTool;
pub use task::TaskTool;
pub use webfetch::WebFetchTool;
pub use websearch::WebSearchTool;
pub use write::WriteTool;

/// One tool exposed to the LLM.
#[async_trait]
pub trait Tool: Send + Sync + std::fmt::Debug {
    /// Stable tool id (matches the TS `id` field).
    fn id(&self) -> &'static str;

    /// Human-readable description (sent to the LLM).
    fn description(&self) -> &'static str;

    /// JSON schema describing the input parameters.
    fn schema(&self) -> serde_json::Value;

    /// Execute the tool.
    async fn execute(
        &self,
        input: serde_json::Value,
        ctx: ToolContext,
    ) -> RuntimeResult<ToolOutput>;
}

/// Execution context handed to a tool.
#[derive(Clone, Debug)]
pub struct ToolContext {
    /// Session id.
    pub session_id: String,
    /// Message id that issued the tool call.
    pub message_id: String,
    /// Agent name (e.g. `"planner"`).
    pub agent: String,
    /// Working directory.
    pub cwd: PathBuf,
    /// Permission service (optional in tests).
    pub permissions: Option<Arc<PermissionService>>,
    /// Session service (optional in tests).
    pub sessions: Option<Arc<SessionService>>,
    /// Free-form extras.
    pub extra: serde_json::Value,
}

impl ToolContext {
    /// Construct a minimal context (handy for tests).
    pub fn bare(cwd: impl Into<PathBuf>) -> Self {
        Self {
            session_id: String::new(),
            message_id: String::new(),
            agent: "default".into(),
            cwd: cwd.into(),
            permissions: None,
            sessions: None,
            extra: serde_json::json!({}),
        }
    }
}

/// Result of a tool execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolOutput {
    /// Short title (shown in UI).
    pub title: String,
    /// Free-form output text.
    pub output: String,
    /// Structured metadata that survives in the message log.
    #[serde(default)]
    pub metadata: serde_json::Value,
}

impl ToolOutput {
    /// Helper to build an output from just a title + body.
    pub fn text(title: impl Into<String>, output: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            output: output.into(),
            metadata: serde_json::json!({}),
        }
    }
}

/// Public-facing descriptor for the LLM tool catalog.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDescriptor {
    /// Tool id.
    pub id: String,
    /// Description.
    pub description: String,
    /// JSON schema.
    pub schema: serde_json::Value,
}

impl ToolDescriptor {
    /// Build a descriptor from any [`Tool`] reference.
    pub fn from_tool(tool: &dyn Tool) -> Self {
        Self {
            id: tool.id().to_string(),
            description: tool.description().to_string(),
            schema: tool.schema(),
        }
    }
}

/// Async registry of available tools, keyed by id.
#[derive(Debug, Default)]
pub struct Registry {
    inner: HashMap<&'static str, Arc<dyn Tool>>,
}

impl Registry {
    /// Construct an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a tool. The first registration for a given id wins; we
    /// expose [`Self::register_or_replace`] for tests that need to swap
    /// the impl.
    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        let id = tool.id();
        self.inner.entry(id).or_insert(tool);
    }

    /// Register a tool, replacing any existing entry with the same id.
    pub fn register_or_replace(&mut self, tool: Arc<dyn Tool>) {
        self.inner.insert(tool.id(), tool);
    }

    /// Number of registered tools.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Lookup a tool by id.
    pub fn get(&self, id: &str) -> Option<Arc<dyn Tool>> {
        self.inner.get(id).cloned()
    }

    /// Lookup a tool by id with two explicit attempts: exact-case, then
    /// lowercase normalisation. Returns `None` when neither key resolves.
    /// Callers receiving `None` should emit an `unknown tool` diagnostic so
    /// the agent loop can correct the tool name on the next turn.
    pub fn get_case_insensitive(&self, id: &str) -> Option<Arc<dyn Tool>> {
        match self.inner.get(id) {
            Some(tool) => Some(tool.clone()),
            None => {
                let lowered = id.to_lowercase();
                if lowered == id {
                    None
                } else {
                    self.inner.get(lowered.as_str()).cloned()
                }
            }
        }
    }

    /// Iterate over descriptors in id order.
    pub fn catalog(&self) -> Vec<ToolDescriptor> {
        let mut out: Vec<_> = self
            .inner
            .values()
            .map(|tool| ToolDescriptor::from_tool(tool.as_ref()))
            .collect();
        out.sort_by(|a, b| a.id.cmp(&b.id));
        out
    }
}

/// Construct a registry pre-populated with the built-in tool set.
pub fn default_registry() -> Registry {
    let mut reg = Registry::new();
    reg.register(Arc::new(BashTool));
    reg.register(Arc::new(ReadTool));
    reg.register(Arc::new(WriteTool));
    reg.register(Arc::new(EditTool));
    reg.register(Arc::new(GlobTool));
    reg.register(Arc::new(GrepTool));
    reg.register(Arc::new(WebFetchTool));
    reg.register(Arc::new(WebSearchTool));
    reg.register(Arc::new(TaskTool));
    reg
}

#[allow(dead_code)]
fn _check_store_trait<T: SessionStore>() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_catalog_sorted_by_id() {
        let reg = default_registry();
        let cat = reg.catalog();
        let mut ids: Vec<_> = cat.iter().map(|d| d.id.clone()).collect();
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(ids, sorted);

        // Re-borrow to keep the assertion explicit:
        ids.sort();
        assert!(ids.contains(&"bash".to_string()));
        assert!(ids.contains(&"read".to_string()));
        assert!(ids.contains(&"glob".to_string()));
    }
}
