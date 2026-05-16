//! Processor pipeline.
//!
//! Ported from `packages/jekko/src/session/processor.ts`. The TS pipeline
//! assembles a system prompt, injects the tool catalog, runs the LLM
//! turn, and routes tool calls. Here we expose only the assembly side —
//! the LLM dispatch lives in `jekko-provider` (packet D).

use serde::{Deserialize, Serialize};

use crate::tool::ToolDescriptor;

/// Pieces that make up the system prompt for one LLM turn.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemPromptParts {
    /// Static instructions (e.g. "you are a Jekko agent…").
    pub instructions: String,
    /// Per-session context (project tree summary, etc.).
    pub context: Vec<String>,
    /// Inline reminders ("Remember to test before claiming done").
    pub reminders: Vec<String>,
}

/// Concatenate parts into a single system prompt body.
pub fn assemble_system_prompt(parts: &SystemPromptParts) -> String {
    let mut out = String::new();
    out.push_str(parts.instructions.trim_start());
    for c in &parts.context {
        out.push_str("\n\n");
        out.push_str(c.trim());
    }
    for r in &parts.reminders {
        out.push_str("\n\nReminder: ");
        out.push_str(r.trim());
    }
    out
}

/// Tool catalog descriptor sent to the LLM. Mirrors the shape that
/// `processor.ts#injectTools` emits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCatalogEntry {
    /// Tool id.
    pub id: String,
    /// Human-readable description.
    pub description: String,
    /// JSON Schema for the tool input.
    pub schema: serde_json::Value,
}

impl From<ToolDescriptor> for ToolCatalogEntry {
    fn from(td: ToolDescriptor) -> Self {
        Self {
            id: td.id,
            description: td.description,
            schema: td.schema,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assembles_instructions_plus_context() {
        let parts = SystemPromptParts {
            instructions: "You are an agent.".into(),
            context: vec!["project: jekko".into()],
            reminders: vec!["be terse".into()],
        };
        let prompt = assemble_system_prompt(&parts);
        assert!(prompt.contains("agent"));
        assert!(prompt.contains("project: jekko"));
        assert!(prompt.contains("Reminder: be terse"));
    }
}
