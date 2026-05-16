//! Per-pattern recognizers for the YAML-ish lexer.
//!
//! Each function inspects a byte slice or string slice and reports how far it
//! consumes. They never mutate caller state and never panic on partial input.

use super::tokens::{YamlScope, YamlSpan, PUNCTUATION};

/// Match a bare boolean keyword or `~`, returning its length when present.
pub(super) fn match_scalar_token(text: &str, base_offset: usize) -> Option<YamlSpan> {
    if let Some(len) = match_bare_boolean(text) {
        return Some(YamlSpan {
            start: base_offset,
            end: base_offset + len,
            scope: YamlScope::Boolean,
        });
    }
    if let Some(len) = match_bare_number(text) {
        return Some(YamlSpan {
            start: base_offset,
            end: base_offset + len,
            scope: YamlScope::Number,
        });
    }
    None
}

/// Return the length of a bare boolean keyword (`true`, `false`, `null`,
/// `yes`, `no`, `~`) when one starts at the beginning of `text`.
pub(super) fn match_bare_boolean(text: &str) -> Option<usize> {
    let lower = text
        .chars()
        .take(5)
        .collect::<String>()
        .to_ascii_lowercase();
    for keyword in ["false", "true", "null", "yes", "no", "~"] {
        let len = keyword.len();
        if lower.len() < len {
            continue;
        }
        if &lower[..len] == keyword && is_word_end(text.as_bytes(), len) {
            return Some(len);
        }
    }
    None
}

/// Return the length of a bare numeric literal at the start of `text`.
///
/// Accepts an optional leading `-`, requires at least one digit, allows one
/// `.` after a digit, and consumes a trailing alphabetic unit (`s`, `ms`,
/// `%`, …).
pub(super) fn match_bare_number(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut idx = 0;
    if bytes.first() == Some(&b'-') {
        idx += 1;
    }
    let mut digit_seen = false;
    let mut dot_seen = false;
    while idx < bytes.len() {
        let b = bytes[idx];
        if b.is_ascii_digit() {
            digit_seen = true;
            idx += 1;
            continue;
        }
        if b == b'.' && !dot_seen && digit_seen {
            dot_seen = true;
            idx += 1;
            continue;
        }
        break;
    }
    if !digit_seen {
        return None;
    }
    while idx < bytes.len() {
        let b = bytes[idx];
        if b.is_ascii_alphabetic() || b == b'%' {
            idx += 1;
            continue;
        }
        break;
    }
    if !is_word_end(bytes, idx) {
        return None;
    }
    Some(idx)
}

/// Match an identifier-shaped word, returning its byte length when present.
pub(super) fn match_word(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let first = bytes.first()?;
    if !(first.is_ascii_alphabetic() || *first == b'_' || *first == b'$') {
        return None;
    }
    let mut idx = 1;
    while idx < bytes.len() {
        let b = bytes[idx];
        if b.is_ascii_alphanumeric()
            || b == b'_'
            || b == b'$'
            || b == b'.'
            || b == b'/'
            || b == b'*'
            || b == b'-'
        {
            idx += 1;
            continue;
        }
        break;
    }
    Some(idx)
}

/// Match an upper-case operator word (`AND`, `OR/NOT`, …) for block scalars.
pub(super) fn match_upper_word(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let first = bytes.first()?;
    if !first.is_ascii_uppercase() {
        return None;
    }
    let mut idx = 1;
    while idx < bytes.len() {
        let b = bytes[idx];
        if b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_' || b == b'/' || b == b'-' {
            idx += 1;
            continue;
        }
        break;
    }
    if idx < 2 {
        return None;
    }
    if !is_word_end(bytes, idx) {
        return None;
    }
    Some(idx)
}

/// Report whether the code portion of a line ends with `:` then `|` or `>`,
/// optionally followed by chomp indicators. Used to enter block-scalar mode.
pub(super) fn line_starts_block_scalar(code: &str) -> bool {
    // `:\s*[|>][+-]?\s*$`
    let trimmed = code.trim_end();
    let mut idx = trimmed.len();
    let bytes = trimmed.as_bytes();
    while idx > 0 && (bytes[idx - 1] == b'+' || bytes[idx - 1] == b'-') {
        idx -= 1;
    }
    if idx == 0 {
        return false;
    }
    let last = bytes[idx - 1];
    if last != b'|' && last != b'>' {
        return false;
    }
    idx -= 1;
    while idx > 0 && (bytes[idx - 1] == b' ' || bytes[idx - 1] == b'\t') {
        idx -= 1;
    }
    if idx == 0 {
        return false;
    }
    bytes[idx - 1] == b':'
}

