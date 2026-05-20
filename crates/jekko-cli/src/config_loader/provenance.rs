use std::collections::HashMap;
use std::path::{Path, PathBuf};

use jekko_core::config::ui::UiConfig;

use super::env::{env_flag, parse_animation_level};

/// Source layer that populated a particular config field.
///
/// Tracked per-field so `/doctor` can explain why a value is what it is --
/// useful when a TOML edit didn't take effect because env or CLI clobbered it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// Compiled-in default (the value came from
    /// [`jekko_core::config::ui::UiConfig::defaults`]).
    Default,
    /// Loaded from a TOML file at the given path.
    TomlPath(PathBuf),
    /// Set by the named environment variable.
    Env(&'static str),
    /// Set by the named CLI flag (e.g. `"--reduced-motion"`).
    Cli(&'static str),
}

impl Source {
    /// Render the source as a short `/doctor`-style label
    /// (`default`, `toml(/path)`, `env(VAR)`, `cli(--flag)`).
    pub fn label(&self) -> String {
        match self {
            Source::Default => "default".to_string(),
            Source::TomlPath(p) => format!("toml({})", p.display()),
            Source::Env(name) => format!("env({name})"),
            Source::Cli(name) => format!("cli({name})"),
        }
    }
}

/// Per-field source map keyed by canonical `"section.field"` strings such as
/// `"ui.theme"` or `"accessibility.reduced_motion"`.
pub type ProvenanceMap = HashMap<&'static str, Source>;

/// Resolved UI config + per-field source map produced by
/// [`super::UiConfigLoader::resolve_with_provenance`].
#[derive(Debug, Clone)]
pub struct ResolvedUiConfig {
    /// Effective config after defaults -> TOML -> env merge.
    pub config: UiConfig,
    /// Per-field provenance. Every key in [`TRACKED_FIELDS`] is present;
    /// fields that no layer touched remain [`Source::Default`].
    pub provenance: ProvenanceMap,
    /// Path of the TOML file consulted (if any). `None` when no file was
    /// explicitly named and the default XDG path didn't exist on disk.
    pub toml_path: Option<PathBuf>,
}

impl ResolvedUiConfig {
    /// Record that a CLI flag overrode a particular field. Mutates the
    /// provenance map so the next `/doctor` print reflects the CLI source.
    ///
    /// `field` must be one of [`TRACKED_FIELDS`] -- passing an unknown key
    /// inserts it anyway so future fields don't silently drop (they will
    /// just print without per-field defaults).
    pub fn record_cli_override(&mut self, field: &'static str, flag: &'static str) {
        self.provenance.insert(field, Source::Cli(flag));
    }

    /// Convenience accessor -- defaults to [`Source::Default`] when a field
    /// is missing from the map. Used by `/doctor` so an unknown future
    /// field doesn't crash.
    pub fn source_for(&self, field: &str) -> Source {
        self.provenance
            .get(field)
            .cloned()
            .unwrap_or(Source::Default)
    }
}

/// Canonical list of fields tracked by [`ProvenanceMap`]. Stable strings
/// because [`Source::Env`]/[`Source::Cli`] both store `&'static str` names.
///
/// One entry per [`UiConfig`] leaf field across every section. New fields
/// added to `jekko_core::config::ui` should be mirrored here so `/doctor`
/// reports their source.
pub const TRACKED_FIELDS: &[&str] = &[
    "ui.theme",
    "ui.alternate_screen",
    "ui.mouse",
    "ui.animations",
    "ui.active_fps",
    "ui.idle_fps",
    "ui.timer_tick_ms",
    "ui.timeline_overscan",
    "ui.stick_to_bottom",
    "ui.compact_transcripts",
    "ui.max_compact_transcript_lines",
    "ui.show_scrollbar",
    "ui.soft_wrap",
    "ui.diff_context_lines",
    "input.single_line_default",
    "input.history_limit",
    "input.at_file_completion",
    "execution.prefer_pty",
    "execution.chunk_latency_ms",
    "execution.chunk_max_bytes",
    "execution.kill_grace_ms",
    "execution.inherit_env",
    "execution.force_color",
    "status.show_model",
    "status.show_pwd",
    "status.show_branch",
    "status.show_profile",
    "accessibility.reduced_motion",
    "accessibility.respect_no_color",
    "accessibility.high_contrast",
];

/// Walk an overlay parsed from a TOML file and mark every field that was
/// explicitly set (i.e. the overlay field is `Some(..)`) with
/// `Source::TomlPath(path)`. Unset fields are left at whatever source the
/// map currently records (typically `Source::Default`).
pub(super) fn record_toml_overlay(overlay: &UiConfig, path: &Path, provenance: &mut ProvenanceMap) {
    let path_buf = path.to_path_buf();
    let mut mark = |field: &'static str| {
        provenance.insert(field, Source::TomlPath(path_buf.clone()));
    };

    if overlay.ui.theme.is_some() {
        mark("ui.theme");
    }
    if overlay.ui.alternate_screen.is_some() {
        mark("ui.alternate_screen");
    }
    if overlay.ui.mouse.is_some() {
        mark("ui.mouse");
    }
    if overlay.ui.animations.is_some() {
        mark("ui.animations");
    }
    if overlay.ui.active_fps.is_some() {
        mark("ui.active_fps");
    }
    if overlay.ui.idle_fps.is_some() {
        mark("ui.idle_fps");
    }
    if overlay.ui.timer_tick_ms.is_some() {
        mark("ui.timer_tick_ms");
    }
    if overlay.ui.timeline_overscan.is_some() {
        mark("ui.timeline_overscan");
    }
    if overlay.ui.stick_to_bottom.is_some() {
        mark("ui.stick_to_bottom");
    }
    if overlay.ui.compact_transcripts.is_some() {
        mark("ui.compact_transcripts");
    }
    if overlay.ui.max_compact_transcript_lines.is_some() {
        mark("ui.max_compact_transcript_lines");
    }
    if overlay.ui.show_scrollbar.is_some() {
        mark("ui.show_scrollbar");
    }
    if overlay.ui.soft_wrap.is_some() {
        mark("ui.soft_wrap");
    }
    if overlay.ui.diff_context_lines.is_some() {
        mark("ui.diff_context_lines");
    }

    if overlay.input.single_line_default.is_some() {
        mark("input.single_line_default");
    }
    if overlay.input.history_limit.is_some() {
        mark("input.history_limit");
    }
    if overlay.input.at_file_completion.is_some() {
        mark("input.at_file_completion");
    }

    if overlay.execution.prefer_pty.is_some() {
        mark("execution.prefer_pty");
    }
    if overlay.execution.chunk_latency_ms.is_some() {
        mark("execution.chunk_latency_ms");
    }
    if overlay.execution.chunk_max_bytes.is_some() {
        mark("execution.chunk_max_bytes");
    }
    if overlay.execution.kill_grace_ms.is_some() {
        mark("execution.kill_grace_ms");
    }
    if overlay.execution.inherit_env.is_some() {
        mark("execution.inherit_env");
    }
    if overlay.execution.force_color.is_some() {
        mark("execution.force_color");
    }

    if overlay.status.show_model.is_some() {
        mark("status.show_model");
    }
    if overlay.status.show_pwd.is_some() {
        mark("status.show_pwd");
    }
    if overlay.status.show_branch.is_some() {
        mark("status.show_branch");
    }
    if overlay.status.show_profile.is_some() {
        mark("status.show_profile");
    }

    if overlay.accessibility.reduced_motion.is_some() {
        mark("accessibility.reduced_motion");
    }
    if overlay.accessibility.respect_no_color.is_some() {
        mark("accessibility.respect_no_color");
    }
    if overlay.accessibility.high_contrast.is_some() {
        mark("accessibility.high_contrast");
    }
}

