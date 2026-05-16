//! Token data structures and span-building helpers.
//!
//! Holds the public [`YamlScope`] enum and [`YamlSpan`] record returned by the
//! YAML-ish lexer, plus a small handful of internal helpers shared across the
//! scanner and recognizer modules.

/// Coarse syntactic category.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum YamlScope {
    /// Property key.
    Property,
    /// Quoted string.
    StringLit,
    /// `true`/`false`/`yes`/`no`/`null`/`~`.
    Boolean,
    /// Numeric literal.
    Number,
    /// `# comment to end of line`.
    Comment,
    /// `<<<ZYAL …>>>`, `<<<END_ZYAL …>>>`, `ZYAL_ARM …`.
    Sentinel,
    /// Bare scalar fragment in plain context.
    Literal,
    /// `{}[]:,`.
    Punctuation,
    /// `-` sequence marker.
    Sequence,
    /// Block-scalar payload character.
    Block,
    /// `|`/`>` block-scalar / fold operator, or upper-case operator words
    /// inside block payloads.
    Operator,
}

/// One byte range tagged with a scope.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct YamlSpan {
    /// Inclusive byte start.
    pub start: usize,
    /// Exclusive byte end.
    pub end: usize,
    /// Scope tag.
    pub scope: YamlScope,
}

/// Punctuation bytes that always close the current token.
pub(super) const PUNCTUATION: [u8; 6] = [b'{', b'}', b'[', b']', b':', b','];

/// Operator bytes that introduce block scalars or folds.
pub(super) const OPERATOR: [u8; 2] = [b'|', b'>'];

/// Append a span only when it covers at least one byte.
pub(super) fn push(tokens: &mut Vec<YamlSpan>, start: usize, end: usize, scope: YamlScope) {
    if end <= start {
        return;
    }
    tokens.push(YamlSpan { start, end, scope });
}

/// Trait providing a named negation so the main scan loop reads naturally.
pub(super) trait NotBool {
    fn not_bool(self) -> bool;
}

impl NotBool for bool {
    fn not_bool(self) -> bool {
        !self
    }
}
