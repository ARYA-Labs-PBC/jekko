//! Keybind parsing and the default key-binding table.
//!
//! Ported from `packages/jekko/src/util/keybind.ts` (parser/serializer) and
//! `packages/jekko/src/config/keybinds.ts` (default action -> chord table).
//!
//! A "chord" describes one combo of modifiers + a base key (e.g.
//! `ctrl+shift+t`). A "sequence" is a comma-separated list of chords
//! that all map to the same action. The TS side accepts both forms.
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

mod chord;
mod defaults;
mod set;

pub use chord::Chord;
pub use defaults::default_bindings;
pub use set::ChordSet;

/// Errors returned when parsing a chord.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum KeybindParseError {
    /// The chord was empty (e.g. `""` or only modifiers with no base key).
    #[error("empty chord")]
    Empty,
    /// The chord referenced an unknown modifier (very rare with the relaxed
    /// parser, but reserved for future strict mode).
    #[error("unknown modifier '{0}'")]
    UnknownModifier(String),
}

/// Strongly-typed action name (e.g. `"engage"`, `"session_new"`).
pub type ActionName = &'static str;

/// Resolved (parsed) keybinds table, mapping each action to a [`ChordSet`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeybindsTable {
    /// Underlying table.
    pub entries: BTreeMap<String, ChordSet>,
}

impl KeybindsTable {
    /// Build the resolved table from [`default_bindings`].
    pub fn defaults() -> Result<Self, KeybindParseError> {
        let mut entries = BTreeMap::new();
        for (name, binding) in default_bindings() {
            entries.insert(name.to_string(), ChordSet::parse(binding)?);
        }
        Ok(Self { entries })
    }

    /// Look up the parsed binding for an action.
    pub fn get(&self, action: &str) -> Option<&ChordSet> {
        self.entries.get(action)
    }
}

#[cfg(test)]
mod tests;