/// Count the run of leading space or tab bytes.
pub(super) fn indentation(line: &str) -> usize {
    let mut count = 0;
    for b in line.bytes() {
        if b == b' ' || b == b'\t' {
            count += 1;
        } else {
            break;
        }
    }
    count
}

/// Return `true` when `-` at `index` introduces a YAML sequence entry
/// (whitespace on both sides, or start-of-line then whitespace).
pub(super) fn is_sequence_marker(bytes: &[u8], index: usize) -> bool {
    let before = if index == 0 {
        None
    } else {
        Some(bytes[index - 1])
    };
    let after = bytes.get(index + 1).copied();
    let before_ok = matches!(before, None | Some(b' ') | Some(b'\t'));
    let after_ok = matches!(after, Some(b' ') | Some(b'\t'));
    before_ok && after_ok
}

/// Scan forward over a quoted string starting at `start`, returning the byte
/// offset just past the closing quote (or `bytes.len()` when unterminated).
pub(super) fn scan_quoted(bytes: &[u8], start: usize, quote: u8) -> usize {
    let mut index = start + 1;
    while index < bytes.len() {
        let ch = bytes[index];
        if ch == b'\\' && quote != b'\'' && index + 1 < bytes.len() {
            index += 2;
            continue;
        }
        if ch == quote {
            if quote == b'\'' && bytes.get(index + 1) == Some(&b'\'') {
                index += 2;
                continue;
            }
            return index + 1;
        }
        index += 1;
    }
    bytes.len()
}

/// Find the next index inside a block-scalar payload where tokenization must
/// re-classify (quote, punctuation, scalar keyword, upper-case operator).
pub(super) fn next_block_boundary(line: &str, start: usize) -> usize {
    let bytes = line.as_bytes();
    let mut idx = start;
    while idx < bytes.len() {
        let b = bytes[idx];
        if b == b'"' || b == b'\'' || b == b'`' {
            return idx;
        }
        if PUNCTUATION.contains(&b) {
            return idx;
        }
        let rest = &line[idx..];
        if match_bare_boolean(rest).is_some()
            || match_bare_number(rest).is_some()
            || match_upper_word(rest).is_some()
        {
            return idx;
        }
        idx += 1;
    }
    line.len()
}

/// Locate the start of an end-of-line `#` comment, skipping `#` inside quoted
/// strings. Returns `None` when the line has no comment.
pub(super) fn find_comment_start(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut in_single = false;
    let mut in_double = false;
    let mut i = 0;
    while i < bytes.len() {
        let ch = bytes[i];
        if ch == b'\\' && (in_single || in_double) {
            i += 2;
            continue;
        }
        if ch == b'"' && !in_single {
            in_double = !in_double;
        } else if ch == b'\'' && !in_double {
            in_single = !in_single;
        } else if ch == b'#'
            && !in_single
            && !in_double
            && (i == 0 || bytes[i - 1] == b' ' || bytes[i - 1] == b'\t')
        {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Detect ZYAL sentinel lines that the prompt pane highlights specially.
pub(super) fn is_sentinel(line: &str) -> bool {
    let trimmed = line.trim_start();
    if trimmed.starts_with("<<<ZYAL") && trimmed.trim_end().ends_with(">>>") {
        return true;
    }
    if trimmed.starts_with("<<<END_ZYAL") && trimmed.trim_end().ends_with(">>>") {
        return true;
    }
    if let Some(rest) = trimmed.strip_prefix("ZYAL_ARM") {
        if rest.is_empty()
            || rest.starts_with(' ')
            || rest.starts_with('\t')
            || rest.starts_with('_')
        {
            return true;
        }
    }
    false
}

/// Advance past any run of spaces or tabs, returning the next index.
pub(super) fn skip_spaces(bytes: &[u8], mut index: usize) -> usize {
    while index < bytes.len() && (bytes[index] == b' ' || bytes[index] == b'\t') {
        index += 1;
    }
    index
}

/// Report whether `idx` lies at the end of a word boundary in `bytes`.
pub(super) fn is_word_end(bytes: &[u8], idx: usize) -> bool {
    match bytes.get(idx) {
        None => true,
        Some(b) => !b.is_ascii_alphanumeric() && *b != b'_',
    }
}
