//! Driver for `tokenize_terminal`.

use super::matchers::{match_keyword, match_number, match_status_badge, match_time, scan_quoted};
use super::types::{TerminalScope, TerminalSpan, MAX_TOKENIZE_LENGTH};

/// Tokenise the given (already-ANSI-stripped) text into a non-overlapping,
/// start-ordered list of spans.
pub fn tokenize_terminal(text: &str) -> Vec<TerminalSpan> {
    let source = if text.len() > MAX_TOKENIZE_LENGTH {
        &text[..MAX_TOKENIZE_LENGTH]
    } else {
        text
    };
    let bytes = source.as_bytes();
    let mut tokens: Vec<TerminalSpan> = Vec::new();

    // Strings (double quoted) — also act as scope anchors for command words.
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'"' || b == b'\'' {
            if let Some(end) = scan_quoted(bytes, i, b) {
                tokens.push(TerminalSpan {
                    start: i,
                    end,
                    scope: TerminalScope::StringLit,
                });
                i = end;
                continue;
            }
        }
        i += 1;
    }

    // Prompts + commands (line-level).
    for line in source.lines() {
        let line_start = line.as_ptr() as usize - source.as_ptr() as usize;
        let trimmed_start = line
            .bytes()
            .take_while(|b| *b == b' ' || *b == b'\t')
            .count();
        let cut = &line[trimmed_start..];
        let prompt_char = cut.bytes().next();
        if let Some(ch) = prompt_char {
            if matches!(ch, b'$' | b'#' | b'>') {
                let sigil_start = line_start + trimmed_start;
                tokens.push(TerminalSpan {
                    start: sigil_start,
                    end: sigil_start + 1,
                    scope: TerminalScope::Prompt,
                });
                // Skip whitespace after sigil and grab the command word.
                let rest = &cut[1..];
                let lead_ws = rest
                    .bytes()
                    .take_while(|b| *b == b' ' || *b == b'\t')
                    .count();
                let word: String = rest[lead_ws..]
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
                    .collect();
                if !word.is_empty() {
                    let cmd_start = sigil_start + 1 + lead_ws;
                    tokens.push(TerminalSpan {
                        start: cmd_start,
                        end: cmd_start + word.len(),
                        scope: TerminalScope::Command,
                    });
                }
            }
        }
    }

    // Status badges, numbers, keywords, punctuation, time.
    let mut idx = 0;
    while idx < bytes.len() {
        let rest = &source[idx..];
        if let Some((len, scope)) = match_status_badge(rest) {
            tokens.push(TerminalSpan {
                start: idx,
                end: idx + len,
                scope,
            });
            idx += len;
            continue;
        }
        if let Some(len) = match_time(rest) {
            tokens.push(TerminalSpan {
                start: idx,
                end: idx + len,
                scope: TerminalScope::Time,
            });
            idx += len;
            continue;
        }
        if let Some(len) = match_number(rest) {
            tokens.push(TerminalSpan {
                start: idx,
                end: idx + len,
                scope: TerminalScope::Number,
            });
            idx += len;
            continue;
        }
        if let Some(len) = match_keyword(rest) {
            tokens.push(TerminalSpan {
                start: idx,
                end: idx + len,
                scope: TerminalScope::Keyword,
            });
            idx += len;
            continue;
        }
        let ch = bytes[idx];
        if matches!(ch, b'[' | b']' | b'{' | b'}' | b'(' | b')') {
            tokens.push(TerminalSpan {
                start: idx,
                end: idx + 1,
                scope: TerminalScope::Punctuation,
            });
            idx += 1;
            continue;
        }
        idx += 1;
    }

    // Sort + collapse overlaps (strings/commands win against later tokens at
    // the same position because we keep the first-pushed span).
    tokens.sort_by(|a, b| {
        if a.start != b.start {
            return a.start.cmp(&b.start);
        }
        b.end.cmp(&a.end)
    });
    let mut resolved: Vec<TerminalSpan> = Vec::with_capacity(tokens.len());
    let mut cursor: usize = 0;
    for token in tokens {
        if token.start >= cursor {
            cursor = token.end;
            resolved.push(token);
        }
    }
    resolved
}
