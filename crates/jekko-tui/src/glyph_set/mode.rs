/// Active glyph mode for this process. Computed on first call from environment
/// variables, then cached in a [`OnceLock`] for the rest of the run.
pub fn mode() -> GlyphMode {
    static CACHED: OnceLock<GlyphMode> = OnceLock::new();
    *CACHED.get_or_init(compute_mode)
}

/// Compute the active mode from process environment. Exposed so tests can
/// exercise the detection without going through the `OnceLock` cache.
pub fn compute_mode() -> GlyphMode {
    if env_truthy("JEKKO_ASCII") {
        return GlyphMode::Ascii;
    }
    if locale_is_c() {
        return GlyphMode::Ascii;
    }
    GlyphMode::Unicode
}

/// Convenience accessor: the [`GlyphSet`] matching the current [`mode`].
pub fn current() -> &'static GlyphSet {
    match mode() {
        GlyphMode::Unicode => &UNICODE,
        GlyphMode::Ascii => &ASCII,
    }
}

fn env_truthy(name: &str) -> bool {
    match std::env::var(name) {
        Ok(value) => {
            let trimmed = value.trim();
            !trimmed.is_empty()
                && !trimmed.eq_ignore_ascii_case("0")
                && !trimmed.eq_ignore_ascii_case("false")
        }
        Err(_) => false,
    }
}

fn locale_is_c() -> bool {
    for var in ["LC_ALL", "LC_CTYPE", "LANG"] {
        match std::env::var(var) {
            Ok(value) => {
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    continue;
                }
                return is_c_locale_value(trimmed);
            }
            Err(_) => continue,
        }
    }
    false
}

fn is_c_locale_value(value: &str) -> bool {
    value.eq_ignore_ascii_case("C") || value.eq_ignore_ascii_case("POSIX")
}
