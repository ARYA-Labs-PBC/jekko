//! Unicode helpers used by the prompt widget.
//!
//! Replaces the JS-side string-width helper and ad-hoc grapheme handling with
//! `unicode-width` (display column width) and `unicode-segmentation` (grapheme
//! cluster offsets). Both crates are pinned with explicit versions in
//! `Cargo.toml` to keep the surface stable for the prompt layer.

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// Display width of a string in terminal columns.
///
/// Wide CJK characters and fullwidth glyphs report 2 columns; combining marks
/// and zero-width joiners report 0; emoji ZWJ sequences collapse to the width
/// of the first glyph (mirroring `unicode-width`'s behavior).
pub fn display_width(s: &str) -> usize {
    UnicodeWidthStr::width(s)
}

/// Byte offsets that mark the start of every grapheme cluster, plus the byte
/// length of the input as a sentinel.
///
/// Callers can move the cursor by grapheme by indexing into the returned
/// slice. For an empty input the result is `[0]`.
pub fn grapheme_offsets(s: &str) -> Vec<usize> {
    let mut offsets: Vec<usize> = s.grapheme_indices(true).map(|(idx, _)| idx).collect();
    offsets.push(s.len());
    offsets
}

/// Count of grapheme clusters in `s`.
pub fn grapheme_count(s: &str) -> usize {
    s.graphemes(true).count()
}

/// Truncate `s` to at most `max_cols` display columns and return the prefix.
///
/// Used by the paste-summary renderer and the right-aligned metadata strip.
pub fn truncate_to_width(s: &str, max_cols: usize) -> String {
    let mut out = String::with_capacity(s.len());
    let mut cols = 0usize;
    for g in s.graphemes(true) {
        let w = UnicodeWidthStr::width(g);
        if cols + w > max_cols {
            break;
        }
        out.push_str(g);
        cols += w;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_width_matches_len() {
        assert_eq!(display_width("hello"), 5);
    }

    #[test]
    fn cjk_is_wide() {
        assert_eq!(display_width("你好"), 4);
    }

    #[test]
    fn emoji_is_wide() {
        // Single emoji codepoint takes 2 columns.
        assert_eq!(display_width("\u{1F600}"), 2);
    }

    #[test]
    fn grapheme_offsets_include_sentinel() {
        let offsets = grapheme_offsets("ab");
        assert_eq!(offsets, vec![0, 1, 2]);
    }

    #[test]
    fn grapheme_count_handles_combining_mark() {
        // 'e' + combining acute accent should be one grapheme cluster.
        assert_eq!(grapheme_count("e\u{0301}"), 1);
    }

    #[test]
    fn truncate_respects_wide_chars() {
        // CJK char is 2 cols, truncating to 3 should drop the second.
        assert_eq!(truncate_to_width("你好x", 3), "你");
    }
}
