//! [`ResolvedTheme`] — map of token name to resolved [`Color`].

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::color::Color;

/// Map of token name -> resolved [`Color`].
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ResolvedTheme {
    /// Token map.
    pub tokens: BTreeMap<String, Color>,
}

impl ResolvedTheme {
    /// Look up a single token.
    pub fn get(&self, name: &str) -> Option<Color> {
        self.tokens.get(name).copied()
    }
}
