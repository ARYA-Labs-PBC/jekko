//! Pure-domain error types shared across the jekko-core crate.
//!
//! Each module typically defines its own narrower error enum (e.g.
//! [`crate::keybind::KeybindParseError`], [`crate::theme::ThemeError`]).
//! This module aggregates a single top-level error suitable for callers
//! that want one error type per crate. It performs no I/O.
use thiserror::Error;

use crate::keybind::KeybindParseError;
use crate::theme::ThemeError;

/// Crate-wide error type. Variants embed module-specific errors so callers
/// can match on the underlying failure when needed.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CoreError {
    /// A keybind chord could not be parsed.
    #[error(transparent)]
    Keybind(#[from] KeybindParseError),
    /// A theme value could not be resolved.
    #[error(transparent)]
    Theme(#[from] ThemeError),
    /// An identifier-shaped string did not match the expected `prefix_*` shape.
    #[error("invalid identifier '{value}' for kind '{kind}'")]
    InvalidId {
        /// Identifier kind tag (e.g. `"session"`, `"message"`).
        kind: &'static str,
        /// Offending value.
        value: String,
    },
    /// A required field was missing while decoding a struct from JSON.
    #[error("missing field '{0}'")]
    MissingField(&'static str),
}

/// Crate-wide [`Result`] alias.
pub type CoreResult<T> = std::result::Result<T, CoreError>;
