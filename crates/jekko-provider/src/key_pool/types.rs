use std::path::PathBuf;

use crate::setup::ModelKeySource;

/// Canonical default user id used in single-user (locked) mode.
pub const DEFAULT_USER_ID: &str = "user";

/// Per-user filename inside `~/.jekko/users/<user_id>/`.
pub const LLM_ENV_FILENAME: &str = "llm.env";

/// Per-user state sqlite filename owned by the balancer.
pub const STATE_DB_FILENAME: &str = "state.sqlite";

/// One discovered user directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserDir {
    /// User id (the directory name under `users/`).
    pub user_id: String,
    /// Absolute path of `~/.jekko/users/<user_id>/`.
    pub dir: PathBuf,
    /// Absolute path of `<dir>/llm.env`.
    pub llm_env_path: PathBuf,
    /// Absolute path of `<dir>/state.sqlite`.
    pub state_db_path: PathBuf,
}

/// One credential candidate sourced from a user's `llm.env`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyCandidate {
    /// User dir id.
    pub user_id: String,
    /// Provider id (matches `CatalogEntry::provider_id`).
    pub provider_id: String,
    /// Resolved env-var name that produced the value (first non-blank from the
    /// catalog's `env_names` list).
    pub env_name: String,
    /// Raw credential value.
    pub key: String,
    /// Path the value came from (for tracing / status output).
    pub source_path: PathBuf,
    /// Source classification.
    pub source: ModelKeySource,
}
