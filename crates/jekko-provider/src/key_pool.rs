//! Multi-user key pool for `~/.jekko/users/<user_id>/llm.env`.
//!
//! Layout:
//!
//! ```text
//! ~/.jekko/
//! ├── users/
//! │   ├── user/                  ← always-on default (locked + unlocked)
//! │   │   ├── llm.env
//! │   │   └── state.sqlite       ← owned by [`crate::key_balancer`] consumers
//! │   ├── user_1/                ← unlock-only; auto-detected if dir + llm.env present
//! │   │   └── llm.env
//! │   └── user_2/                ← drop in at any time; picked up on next scan
//! │       └── llm.env
//! └── jekko.env.bak              ← post-migration only
//! ```
//!
//! Locked mode (no jnoccio unlock): only `users/user/` is read.
//! Unlocked mode: every `users/*/` subdir containing `llm.env` becomes a
//! candidate. Each `(provider, user)` pair is a unique candidate; the model
//! axis is layered on top by the balancer.

mod dirs;
mod pool;
mod types;

#[cfg(test)]
mod tests;

pub use dirs::{discover_in, discover_user_dirs, user_dir, users_root};
pub use pool::KeyPool;
pub use types::{KeyCandidate, UserDir, DEFAULT_USER_ID, LLM_ENV_FILENAME, STATE_DB_FILENAME};
