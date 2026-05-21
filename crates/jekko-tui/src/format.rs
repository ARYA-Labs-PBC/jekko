//! Shared formatting helpers (COWBOY.md T1-V8, per tips/fucktui/tip9.txt §5).
//!
//! Hosts compact number/token formatters used by the multi-agent rail, the
//! footer status row, and future Tier 1 widgets (permission banner, working
//! strip). Centralising the algorithm here keeps `↑ 12.3k · ↓ 170.9k` style
//! suffixes consistent across panels and avoids the per-call drift that
//! crept in while each surface rolled its own `format!` block.

use crate::agents::TokenStats;
use crate::glyph_set;

/// Compact token-count formatter per tip9 §5.
///
/// Rules:
/// * `n < 1_000` → plain integer (`"42"`).
/// * `1_000 ≤ n < 1_000_000` → one decimal `k` (`"12.3k"`).
/// * `n ≥ 1_000_000` → one decimal `m` (`"1.7m"`).
///
/// We use lowercase `m` (matching tip9 spec lines 387–397) rather than the
/// uppercase `M` the local `agents/panel.rs` shim used before; the panel
/// switches over to this helper, so the casing now agrees with the spec.
pub fn format_tokens(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}m", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

/// Render directional token stats as `"↑ <out> · ↓ <in>"`.
///
/// Drops a side whose count is `0`. When both sides are zero the result is
/// an empty string so callers can detect the "no token activity yet" case
/// without re-checking the struct (e.g. to skip the surrounding separator).
///
/// Per tip1.txt lines 1330–1334: `↑` = output / generation, `↓` = input /
/// consumption. The arrow + space ordering mirrors what the agent rail
/// renders today so a side-by-side audit looks identical.
///
/// T-A11Y-MIGRATION: the arrow glyphs honor the active `GlyphMode` so
/// `JEKKO_ASCII=1` / `LC_ALL=C` swaps them to ASCII (`^` / `v`).
pub fn format_tokens_with_direction(stats: &TokenStats) -> String {
    let glyphs = glyph_set::current();
    let up = glyphs.arrow_up;
    let down = glyphs.arrow_down;
    match (stats.output, stats.input) {
        (0, 0) => String::new(),
        (0, input) => format!("{down} {}", format_tokens(input)),
        (output, 0) => format!("{up} {}", format_tokens(output)),
        (output, input) => format!(
            "{up} {} · {down} {}",
            format_tokens(output),
            format_tokens(input)
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // WHY: parameterised boundary table — every threshold across the three
    // branches (sub-k, sub-m, super-m) plus the historically tricky
    // 999_999 boundary that rounds up into "1000.0k".
    #[test]
    fn format_tokens_boundary_table() {
        let cases: &[(u64, &str)] = &[
            (0, "0"),
            (1, "1"),
            (42, "42"),
            (999, "999"),
            (1_000, "1.0k"),
            (1_500, "1.5k"),
            (12_300, "12.3k"),
            (170_900, "170.9k"),
            (999_999, "1000.0k"),
            (1_000_000, "1.0m"),
            (1_500_000, "1.5m"),
            (1_700_000, "1.7m"),
        ];
        for (input, expected) in cases {
            assert_eq!(
                format_tokens(*input),
                *expected,
                "format_tokens({input}) should produce {expected}",
            );
        }
    }

    #[test]
    fn format_tokens_does_not_underflow_or_panic_for_max() {
        // Largest u64 still goes through the `m` arm without panic.
        let s = format_tokens(u64::MAX);
        assert!(s.ends_with('m'), "expected m suffix, got {s}");
    }

    #[test]
    fn direction_both_sides_present() {
        let stats = TokenStats {
            input: 170_900,
            output: 12_300,
        };
        assert_eq!(format_tokens_with_direction(&stats), "↑ 12.3k · ↓ 170.9k");
    }

    #[test]
    fn direction_output_only() {
        let stats = TokenStats {
            input: 0,
            output: 130_800,
        };
        assert_eq!(format_tokens_with_direction(&stats), "↑ 130.8k");
    }

    #[test]
    fn direction_input_only() {
        let stats = TokenStats {
            input: 8_400,
            output: 0,
        };
        assert_eq!(format_tokens_with_direction(&stats), "↓ 8.4k");
    }

    #[test]
    fn direction_neither_is_empty() {
        let stats = TokenStats::default();
        assert!(format_tokens_with_direction(&stats).is_empty());
    }
}
