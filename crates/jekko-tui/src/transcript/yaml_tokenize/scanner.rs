//! Per-line scan loops for the YAML-ish lexer.
//!
//! [`tokenize_yaml_code`] handles ordinary YAML lines and [`tokenize_block_line`]
//! handles lines that fall inside a block scalar opened earlier in the input.

use super::recognizers::{
    is_sequence_marker, match_scalar_token, match_upper_word, match_word, next_block_boundary,
    scan_quoted, skip_spaces,
};
use super::tokens::{push, YamlScope, YamlSpan, OPERATOR, PUNCTUATION};

/// Scan one regular YAML line, emitting spans into `tokens`.
pub(super) fn tokenize_yaml_code(code: &str, base_offset: usize, tokens: &mut Vec<YamlSpan>) {
    let bytes = code.as_bytes();
    let mut index: usize = 0;
    while index < bytes.len() {
        let ch = bytes[index];
        if ch == b' ' || ch == b'\t' {
            index += 1;
            continue;
        }
        if ch == b'-' && is_sequence_marker(bytes, index) {
            push(
                tokens,
                base_offset + index,
                base_offset + index + 1,
                YamlScope::Sequence,
            );
            index += 1;
            continue;
        }
        if ch == b'"' || ch == b'\'' || ch == b'`' {
            let end = scan_quoted(bytes, index, ch);
            push(
                tokens,
                base_offset + index,
                base_offset + end,
                YamlScope::StringLit,
            );
            index = end;
            continue;
        }
        if PUNCTUATION.contains(&ch) {
            push(
                tokens,
                base_offset + index,
                base_offset + index + 1,
                YamlScope::Punctuation,
            );
            index += 1;
            continue;
        }
        if OPERATOR.contains(&ch) {
            push(
                tokens,
                base_offset + index,
                base_offset + index + 1,
                YamlScope::Operator,
            );
            index += 1;
            continue;
        }
        if let Some(scalar) = match_scalar_token(&code[index..], base_offset + index) {
            let len = scalar.end - scalar.start;
            tokens.push(scalar);
            index += len;
            continue;
        }
        if let Some(word_len) = match_word(&code[index..]) {
            let start = index;
            let end = index + word_len;
            let after = skip_spaces(bytes, end);
            if bytes.get(after) == Some(&b':') {
                push(
                    tokens,
                    base_offset + start,
                    base_offset + end,
                    YamlScope::Property,
                );
                push(
                    tokens,
                    base_offset + after,
                    base_offset + after + 1,
                    YamlScope::Punctuation,
                );
                index = after + 1;
            } else {
                push(
                    tokens,
                    base_offset + start,
                    base_offset + end,
                    YamlScope::Literal,
                );
                index = end;
            }
            continue;
        }
        push(
            tokens,
            base_offset + index,
            base_offset + index + 1,
            YamlScope::Literal,
        );
        index += 1;
    }
}

/// Scan one line that sits inside an active block scalar.
pub(super) fn tokenize_block_line(line: &str, base_offset: usize, tokens: &mut Vec<YamlSpan>) {
    let bytes = line.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        let ch = bytes[index];
        if ch == b' ' || ch == b'\t' {
            index += 1;
            continue;
        }
        if ch == b'-' && is_sequence_marker(bytes, index) {
            push(
                tokens,
                base_offset + index,
                base_offset + index + 1,
                YamlScope::Sequence,
            );
            index += 1;
            continue;
        }
        if ch == b'"' || ch == b'\'' || ch == b'`' {
            let end = scan_quoted(bytes, index, ch);
            push(
                tokens,
                base_offset + index,
                base_offset + end,
                YamlScope::StringLit,
            );
            index = end;
            continue;
        }
        if PUNCTUATION.contains(&ch) {
            push(
                tokens,
                base_offset + index,
                base_offset + index + 1,
                YamlScope::Punctuation,
            );
            index += 1;
            continue;
        }
        if let Some(scalar) = match_scalar_token(&line[index..], base_offset + index) {
            let len = scalar.end - scalar.start;
            tokens.push(scalar);
            index += len;
            continue;
        }
        if let Some(upper) = match_upper_word(&line[index..]) {
            push(
                tokens,
                base_offset + index,
                base_offset + index + upper,
                YamlScope::Operator,
            );
            index += upper;
            continue;
        }
        let next = next_block_boundary(line, index + 1);
        push(
            tokens,
            base_offset + index,
            base_offset + next,
            YamlScope::Block,
        );
        index = next;
    }
}
