//! Pure UI configuration schema for `~/.config/jekko/ui.toml`.
//!
//! This module intentionally contains no filesystem, environment, clock, or TOML
//! loading logic. Higher crates own side effects and feed parsed values into
//! these serde-friendly types.

use serde::{Deserialize, Serialize};

macro_rules! pick {
    ($self:ident, $other:ident, $($field:ident),* $(,)?) => {
        $(
            if $other.$field.is_some() {
                $self.$field = $other.$field;
            }
        )*
    };
}

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

/// `[ui]` section values.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct UiSection {
    /// Theme identifier such as `codex-dark`.
    pub theme: Option<String>,
    /// Use an alternate-screen app surface.
    pub alternate_screen: Option<bool>,
    /// Enable mouse capture/handling.
    pub mouse: Option<bool>,
    /// Animation intensity.
    pub animations: Option<AnimationLevel>,
    /// Active redraw FPS.
    pub active_fps: Option<u16>,
    /// Idle redraw FPS.
    pub idle_fps: Option<u16>,
    /// Timer text update cadence in milliseconds.
    pub timer_tick_ms: Option<u64>,
    /// Extra rows rendered outside the visible timeline window.
    pub timeline_overscan: Option<u16>,
    /// Keep transcript pinned to the bottom while new output arrives.
    pub stick_to_bottom: Option<bool>,
    /// Enable transcript compaction support.
    pub compact_transcripts: Option<bool>,
    /// Number of latest transcript turns retained during compaction.
    pub max_compact_transcript_lines: Option<usize>,
    /// Show a scrollbar.
    pub show_scrollbar: Option<bool>,
    /// Soft-wrap transcript rows.
    pub soft_wrap: Option<bool>,
    /// Diff context lines.
    pub diff_context_lines: Option<usize>,
}

impl UiSection {
    /// Runtime defaults for `[ui]`.
    pub fn defaults() -> Self {
        Self {
            theme: Some("codex-dark".to_string()),
            alternate_screen: Some(true),
            mouse: Some(true),
            animations: Some(AnimationLevel::Full),
            active_fps: Some(30),
            idle_fps: Some(5),
            timer_tick_ms: Some(1000),
            timeline_overscan: Some(6),
            stick_to_bottom: Some(true),
            compact_transcripts: Some(true),
            max_compact_transcript_lines: Some(5),
            show_scrollbar: Some(false),
            soft_wrap: Some(true),
            diff_context_lines: Some(3),
        }
    }

    /// Merge section values, preferring values in `other`.
    pub fn merge(mut self, other: Self) -> Self {
        pick!(
            self,
            other,
            theme,
            alternate_screen,
            mouse,
            animations,
            active_fps,
            idle_fps,
            timer_tick_ms,
            timeline_overscan,
            stick_to_bottom,
            compact_transcripts,
            max_compact_transcript_lines,
            show_scrollbar,
            soft_wrap,
            diff_context_lines
        );
        self
    }
}

/// `[input]` section values.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct InputSection {
    /// Composer starts in single-line mode by default.
    pub single_line_default: Option<bool>,
    /// Prompt history entry limit.
    pub history_limit: Option<usize>,
    /// Enable `@file` completion.
    pub at_file_completion: Option<bool>,
}

impl InputSection {
    /// Runtime defaults for `[input]`.
    pub fn defaults() -> Self {
        Self {
            single_line_default: Some(true),
            history_limit: Some(500),
            at_file_completion: Some(true),
        }
    }

    /// Merge section values, preferring values in `other`.
    pub fn merge(mut self, other: Self) -> Self {
        pick!(
            self,
            other,
            single_line_default,
            history_limit,
            at_file_completion
        );
        self
    }
}

/// `[execution]` section values.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ExecutionSection {
    /// Prefer PTY execution for commands.
    pub prefer_pty: Option<bool>,
    /// Stream chunk latency in milliseconds.
    pub chunk_latency_ms: Option<u64>,
    /// Maximum stream chunk size in bytes.
    pub chunk_max_bytes: Option<usize>,
    /// Grace period before force-killing a process in milliseconds.
    pub kill_grace_ms: Option<u64>,
    /// Inherit the parent environment.
    pub inherit_env: Option<bool>,
    /// Force colored command output where possible.
    pub force_color: Option<bool>,
}

impl ExecutionSection {
    /// Runtime defaults for `[execution]`.
    pub fn defaults() -> Self {
        Self {
            prefer_pty: Some(true),
            chunk_latency_ms: Some(16),
            chunk_max_bytes: Some(8192),
            kill_grace_ms: Some(2000),
            inherit_env: Some(true),
            force_color: Some(true),
        }
    }

