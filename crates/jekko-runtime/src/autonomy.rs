//! Autonomy boundaries loader.
//!
//! Reads the `[autonomy]` block from `agent/boundaries.toml` at runtime
//! init and exposes the limits as a typed [`AutonomyConfig`]. The block is
//! the cross-repo lockstep surface defined in
//! `ARYA-Labs-PBC/QantmOrchstrtr-RSI` ARY-2292 (E4a). Runtime enforcement
//! (Safety Kernel `autonomous_action.*` policy type and the policy-sidecar
//! session budget) lives in ARY-2293 / ARY-2294 on the QO side; this
//! module is the Jekko-side typed view of the same configuration so the
//! agent runtime can refuse prohibited actions before they leave the
//! process.
//!
//! Scope (ARY-2303): module + tests + one representative wiring point in
//! [`crate::Runtime::new`]. Full gating of every agent-initiated decision
//! surface is intentionally a follow-up — see the ticket.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use thiserror::Error;

/// Default ceiling on the number of actions an autonomous agent may take
/// in a single session before a user-visible checkpoint is required.
///
/// Mirrors the QO ARY-2300 T10 default. Applied when the configured value
/// is missing, zero, or the file cannot be parsed.
pub const DEFAULT_MAX_ACTIONS_BEFORE_CHECKPOINT: u32 = 20;

/// Default checkpoint interval in minutes.
///
/// Mirrors the QO ARY-2300 T10 default. Applied when the configured
/// value is missing, zero, or the file cannot be parsed.
pub const DEFAULT_SESSION_CHECKPOINT_INTERVAL_MINUTES: u32 = 30;

/// Canonical relative path to the boundaries file. Resolved against the
/// process current working directory by [`AutonomyConfig::load_default`].
pub const DEFAULT_BOUNDARIES_PATH: &str = "agent/boundaries.toml";

/// Typed view of the `[autonomy]` block.
///
/// Fields mirror the four keys from the canonical `agent/boundaries.toml`
/// shipped at the root of this repo. Construction is via
/// [`Self::load_from_path`] / [`Self::load_default`] — direct field
/// construction is intentionally allowed for tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutonomyConfig {
    /// Action labels the agent MUST NOT initiate without explicit
    /// per-session user instruction. Match the strings in
    /// `[autonomy].prohibited_autonomous_actions` exactly.
    pub prohibited_autonomous_actions: Vec<String>,
    /// Action labels that require an explicit per-call user confirmation
    /// even when the surrounding session has been pre-authorized.
    pub require_explicit_confirmation_for: Vec<String>,
    /// Minutes between mandatory session checkpoints.
    pub session_checkpoint_interval_minutes: u32,
    /// Maximum number of actions before a checkpoint is forced.
    pub max_actions_before_checkpoint: u32,
}

impl AutonomyConfig {
    /// Construct a config populated entirely from safe defaults. Useful
    /// as a fallback when the on-disk file is missing or malformed.
    pub fn defaults() -> Self {
        Self {
            prohibited_autonomous_actions: Vec::new(),
            require_explicit_confirmation_for: Vec::new(),
            session_checkpoint_interval_minutes:
                DEFAULT_SESSION_CHECKPOINT_INTERVAL_MINUTES,
            max_actions_before_checkpoint:
                DEFAULT_MAX_ACTIONS_BEFORE_CHECKPOINT,
        }
    }

