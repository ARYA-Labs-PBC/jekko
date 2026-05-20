/// Returns true if reduced-motion is inactive. Reads env/config once, then
/// caches the result.
///
/// This is the legacy entry point — it consults env vars and a narrow
/// on-disk TOML parse. Prefer [`motion_enabled_with_cfg`] when the caller
/// already has a resolved [`UiConfig`] from `jekko_cli::config_loader`, since
/// that respects the full override chain (defaults → TOML → env → CLI flags)
/// without re-implementing it here.
pub fn motion_enabled() -> bool {
    static CACHED: OnceLock<bool> = OnceLock::new();
    *CACHED.get_or_init(|| {
        motion_enabled_from_sources(
            env::var("MOTION").ok().as_deref(),
            env::var("JEKKO_REDUCED_MOTION").ok().as_deref(),
            configured_reduced_motion(),
        )
    })
}

/// Preferred entry point: returns true if reduced-motion is inactive, given
/// a resolved [`UiConfig`] from `jekko_cli::config_loader::UiConfigLoader`.
///
/// When `cfg` is `Some` and `cfg.accessibility.reduced_motion == Some(true)`,
/// motion is disabled. When `cfg` is `None`, falls back to [`motion_enabled`]
/// which inspects env vars and the on-disk TOML directly. This means tests
/// and renderers that have not yet been wired to the CLI loader keep working
/// unchanged.
pub fn motion_enabled_with_cfg(cfg: Option<&UiConfig>) -> bool {
    if let Some(cfg) = cfg {
        if cfg.accessibility.reduced_motion == Some(true) {
            return false;
        }
    }
    let configured_motion = match cfg.and_then(|c| c.accessibility.reduced_motion) {
        Some(value) => Some(value),
        None => configured_reduced_motion(),
    };
    motion_enabled_from_sources(
        env::var("MOTION").ok().as_deref(),
        env::var("JEKKO_REDUCED_MOTION").ok().as_deref(),
        configured_motion,
    )
}

fn motion_enabled_from_sources(
    motion: Option<&str>,
    jekko_reduced_motion: Option<&str>,
    config_reduced_motion: Option<bool>,
) -> bool {
    if motion == Some("0") {
        return false;
    }
    if jekko_reduced_motion == Some("1") {
        return false;
    }
    config_reduced_motion != Some(true)
}

fn configured_reduced_motion() -> Option<bool> {
    let path = ui_config_path()?;
    let text = fs::read_to_string(path).ok()?;
    parse_ui_toml_reduced_motion(&text)
}

fn ui_config_path() -> Option<PathBuf> {
    let base = match env::var_os("XDG_CONFIG_HOME") {
        Some(value) => PathBuf::from(value),
        None => match env::var_os("HOME") {
            Some(home) => PathBuf::from(home).join(".config"),
            None => return None,
        },
    };
    Some(base.join("jekko").join("ui.toml"))
}

fn parse_ui_toml_reduced_motion(text: &str) -> Option<bool> {
    let mut in_animation = false;
    let mut reduced_motion = None;

    for raw in text.lines() {
        let line = strip_toml_comment(raw).trim();
        if line.is_empty() {
            continue;
        }

        if let Some(section) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            in_animation = section.trim() == "ui.animation";
            continue;
        }

        if !in_animation {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() == "reduced_motion" {
            if let Some(value) = parse_toml_bool(value) {
                reduced_motion = Some(value);
            }
        }
    }

    reduced_motion
}

fn strip_toml_comment(line: &str) -> &str {
    line.split_once('#').map_or(line, |(before, _)| before)
}

fn parse_toml_bool(value: &str) -> Option<bool> {
    match value.split_whitespace().next()? {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}
