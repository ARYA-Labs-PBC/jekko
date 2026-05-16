//! Section: Jankurai Detection
//!
//! Binary presence check. Returns `false` when jankurai is not in `PATH`.
//! The install URL is a stable constant so it can be shown in the panel
//! and in chat intercept messages without duplicating the string.

/// Install URL shown when jankurai is not found.
pub const JANKURAI_INSTALL_URL: &str = "https://github.com/neverhuman/jankurai/";

/// Returns `true` when a `jankurai` binary is reachable via `PATH`.
pub fn is_jankurai_installed() -> bool {
    std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .any(|dir| dir.join("jankurai").exists())
}