    /// Load the `[autonomy]` block from a TOML file at `path`.
    ///
    /// Behavior:
    /// - Missing file → returns [`Self::defaults`] (NOT an error). Callers
    ///   that need to distinguish "file absent" from "file present but
    ///   malformed" should stat the path themselves first.
    /// - File present but `[autonomy]` section absent → defaults.
    /// - Integer limits absent or set to `0` → clamped to the module
    ///   defaults (`20` actions / `30` minutes).
    /// - Any other parse failure (invalid TOML, wrong types) → returns
    ///   `Err(AutonomyError::*)`.
    pub fn load_from_path(path: &Path) -> Result<Self, AutonomyError> {
        let text = match fs::read_to_string(path) {
            Ok(t) => t,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::defaults());
            }
            Err(err) => {
                return Err(AutonomyError::Io {
                    path: path.to_path_buf(),
                    source: err,
                });
            }
        };
        Self::from_toml_str(&text)
    }

    /// Parse a TOML string into an [`AutonomyConfig`]. Exposed separately
    /// so tests can exercise the parsing logic without writing tempfiles.
    pub fn from_toml_str(text: &str) -> Result<Self, AutonomyError> {
        #[derive(Deserialize)]
        struct Root {
            autonomy: Option<RawAutonomy>,
        }
        #[derive(Deserialize, Default)]
        struct RawAutonomy {
            #[serde(default)]
            prohibited_autonomous_actions: Vec<String>,
            #[serde(default)]
            require_explicit_confirmation_for: Vec<String>,
            #[serde(default)]
            session_checkpoint_interval_minutes: Option<u32>,
            #[serde(default)]
            max_actions_before_checkpoint: Option<u32>,
        }

        let root: Root = toml::from_str(text).map_err(AutonomyError::Toml)?;
        let raw = root.autonomy.unwrap_or_default();
        let interval = clamp_positive(
            raw.session_checkpoint_interval_minutes,
            DEFAULT_SESSION_CHECKPOINT_INTERVAL_MINUTES,
        );
        let max_actions = clamp_positive(
            raw.max_actions_before_checkpoint,
            DEFAULT_MAX_ACTIONS_BEFORE_CHECKPOINT,
        );
        Ok(Self {
            prohibited_autonomous_actions: raw.prohibited_autonomous_actions,
            require_explicit_confirmation_for: raw
                .require_explicit_confirmation_for,
            session_checkpoint_interval_minutes: interval,
            max_actions_before_checkpoint: max_actions,
        })
    }

    /// Infallible loader that always returns SOMETHING. Reads
    /// [`DEFAULT_BOUNDARIES_PATH`] relative to the process CWD and falls
    /// back to defaults on any error (logging at WARN). Use this from
    /// runtime init paths that cannot fail.
    pub fn load_default() -> Self {
        let path = PathBuf::from(DEFAULT_BOUNDARIES_PATH);
        match Self::load_from_path(&path) {
            Ok(cfg) => cfg,
            Err(err) => {
                tracing::warn!(
                    target: "jekko_runtime::autonomy",
                    error = %err,
                    path = %path.display(),
                    "failed to load autonomy boundaries; using safe defaults"
                );
                Self::defaults()
            }
        }
    }

    /// Returns `true` iff `action` matches one of the
    /// `prohibited_autonomous_actions` labels exactly (case-sensitive,
    /// mirroring the TOML keys). Use this at agent-initiated decision
    /// surfaces to fail closed before invoking the underlying capability.
    pub fn is_prohibited(&self, action: &str) -> bool {
        self.prohibited_autonomous_actions
            .iter()
            .any(|a| a == action)
    }
}

/// Errors surfaced by [`AutonomyConfig::load_from_path`].
#[derive(Debug, Error)]
pub enum AutonomyError {
    /// Filesystem error reading the boundaries file (other than NotFound,
    /// which is treated as "use defaults").
    #[error("failed to read autonomy boundaries at {path}: {source}")]
    Io {
        /// Path that failed to load.
        path: PathBuf,
        /// Underlying IO error.
        #[source]
        source: std::io::Error,
    },
    /// TOML parse failure.
    #[error("failed to parse autonomy boundaries TOML: {0}")]
    Toml(#[from] toml::de::Error),
}

/// Returned by [`crate::Runtime::gate_action`] (and by per-surface gating
/// callsites added under ARY-2305) when an action is on the
/// `prohibited_autonomous_actions` list.
///
/// Mirrors the shape of QO `packages.safety.autonomy.AutonomousActionDenied`
/// (PermissionError subclass carrying `action` + `reason`) so cross-process
/// callers that already handle the QO error can map this 1:1 over the IPC /
/// MCP boundary. We do NOT alias this into [`RuntimeError`] because the
/// runtime error type intentionally lives one layer above the gate; a deny
/// is a policy outcome, not an IO/store/permission failure, and should be
/// surfaced as its own variant at each gated surface.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("autonomous action denied: {action} ({reason})")]
pub struct AutonomyDeny {
    /// Action label that was refused. Matches the string in
    /// `[autonomy].prohibited_autonomous_actions`.
    pub action: String,
    /// Stable machine-readable reason code. Currently always
    /// `"prohibited_autonomous_action"` to mirror the kernel-side string;
    /// the field is kept so future reasons (e.g. `"requires_confirmation"`,
    /// `"session_budget_exhausted"`) can flow through without a breaking
    /// change.
    pub reason: String,
}

impl AutonomyDeny {
    /// Stable reason code emitted by the prohibited-action gate.
    pub const REASON_PROHIBITED: &'static str = "prohibited_autonomous_action";

