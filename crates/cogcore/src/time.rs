//! Deterministic time helpers.
//!
//! The cogcore hot path never reads `std::time::SystemTime::now()`. The
//! benchmark and embedding hosts pin the clock via `BENCH_NOW`; ISO-8601
//! strings are compared lexicographically (works because we use the fully
//! padded `YYYY-MM-DDThh:mm:ssZ` form everywhere).

/// Canonical "now" used by deterministic runs. Aligns with
/// `memory_benchmark::memory_api::BENCH_NOW`.
pub const BENCH_NOW: &str = "2026-05-12T00:00:00Z";

/// Lexicographic less-than on ISO-8601 strings.
#[inline]
pub fn iso_lt(a: &str, b: &str) -> bool {
    a < b
}

/// Lexicographic less-or-equal on ISO-8601 strings.
#[inline]
pub fn iso_le(a: &str, b: &str) -> bool {
    a <= b
}

/// Lexicographic greater-or-equal on ISO-8601 strings.
#[inline]
pub fn iso_ge(a: &str, b: &str) -> bool {
    a >= b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso_lex_matches_chrono() {
        assert!(iso_lt("2024-01-01T00:00:00Z", "2024-01-02T00:00:00Z"));
        assert!(iso_lt("2024-01-01T00:00:00Z", "2025-01-01T00:00:00Z"));
        assert!(iso_ge(BENCH_NOW, "2024-01-01T00:00:00Z"));
    }
}