/// Mirror of [`super::UiConfigLoader::merge_env`] that also records which
/// fields were touched.
pub(super) fn apply_env_with_provenance(
    mut cfg: UiConfig,
    provenance: &mut ProvenanceMap,
) -> UiConfig {
    if let Ok(value) = std::env::var("JEKKO_UI_THEME") {
        if !value.is_empty() {
            cfg.ui.theme = Some(value);
            provenance.insert("ui.theme", Source::Env("JEKKO_UI_THEME"));
        }
    }
    if let Ok(value) = std::env::var("JEKKO_UI_ANIMATIONS") {
        if let Some(level) = parse_animation_level(&value) {
            cfg.ui.animations = Some(level);
            provenance.insert("ui.animations", Source::Env("JEKKO_UI_ANIMATIONS"));
        }
    }
    if env_flag("JEKKO_REDUCED_MOTION") {
        cfg.accessibility.reduced_motion = Some(true);
        provenance.insert(
            "accessibility.reduced_motion",
            Source::Env("JEKKO_REDUCED_MOTION"),
        );
    }
    if env_flag("JEKKO_NO_ALT_SCREEN") {
        cfg.ui.alternate_screen = Some(false);
        provenance.insert("ui.alternate_screen", Source::Env("JEKKO_NO_ALT_SCREEN"));
    }
    if env_flag("JEKKO_NO_MOUSE") {
        cfg.ui.mouse = Some(false);
        provenance.insert("ui.mouse", Source::Env("JEKKO_NO_MOUSE"));
    }
    if let Ok(value) = std::env::var("JEKKO_HISTORY_SIZE") {
        if let Ok(parsed) = value.parse::<usize>() {
            cfg.input.history_limit = Some(parsed);
            provenance.insert("input.history_limit", Source::Env("JEKKO_HISTORY_SIZE"));
        }
    }
    cfg
}
