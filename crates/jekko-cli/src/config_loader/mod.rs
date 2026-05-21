//! TOML/env loader for the extended UI configuration described in tip8.
//!
//! Lives in `jekko-cli` (not `jekko-core`) because `jekko-core` is intentionally
//! I/O-free (see crate-root docstring: "no filesystem, no network, no SQL, no
//! clock access"). Filesystem + environment resolution happens here, and the
//! CLI feeds the resolved [`jekko_core::config::ui::UiConfig`] overlay down
//! to the runtime/TUI layers that need it.
//!
//! Override chain (highest precedence last):
//!
//! 1. Compiled-in defaults ([`jekko_core::config::ui::UiConfig::defaults`]).
//! 2. TOML file at the resolved XDG path (see [`UiConfigLoader::resolved_path`]).
//! 3. Process environment ([`UiConfigLoader::merge_env`]) -- see the env map below.
//! 4. CLI flags applied by the caller after [`UiConfigLoader::merge_env`].
//!
//! # Environment variable map
//!
//! Only the most-requested switches are exposed; an exhaustive map is overkill.
//! The variables are documented here so the integration surface stays narrow:
//!
//! | Variable                 | Field                                |
//! |--------------------------|--------------------------------------|
//! | `JEKKO_UI_THEME`         | `ui.theme`                           |
//! | `JEKKO_UI_ANIMATIONS`    | `ui.animations` (`off`/`subtle`/`full`) |
//! | `JEKKO_REDUCED_MOTION=1` | `accessibility.reduced_motion=true`  |
//! | `JEKKO_NO_ALT_SCREEN=1`  | `ui.alternate_screen=false`          |
//! | `JEKKO_NO_MOUSE=1`       | `ui.mouse=false`                     |
//! | `JEKKO_HISTORY_SIZE`     | `input.history_limit`                |
//!
//! Note: `JEKKO_REDUCED_MOTION` previously targeted `ui.animation.reduced_motion`
//! in the now-removed loader-local schema. The canonical home in Codex's
//! overlay schema is `accessibility.reduced_motion`. Similarly `JEKKO_HISTORY_SIZE`
//! previously targeted `input.history_size`; the canonical field is
//! `input.history_limit`.

mod env;
mod error;
mod loader;
mod paths;
mod provenance;

#[cfg(test)]
mod tests;

pub use error::LoadError;
pub use loader::UiConfigLoader;
pub use provenance::{ProvenanceMap, ResolvedUiConfig, Source, TRACKED_FIELDS};
