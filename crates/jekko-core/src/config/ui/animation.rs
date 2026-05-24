use serde::{Deserialize, Serialize};

/// Animation intensity for terminal UI motion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AnimationLevel {
    /// Disable nonessential motion.
    Off,
    /// Keep low-frequency motion only.
    Subtle,
    /// Enable full motion budget.
    Full,
}
