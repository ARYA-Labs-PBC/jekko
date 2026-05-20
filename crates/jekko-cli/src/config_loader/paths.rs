use std::path::PathBuf;

use super::UiConfigLoader;

impl UiConfigLoader {
    /// Resolve the default config path under XDG conventions.
    ///
    /// Returns `Some($XDG_CONFIG_HOME/jekko/ui.toml)` if `XDG_CONFIG_HOME` is
    /// set and non-empty; otherwise `Some($HOME/.config/jekko/ui.toml)` if
    /// `HOME` is set; otherwise `None`.
    pub fn resolved_path() -> Option<PathBuf> {
        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            if !xdg.is_empty() {
                return Some(PathBuf::from(xdg).join("jekko").join("ui.toml"));
            }
        }
        if let Ok(home) = std::env::var("HOME") {
            if !home.is_empty() {
                return Some(
                    PathBuf::from(home)
                        .join(".config")
                        .join("jekko")
                        .join("ui.toml"),
                );
            }
        }
        None
    }
}
