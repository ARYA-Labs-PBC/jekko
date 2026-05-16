//! Small per-token matchers used during tokenisation.

use super::types::TerminalScope;

pub(super) fn scan_quoted(bytes: &[u8], start: usize, quote: u8) -> Option<usize> {
    let mut i = start + 1;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'\n' {
            return None;
        }
        if b == b'\\' && quote != b'\'' && i + 1 < bytes.len() {
            i += 2;
            continue;
        }
        if b == quote {
            return Some(i + 1);
        }
        i += 1;
    }
    None
}

pub(super) fn match_status_badge(rest: &str) -> Option<(usize, TerminalScope)> {
    let bytes = rest.as_bytes();
    // ✓ / ✗ glyphs
    if rest.starts_with('\u{2713}') {
        return Some(('\u{2713}'.len_utf8(), TerminalScope::Success));
    }
    if rest.starts_with('\u{2717}') {
        return Some(('\u{2717}'.len_utf8(), TerminalScope::Error));
    }
    // word-boundary scan for textual badges
    if !at_word_boundary(bytes) {
        return None;
    }
    let upper = rest
        .chars()
        .take_while(|c| c.is_ascii_uppercase())
        .collect::<String>();
    if upper.is_empty() {
        return None;
    }
    let len = upper.len();
    if !is_word_end(bytes, len) {
        return None;
    }
    let scope = match upper.as_str() {
        "PASS" | "OK" | "SUCCESS" => TerminalScope::Success,
        "FAIL" | "FAILED" | "ERROR" | "ERR" => TerminalScope::Error,
        "WARN" | "WARNING" => TerminalScope::Warning,
        _ => return None,
    };
    Some((len, scope))
}

pub(super) fn match_time(rest: &str) -> Option<usize> {
    let bytes = rest.as_bytes();
    if !at_word_boundary(bytes) && bytes.first() != Some(&b'[') {
        return None;
    }
    let mut idx = 0;
    if bytes.first() == Some(&b'[') {
        idx += 1;
        while idx < bytes.len() && bytes[idx] == b' ' {
            idx += 1;
        }
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
    let unit_start = idx;
    if rest[unit_start..].starts_with("ms") {
        idx += 2;
    } else if rest[unit_start..].starts_with('s') {
        idx += 1;
    } else {
        return None;
    }
    if bytes.first() == Some(&b'[') {
        while idx < bytes.len() && bytes[idx] == b' ' {
            idx += 1;
        }
        if bytes.get(idx) == Some(&b']') {
            idx += 1;
        }
    }
    Some(idx)
}

pub(super) fn match_number(rest: &str) -> Option<usize> {
    let bytes = rest.as_bytes();
    if !at_word_boundary(bytes) {
        return None;
    }
    let mut idx = 0;
    let mut dot_seen = false;
    let mut digit_seen = false;
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
    if !is_word_end(bytes, idx) {
        return None;
    }
    Some(idx)
}

pub(super) fn match_keyword(rest: &str) -> Option<usize> {
    if !at_word_boundary(rest.as_bytes()) {
        return None;
    }
    for keyword in ["true", "false", "null", "undefined", "yes", "no"] {
        if rest.starts_with(keyword) && is_word_end(rest.as_bytes(), keyword.len()) {
            return Some(keyword.len());
        }
    }
    None
}

pub(super) fn at_word_boundary(bytes: &[u8]) -> bool {
    if bytes.is_empty() {
        return false;
    }
    // Only meaningful when applied at a slice start — caller invariant.
    true
}

pub(super) fn is_word_end(bytes: &[u8], idx: usize) -> bool {
    match bytes.get(idx) {
        None => true,
        Some(b) => !b.is_ascii_alphanumeric() && *b != b'_',
    }
}
