//! Pure-domain types for the Jekko Rust port.
//!
//! This crate is intentionally I/O-free: no filesystem, no network, no SQL,
//! and no clock access. Use the higher crates (`jekko-store`, `jekko-runtime`,
//! `jekko-tui`, …) for side-effecting work.
//!
//! Modules mirror the TypeScript layout in `packages/jekko/src/`:
//!
//! - [`session`] — session/message/prompt domain types and id newtypes.
//! - [`provider`] — provider/model catalog types.
//! - [`project`] — project identifier.
//! - [`permission`] — permission rules used by `jekko.json`.
//! - [`keybind`] — keybind chord parser + default action table.
//! - [`theme`] — theme JSON shape, color parser, and default presets.
//! - [`config`] — top-level `jekko.json` configuration shape + merge.
//! - [`v2`] — v2 (event-sourced) session schema.
//! - [`github`] — minimal GitHub-event payload parser (retained from before).
#![deny(rust_2018_idioms)]
#![warn(missing_docs)]

/// Defines a transparent newtype around `String` together with the conventional
/// constructor / accessor / conversion impls used by every identifier brand in
/// this crate (`Display`, `FromStr`, `AsRef<str>`, `From<$name> for String`).
///
/// Crate-private (no `#[macro_export]`): defined at crate root in `lib.rs`,
/// so any sibling module declared below can invoke `string_newtype!(...)`.
macro_rules! string_newtype {
    ($(#[$meta:meta])* $vis:vis $name:ident, $kind:expr) => {
        $(#[$meta])*
        #[derive(
            Debug,
            Clone,
            Hash,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            ::serde::Serialize,
            ::serde::Deserialize,
        )]
        #[serde(transparent)]
        $vis struct $name(String);

        impl $name {
            /// Identifier kind tag (matches the TypeScript brand).
            pub const KIND: &'static str = $kind;

            /// Construct from any string-like value. No validation is performed —
            /// callers that need strict validation should use [`Self::from_str`].
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            /// Borrow the underlying string slice.
            pub fn as_str(&self) -> &str {
                &self.0
            }

            /// Consume into the underlying [`String`].
            pub fn into_inner(self) -> String {
                self.0
            }
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl ::std::str::FromStr for $name {
            type Err = $crate::session::IdParseError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                if s.is_empty() {
                    return Err($crate::session::IdParseError {
                        kind: $kind,
                        value: s.to_string(),
                    });
                }
                Ok(Self(s.to_string()))
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}

pub mod config;
pub mod error;
pub mod github;
pub mod jankurai;
pub mod keybind;
pub mod permission;
pub mod project;
pub mod provider;
pub mod session;
pub mod theme;
pub mod v2;

pub use error::{CoreError, CoreResult};