    /// Construct an [`AutonomyDeny`] for a prohibited action.
    pub fn prohibited(action: impl Into<String>) -> Self {
        Self {
            action: action.into(),
            reason: Self::REASON_PROHIBITED.to_string(),
        }
    }
}

/// Clamp a `Some(0)`/`None` integer to a positive default. Anything
/// `Some(n)` with `n > 0` passes through unchanged.
fn clamp_positive(value: Option<u32>, default: u32) -> u32 {
    match value {
        Some(n) if n > 0 => n,
        _ => default,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    const VALID_TOML: &str = r#"
[autonomy]
max_actions_before_checkpoint = 12
prohibited_autonomous_actions = [
    "launch_training_run",
    "push_to_hf_or_gcs",
]
require_explicit_confirmation_for = [
    "merge_pr",
    "force_push",
]
session_checkpoint_interval_minutes = 45
"#;

    #[test]
    fn happy_path_load_from_path() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("boundaries.toml");
        fs::write(&path, VALID_TOML).unwrap();
        let cfg = AutonomyConfig::load_from_path(&path).expect("valid toml loads");
        assert_eq!(cfg.max_actions_before_checkpoint, 12);
        assert_eq!(cfg.session_checkpoint_interval_minutes, 45);
        assert_eq!(
            cfg.prohibited_autonomous_actions,
            vec![
                "launch_training_run".to_string(),
                "push_to_hf_or_gcs".to_string(),
            ]
        );
        assert_eq!(
            cfg.require_explicit_confirmation_for,
            vec!["merge_pr".to_string(), "force_push".to_string()]
        );
        assert!(cfg.is_prohibited("launch_training_run"));
        assert!(!cfg.is_prohibited("merge_pr"));
        assert!(!cfg.is_prohibited("unknown_action"));
    }

    #[test]
    fn missing_file_returns_defaults() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("does_not_exist.toml");
        let cfg = AutonomyConfig::load_from_path(&path).expect("missing → defaults");
        assert_eq!(
            cfg.max_actions_before_checkpoint,
            DEFAULT_MAX_ACTIONS_BEFORE_CHECKPOINT
        );
        assert_eq!(
            cfg.session_checkpoint_interval_minutes,
            DEFAULT_SESSION_CHECKPOINT_INTERVAL_MINUTES
        );
        assert!(cfg.prohibited_autonomous_actions.is_empty());
        assert!(cfg.require_explicit_confirmation_for.is_empty());
    }

    #[test]
    fn missing_autonomy_section_returns_defaults() {
        let toml_no_section = r#"
[db]
constraint_paths = ["db/constraints"]
"#;
        let cfg = AutonomyConfig::from_toml_str(toml_no_section)
            .expect("valid toml without [autonomy] parses");
        assert_eq!(
            cfg.max_actions_before_checkpoint,
            DEFAULT_MAX_ACTIONS_BEFORE_CHECKPOINT
        );
        assert_eq!(
            cfg.session_checkpoint_interval_minutes,
            DEFAULT_SESSION_CHECKPOINT_INTERVAL_MINUTES
        );
        assert!(cfg.prohibited_autonomous_actions.is_empty());
    }

    #[test]
    fn zero_max_actions_clamps_to_default() {
        // Mirrors QO ARY-2300 T10 — a 0 ceiling would disable the gate, so
        // we clamp back to the safe default.
        let toml_zero = r#"
[autonomy]
max_actions_before_checkpoint = 0
session_checkpoint_interval_minutes = 0
prohibited_autonomous_actions = ["launch_training_run"]
"#;
        let cfg = AutonomyConfig::from_toml_str(toml_zero).expect("parses");
        assert_eq!(
            cfg.max_actions_before_checkpoint,
            DEFAULT_MAX_ACTIONS_BEFORE_CHECKPOINT
        );
        assert_eq!(
            cfg.session_checkpoint_interval_minutes,
            DEFAULT_SESSION_CHECKPOINT_INTERVAL_MINUTES
        );
        // Lists still carry through.
        assert!(cfg.is_prohibited("launch_training_run"));
    }

    #[test]
    fn defaults_constructor_matches_constants() {
        let cfg = AutonomyConfig::defaults();
        assert_eq!(
            cfg.max_actions_before_checkpoint,
            DEFAULT_MAX_ACTIONS_BEFORE_CHECKPOINT
        );
        assert_eq!(
            cfg.session_checkpoint_interval_minutes,
            DEFAULT_SESSION_CHECKPOINT_INTERVAL_MINUTES
        );
    }

    #[test]
    fn malformed_toml_surfaces_error() {
        let bad = "[autonomy\nmax_actions_before_checkpoint = 5";
        let err = AutonomyConfig::from_toml_str(bad).unwrap_err();
        assert!(matches!(err, AutonomyError::Toml(_)));
    }

    #[test]
    fn autonomy_deny_prohibited_carries_action_and_reason_code() {
        let deny = AutonomyDeny::prohibited("launch_training_run");
        assert_eq!(deny.action, "launch_training_run");
        assert_eq!(deny.reason, AutonomyDeny::REASON_PROHIBITED);
        // Display string is the cross-process audit form; keep stable.
        assert_eq!(
            deny.to_string(),
            "autonomous action denied: launch_training_run \
             (prohibited_autonomous_action)"
        );
    }

    #[test]
    fn autonomy_deny_reason_code_constant_is_stable() {
        // Locked: this string is the wire-stable reason that mirrors the QO
        // kernel-side label. Changing it is a cross-repo break.
        assert_eq!(
            AutonomyDeny::REASON_PROHIBITED,
            "prohibited_autonomous_action"
        );
    }
}
