use std::str::FromStr;

use serde::{Deserialize, Serialize};

use super::{Chord, KeybindParseError};

/// A sequence of comma-separated chords (e.g. `"pageup,ctrl+alt+b"`).
///
/// Bindings of the literal string `"none"` resolve to an empty sequence.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ChordSet(pub Vec<Chord>);

impl ChordSet {
    /// Parse a comma-separated set of chords. `"none"` yields an empty set.
    pub fn parse(input: &str) -> Result<Self, KeybindParseError> {
        if input == "none" {
            return Ok(Self(Vec::new()));
        }
        let mut out = Vec::new();
        for combo in input.split(',') {
            let trimmed = combo.trim();
            if trimmed.is_empty() {
                continue;
            }
            out.push(Chord::parse(trimmed)?);
        }
        Ok(Self(out))
    }

    /// Render back to canonical text. Empty sets render as `"none"`.
    pub fn to_string_canonical(&self) -> String {
        if self.0.is_empty() {
            return "none".to_string();
        }
        self.0
            .iter()
            .map(Chord::to_string_canonical)
            .collect::<Vec<_>>()
            .join(",")
    }

    /// Iterate over the chords in declaration order.
    pub fn iter(&self) -> std::slice::Iter<'_, Chord> {
        self.0.iter()
    }

    /// Number of chords in the set.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the set is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl FromStr for ChordSet {
    type Err = KeybindParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}
