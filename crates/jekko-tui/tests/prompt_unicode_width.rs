//! Verifies the prompt module's unicode-width handling against the corner
//! cases that bit the original JS implementation (emoji, CJK, combining marks,
//! ZWJ sequences, fullwidth glyphs).

use jekko_tui::prompt::unicode::{display_width, grapheme_count, grapheme_offsets};

#[test]
fn ascii_string_width_matches_length() {
    assert_eq!(display_width("hello world"), 11);
}

#[test]
fn cjk_chinese_is_double_width() {
    assert_eq!(display_width("你好"), 4);
}

#[test]
fn cjk_mixed_with_ascii() {
    assert_eq!(display_width("hi你好"), 6);
}

#[test]
fn combining_acute_has_width_one() {
    // Latin small e + combining acute accent = é, displayed in one column.
    let s = "e\u{0301}";
    assert_eq!(display_width(s), 1);
}

#[test]
fn zwj_emoji_sequence_renders_as_two_columns() {
    // Family of woman + ZWJ + man = a single emoji rendered in two cols.
    let zwj = "\u{1F469}\u{200D}\u{1F468}";
    // unicode-width treats this as the sum of widths of its components plus the
    // zero-width joiner — the renderer rounds it to the lead glyph's width on
    // most terminals; we only require it to be at least 2 cols.
    assert!(display_width(zwj) >= 2);
}

#[test]
fn fullwidth_latin_is_double_width() {
    // Fullwidth Latin capital A (U+FF21).
    assert_eq!(display_width("\u{FF21}"), 2);
}

#[test]
fn grapheme_count_counts_clusters_not_codepoints() {
    // 2 codepoints, 1 grapheme cluster.
    assert_eq!(grapheme_count("e\u{0301}"), 1);
    // ASCII matches char count.
    assert_eq!(grapheme_count("abc"), 3);
    // CJK characters are individual graphemes.
    assert_eq!(grapheme_count("你好"), 2);
}

#[test]
fn grapheme_offsets_include_byte_sentinel() {
    let s = "ab";
    let offsets = grapheme_offsets(s);
    assert_eq!(offsets, vec![0, 1, 2]);

    let s = "你好";
    let offsets = grapheme_offsets(s);
    // Each CJK char is 3 bytes in UTF-8.
    assert_eq!(offsets, vec![0, 3, 6]);
}

#[test]
fn grapheme_offsets_walk_a_zwj_sequence() {
    // Family ZWJ sequence — 5 codepoints, 1 grapheme cluster, several bytes.
    let s = "\u{1F469}\u{200D}\u{1F468}";
    let offsets = grapheme_offsets(s);
    // Sentinel is always the byte length, so the result has ≥ 2 entries.
    assert!(offsets.len() >= 2);
    assert_eq!(*offsets.last().unwrap(), s.len());
}

#[test]
fn empty_string_offsets_have_only_sentinel() {
    let offsets = grapheme_offsets("");
    assert_eq!(offsets, vec![0]);
    assert_eq!(grapheme_count(""), 0);
    assert_eq!(display_width(""), 0);
}
