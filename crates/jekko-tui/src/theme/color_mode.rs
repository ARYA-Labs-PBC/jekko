use std::io::IsTerminal;
use std::{env, io};

#[cfg(not(test))]
use std::sync::OnceLock;

use ratatui::style::Modifier;

/// Whether jekko should emit color escapes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorMode {
    /// Full RGB palette is enabled.
    Full,
    /// Color output is suppressed; emphasis is conveyed via [`Modifier`].
    Monochrome,
}

/// Cached env-derived [`ColorMode`].
#[cfg(not(test))]
pub fn color_mode() -> ColorMode {
    static CACHED: OnceLock<ColorMode> = OnceLock::new();
    *CACHED.get_or_init(compute_color_mode)
}

#[cfg(test)]
pub fn color_mode() -> ColorMode {
    ColorMode::Full
}

/// Uncached evaluation of the env-var precedence for env-mutating tests.
#[cfg(test)]
pub(crate) fn compute_color_mode_for_tests() -> ColorMode {
    compute_color_mode()
}

fn compute_color_mode() -> ColorMode {
    if env_truthy("CLICOLOR_FORCE") || env_truthy("FORCE_COLOR") {
        return ColorMode::Full;
    }
    if env_present_nonempty("NO_COLOR") {
        return ColorMode::Monochrome;
    }
    if env_var_eq("CLICOLOR", "0") && stdout_is_tty() {
        return ColorMode::Monochrome;
    }
    ColorMode::Full
}

fn env_truthy(name: &str) -> bool {
    match env::var(name) {
        Ok(v) => {
            let trimmed = v.trim();
            !trimmed.is_empty()
                && !trimmed.eq_ignore_ascii_case("0")
                && !trimmed.eq_ignore_ascii_case("false")
                && !trimmed.eq_ignore_ascii_case("off")
        }
        Err(_) => false,
    }
}

fn env_present_nonempty(name: &str) -> bool {
    matches!(env::var(name), Ok(v) if !v.is_empty())
}

fn env_var_eq(name: &str, expected: &str) -> bool {
    matches!(env::var(name), Ok(v) if v == expected)
}

fn stdout_is_tty() -> bool {
    io::stdout().is_terminal()
}

/// Monochrome emphasis modifier for strong/bright text.
pub fn mono_strong() -> Modifier {
    Modifier::BOLD
}

/// Monochrome emphasis modifier for secondary text.
pub fn mono_muted() -> Modifier {
    Modifier::ITALIC
}

/// Monochrome emphasis modifier for tertiary text.
pub fn mono_dim() -> Modifier {
    Modifier::DIM
}
