//! Span types for terminal tokens.

/// Coarse syntactic category for terminal output.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum TerminalScope {
    /// `PASS`/`OK`/`SUCCESS`/`✓`.
    Success,
    /// `FAIL`/`ERROR`/`ERR`/`✗`.
    Error,
    /// `WARN`/`WARNING`.
    Warning,
    /// `12ms`, `1.5s`, `[ 0.012s]`.
    Time,
    /// First word after a `$`/`#`/`>` prompt.
    Command,
    /// Single- or double-quoted string.
    StringLit,
    /// `[]`, `{}`, `()`.
    Punctuation,
    /// Numeric literal.
    Number,
    /// `true`/`false`/`yes`/`no`/`null`/`undefined`.
    Keyword,
    /// `$`/`#`/`>` shell prompt sigil.
    Prompt,
}

/// One byte range tagged with a scope.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct TerminalSpan {
    /// Inclusive byte start.
    pub start: usize,
    /// Exclusive byte end.
    pub end: usize,
    /// Scope tag.
    pub scope: TerminalScope,
}

pub(super) const MAX_TOKENIZE_LENGTH: usize = 10_000;
