//! YAML-ish lexer for the agent-script preview pane.
//!
//! Ports `packages/jekko/src/cli/cmd/tui/util/yaml-tokenize.ts`. The TS version
//! lights up YAML in the prompt while the user is mid-paste, so it has to be
//! tolerant — partial input, weird block scalars, ZYAL sentinel envelopes.
//! This Rust port mirrors the same behaviour: it never panics and always
//! returns a stable, non-overlapping list of [`YamlSpan`]s.
//!
//! Offsets are byte offsets into the input.
//!
//! The implementation splits across three internal modules so each piece stays
//! easy to read on its own:
//!
//! * [`tokens`] — public [`YamlScope`] / [`YamlSpan`] data and shared helpers.
//! * [`scanner`] — per-line dispatch loops (regular YAML and block scalars).
//! * [`recognizers`] — per-pattern matchers (numbers, booleans, quoted strings,
//!   sentinels, comments).

mod recognizers;
mod scanner;
mod tokens;

#[cfg(test)]
mod tests;

pub use tokens::{YamlScope, YamlSpan};

use recognizers::{find_comment_start, indentation, is_sentinel, line_starts_block_scalar};
use scanner::{tokenize_block_line, tokenize_yaml_code};
use tokens::{push, NotBool};

/// Tokenize a YAML-ish text buffer. Always returns a vector (possibly empty).
pub fn tokenize_yaml(text: &str) -> Vec<YamlSpan> {
    let mut tokens: Vec<YamlSpan> = Vec::new();
    let mut offset: usize = 0;
    let mut block_parent_indent: Option<usize> = None;

    for line in text.split('\n') {
        let line_end = offset + line.len();
        let indent = indentation(line);
        let non_blank = line.trim().is_empty().not_bool();

        if let Some(parent) = block_parent_indent {
            if non_blank && indent <= parent {
                block_parent_indent = None;
            }
        }

        if is_sentinel(line) {
            push(&mut tokens, offset, line_end, YamlScope::Sentinel);
            offset = line_end + 1;
            continue;
        }

        if block_parent_indent.is_some() {
            tokenize_block_line(line, offset, &mut tokens);
            offset = line_end + 1;
            continue;
        }

        let hash_idx = find_comment_start(line);
        let code_end = hash_idx.unwrap_or(line.len());
        let code_part = &line[..code_end];

        tokenize_yaml_code(code_part, offset, &mut tokens);
        if line_starts_block_scalar(code_part) {
            block_parent_indent = Some(indent);
        }

        if let Some(idx) = hash_idx {
            push(&mut tokens, offset + idx, line_end, YamlScope::Comment);
        }

        offset = line_end + 1;
    }

    tokens
}
