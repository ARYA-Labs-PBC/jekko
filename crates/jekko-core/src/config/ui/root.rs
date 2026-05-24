use serde::{Deserialize, Serialize};

use super::sections::{
    AccessibilitySection, ExecutionSection, InputSection, StatusSection, UiSection,
};

/// Top-level UI TOML configuration.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    /// `[ui]` section.
    pub ui: UiSection,
    /// `[input]` section.
    pub input: InputSection,
    /// `[execution]` section.
    pub execution: ExecutionSection,
    /// `[status]` section.
    pub status: StatusSection,
    /// `[accessibility]` section.
    pub accessibility: AccessibilitySection,
}

impl UiConfig {
    /// Runtime defaults from `tips/fucktui/tip8.txt` section 22.
    pub fn defaults() -> Self {
        Self {
            ui: UiSection::defaults(),
            input: InputSection::defaults(),
            execution: ExecutionSection::defaults(),
            status: StatusSection::defaults(),
            accessibility: AccessibilitySection::defaults(),
        }
    }

    /// Merge `other` into `self`, preferring explicitly-set values in `other`.
    pub fn merge(self, other: Self) -> Self {
        Self {
            ui: self.ui.merge(other.ui),
            input: self.input.merge(other.input),
            execution: self.execution.merge(other.execution),
            status: self.status.merge(other.status),
            accessibility: self.accessibility.merge(other.accessibility),
        }
    }
}