    /// Merge section values, preferring values in `other`.
    pub fn merge(mut self, other: Self) -> Self {
        pick!(
            self,
            other,
            prefer_pty,
            chunk_latency_ms,
            chunk_max_bytes,
            kill_grace_ms,
            inherit_env,
            force_color
        );
        self
    }
}

/// `[status]` section values.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct StatusSection {
    /// Show the active model.
    pub show_model: Option<bool>,
    /// Show the current working directory.
    pub show_pwd: Option<bool>,
    /// Show the current git branch.
    pub show_branch: Option<bool>,
    /// Show the active profile.
    pub show_profile: Option<bool>,
}

impl StatusSection {
    /// Runtime defaults for `[status]`.
    pub fn defaults() -> Self {
        Self {
            show_model: Some(true),
            show_pwd: Some(true),
            show_branch: Some(true),
            show_profile: Some(true),
        }
    }

    /// Merge section values, preferring values in `other`.
    pub fn merge(mut self, other: Self) -> Self {
        pick!(self, other, show_model, show_pwd, show_branch, show_profile);
        self
    }
}

/// `[accessibility]` section values.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AccessibilitySection {
    /// Disable nonessential motion.
    pub reduced_motion: Option<bool>,
    /// Respect `NO_COLOR`/`CLICOLOR`/related terminal color variables.
    pub respect_no_color: Option<bool>,
    /// Prefer higher contrast rendering.
    pub high_contrast: Option<bool>,
}

impl AccessibilitySection {
    /// Runtime defaults for `[accessibility]`.
    pub fn defaults() -> Self {
        Self {
            reduced_motion: Some(false),
            respect_no_color: Some(true),
            high_contrast: Some(false),
        }
    }

    /// Merge section values, preferring values in `other`.
    pub fn merge(mut self, other: Self) -> Self {
        pick!(self, other, reduced_motion, respect_no_color, high_contrast);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_tip8_contract() {
        let cfg = UiConfig::defaults();
        assert_eq!(cfg.ui.theme.as_deref(), Some("codex-dark"));
        assert_eq!(cfg.ui.animations, Some(AnimationLevel::Full));
        assert_eq!(cfg.ui.active_fps, Some(30));
        assert_eq!(cfg.input.history_limit, Some(500));
        assert_eq!(cfg.execution.chunk_max_bytes, Some(8192));
        assert_eq!(cfg.status.show_branch, Some(true));
        assert_eq!(cfg.accessibility.reduced_motion, Some(false));
    }

    #[test]
    fn empty_deserialize_is_overlay_not_defaults() {
        let cfg: UiConfig = serde_json::from_value(serde_json::json!({})).unwrap();
        assert_eq!(cfg, UiConfig::default());
        assert!(cfg.ui.theme.is_none());
    }

    #[test]
    fn deserialize_tip8_shape_from_raw_values() {
        let cfg: UiConfig = serde_json::from_value(serde_json::json!({
            "ui": {
                "theme": "codex-light",
                "animations": "subtle",
                "active_fps": 24,
                "soft_wrap": false
            },
            "input": {
                "history_limit": 42
            },
            "execution": {
                "prefer_pty": false,
                "chunk_max_bytes": 4096
            },
            "status": {
                "show_model": false
            },
            "accessibility": {
                "reduced_motion": true
            }
        }))
        .unwrap();

        assert_eq!(cfg.ui.theme.as_deref(), Some("codex-light"));
        assert_eq!(cfg.ui.animations, Some(AnimationLevel::Subtle));
        assert_eq!(cfg.ui.soft_wrap, Some(false));
        assert_eq!(cfg.input.history_limit, Some(42));
        assert_eq!(cfg.execution.prefer_pty, Some(false));
        assert_eq!(cfg.status.show_model, Some(false));
        assert_eq!(cfg.accessibility.reduced_motion, Some(true));
    }

    #[test]
    fn merge_prefers_overlay_values_and_preserves_missing_defaults() {
        let base = UiConfig::defaults();
        let overlay = UiConfig {
            ui: UiSection {
                active_fps: Some(12),
                show_scrollbar: Some(true),
                ..UiSection::default()
            },
            accessibility: AccessibilitySection {
                reduced_motion: Some(true),
                ..AccessibilitySection::default()
            },
            ..UiConfig::default()
        };

        let merged = base.merge(overlay);
        assert_eq!(merged.ui.theme.as_deref(), Some("codex-dark"));
        assert_eq!(merged.ui.active_fps, Some(12));
        assert_eq!(merged.ui.show_scrollbar, Some(true));
        assert_eq!(merged.accessibility.reduced_motion, Some(true));
        assert_eq!(merged.execution.prefer_pty, Some(true));
    }
}
