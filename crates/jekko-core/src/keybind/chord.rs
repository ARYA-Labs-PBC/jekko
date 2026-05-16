use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use super::KeybindParseError;

/// A single keybind chord: modifiers + a base key name.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Chord {
    /// Base key name (e.g. `"p"`, `"escape"`, `"tab"`). Empty string when the
    /// chord is `<leader>` alone (rare).
    pub name: String,
    /// Whether `ctrl` is held.
    pub ctrl: bool,
    /// Whether `alt`/`meta`/`option` is held.
    pub meta: bool,
    /// Whether `shift` is held.
    pub shift: bool,
    /// Whether `super` (a.k.a. `cmd`) is held.
    pub super_: bool,
    /// Whether the chord follows the configured leader key.
    pub leader: bool,
}

impl Chord {
    /// Parse a chord like `"ctrl+shift+t"` or `"<leader>q"`.
    ///
    /// Mirrors the TypeScript parser in `util/keybind.ts`. The parser is
    /// lenient — unknown segments are treated as the base key name (last one wins).
    pub fn parse(input: &str) -> Result<Self, KeybindParseError> {
        if input.is_empty() {
            return Err(KeybindParseError::Empty);
        }
        let normalized = input.replace("<leader>", "leader+");
        let lowered = normalized.to_ascii_lowercase();
        let mut chord = Chord::default();
        for part in lowered.split('+') {
            if part.is_empty() {
                continue;
            }
            match part {
                "ctrl" => chord.ctrl = true,
                "alt" | "meta" | "option" => chord.meta = true,
                "super" | "cmd" => chord.super_ = true,
                "shift" => chord.shift = true,
                "leader" => chord.leader = true,
                "esc" => chord.name = "escape".to_string(),
                other => chord.name = other.to_string(),
            }
        }
        if !chord.leader
            && chord.name.is_empty()
            && !chord.ctrl
            && !chord.meta
            && !chord.shift
            && !chord.super_
        {
            return Err(KeybindParseError::Empty);
        }
        Ok(chord)
    }

    /// Render the chord in the canonical `ctrl+shift+name` order (matching the
    /// TypeScript `toString`).
    pub fn to_string_canonical(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if self.ctrl {
            parts.push("ctrl".to_string());
        }
        if self.meta {
            parts.push("alt".to_string());
        }
        if self.super_ {
            parts.push("super".to_string());
        }
        if self.shift {
            parts.push("shift".to_string());
        }
        if !self.name.is_empty() {
            let name = if self.name == "delete" {
                "del".to_string()
            } else {
                self.name.clone()
            };
            parts.push(name);
        }
        let core = parts.join("+");
        match (self.leader, core.is_empty()) {
            (true, true) => "<leader>".to_string(),
            (true, false) => format!("<leader>+{core}"),
            (false, _) => core,
        }
    }
}

impl fmt::Display for Chord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_string_canonical())
    }
}

impl FromStr for Chord {
    type Err = KeybindParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}
