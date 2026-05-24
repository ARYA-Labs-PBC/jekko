use serde::{Deserialize, Serialize};

/// Log verbosity, mirroring the `LogLevel` literal union.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LogLevel {
    /// Verbose debug logging.
    #[serde(rename = "DEBUG")]
    Debug,
    /// Informational logging (default).
    #[serde(rename = "INFO")]
    Info,
    /// Warnings only.
    #[serde(rename = "WARN")]
    Warn,
    /// Errors only.
    #[serde(rename = "ERROR")]
    Error,
}

/// Sharing policy literal (`manual` / `auto` / `disabled`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SharePolicy {
    /// Manual sharing only.
    Manual,
    /// Automatic sharing.
    Auto,
    /// Sharing disabled.
    Disabled,
}

/// Either `true`/`false` or the literal `"notify"`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AutoUpdate {
    /// Boolean form.
    Bool(bool),
    /// Notify-only form.
    Notify(NotifyLiteral),
}

/// Helper enum so serde can prefer the boolean variant in [`AutoUpdate`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotifyLiteral {
    /// `"notify"` literal.
    #[serde(rename = "notify")]
    Notify,
}

/// Layout literal (`stretch` is the only valid value today; the field is
/// retained for compatibility).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Layout {
    /// Stretch layout.
    Stretch,
}
