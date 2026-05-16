//! Shared parity diff primitives.
//!
//! Several xtask commands (`cli-help-parity`, `tool-schema-parity`,
//! `session-fixture-parity`, `httpapi-parity`) take a "current" set of
//! identifiers and compare it against a "expected" set captured on
//! disk. The exact comparison shape varies — sometimes it's a line-by-line
//! string diff, sometimes a set diff — but the reporting + strict-mode
//! semantics are identical.

use std::collections::BTreeSet;

/// One pair of `(actual_only, expected_only)` deltas computed from a
/// set-style comparison.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SetDiff {
    /// Items present in `actual` but missing from `expected`.
    pub added: Vec<String>,
    /// Items present in `expected` but missing from `actual`.
    pub removed: Vec<String>,
}

impl SetDiff {
    /// Compute a set diff between two collections of names.
    pub fn compute<A, E>(actual: A, expected: E) -> Self
    where
        A: IntoIterator<Item = String>,
        E: IntoIterator<Item = String>,
    {
        let actual: BTreeSet<String> = actual.into_iter().collect();
        let expected: BTreeSet<String> = expected.into_iter().collect();
        let added: Vec<String> = actual.difference(&expected).cloned().collect();
        let removed: Vec<String> = expected.difference(&actual).cloned().collect();
        Self { added, removed }
    }

    /// Whether the two sets are identical.
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty()
    }
}

/// Compute a simple line-by-line diff. Lines that appear in `actual` but
/// not `expected` show up as `+`, lines that appear in `expected` but not
/// `actual` show up as `-`. Order is preserved from the inputs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LineDiff {
    /// `+` lines (in actual, not in expected).
    pub added: Vec<String>,
    /// `-` lines (in expected, not in actual).
    pub removed: Vec<String>,
}

impl LineDiff {
    /// Compute a multiset-style line diff. Lines present in both sides
    /// (one-for-one) are dropped; everything else appears as added or
    /// removed.
    pub fn compute(actual: &str, expected: &str) -> Self {
        let actual_lines: Vec<&str> = actual.lines().collect();
        let mut expected_remaining: Vec<&str> = expected.lines().collect();

        // Strip matching lines pairwise (preserve order on what's left).
        let mut added: Vec<String> = Vec::new();
        let mut removed: Vec<String> = Vec::new();

        for line in actual_lines {
            if let Some(pos) = expected_remaining.iter().position(|e| *e == line) {
                expected_remaining.remove(pos);
            } else {
                added.push(line.to_string());
            }
        }
        for line in expected_remaining {
            removed.push(line.to_string());
        }

        Self { added, removed }
    }

    /// Whether the two strings are identical (line-wise).
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_diff_finds_added_and_removed() {
        let diff = SetDiff::compute(
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
            vec!["b".to_string(), "c".to_string(), "d".to_string()],
        );
        assert_eq!(diff.added, vec!["a".to_string()]);
        assert_eq!(diff.removed, vec!["d".to_string()]);
        assert!(!diff.is_empty());
    }

    #[test]
    fn set_diff_is_empty_when_equal() {
        let diff = SetDiff::compute(
            vec!["x".to_string(), "y".to_string()],
            vec!["x".to_string(), "y".to_string()],
        );
        assert!(diff.is_empty());
    }

    #[test]
    fn line_diff_handles_pure_addition() {
        let diff = LineDiff::compute("alpha\nbeta\ngamma\n", "alpha\ngamma\n");
        assert_eq!(diff.added, vec!["beta".to_string()]);
        assert!(diff.removed.is_empty());
    }

    #[test]
    fn line_diff_handles_pure_removal() {
        let diff = LineDiff::compute("alpha\n", "alpha\nbeta\n");
        assert!(diff.added.is_empty());
        assert_eq!(diff.removed, vec!["beta".to_string()]);
    }

    #[test]
    fn line_diff_handles_substitution() {
        let diff = LineDiff::compute("alpha\nNEW\n", "alpha\nPREVIOUS\n");
        assert_eq!(diff.added, vec!["NEW".to_string()]);
        assert_eq!(diff.removed, vec!["PREVIOUS".to_string()]);
    }

    #[test]
    fn line_diff_is_empty_when_identical() {
        let diff = LineDiff::compute("a\nb\nc\n", "a\nb\nc\n");
        assert!(diff.is_empty());
    }
}
