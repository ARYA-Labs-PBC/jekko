//! Prompt parsing.
//!
//! Ported from `packages/jekko/src/session/prompt.ts`. The TS prompt
//! parser extracts inline `@agent` mentions and file attachments from a
//! raw text prompt. This module exposes the same surface in a smaller
//! footprint: it returns a structured [`ParsedPrompt`] without trying to
//! interpret the rest of the prompt body.

use serde::{Deserialize, Serialize};

/// Result of [`parse`].
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ParsedPrompt {
    /// Cleaned plain-text portion of the prompt.
    pub text: String,
    /// Inline `@agent` references found in the prompt body.
    pub agents: Vec<String>,
    /// Inline file paths referenced by `@/path/to/file`.
    pub files: Vec<String>,
}

/// Parse a prompt, extracting `@agent` / `@/path` references.
pub fn parse(raw: &str) -> ParsedPrompt {
    let mut agents = Vec::new();
    let mut files = Vec::new();
    let mut text = String::with_capacity(raw.len());

    let bytes = raw.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'@' && (i == 0 || matches!(bytes[i - 1], b' ' | b'\n' | b'\t')) {
            // Extract a path or agent name following the @.
            let start = i + 1;
            let mut j = start;
            while j < bytes.len() {
                let b = bytes[j];
                if b.is_ascii_whitespace() {
                    break;
                }
                j += 1;
            }
            if j == start {
                text.push('@');
                i += 1;
                continue;
            }
            let token = &raw[start..j];
            if token.starts_with('/') || token.starts_with("./") {
                files.push(token.to_string());
            } else {
                agents.push(token.to_string());
            }
            // Preserve a marker in the cleaned text so the LLM still sees
            // the mention. The TS parser strips and re-attaches them
            // separately; we keep it simple here.
            text.push_str(&raw[i..j]);
            i = j;
        } else {
            text.push(bytes[i] as char);
            i += 1;
        }
    }
    ParsedPrompt {
        text,
        agents,
        files,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_agent_and_file() {
        let parsed = parse("hi @planner please look at @/foo/bar.rs");
        assert_eq!(parsed.agents, vec!["planner"]);
        assert_eq!(parsed.files, vec!["/foo/bar.rs"]);
    }

    #[test]
    fn no_mention_no_extraction() {
        let parsed = parse("hello world");
        assert!(parsed.agents.is_empty());
        assert!(parsed.files.is_empty());
    }

    #[test]
    fn email_like_does_not_count() {
        let parsed = parse("alice@example.com is fine");
        assert!(parsed.agents.is_empty());
    }
}
