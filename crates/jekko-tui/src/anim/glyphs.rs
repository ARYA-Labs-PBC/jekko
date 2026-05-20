/// Pick a pulse glyph for the current elapsed time. Returns a static `&str` so
/// callers can drop it into a `Span` without allocating. When reduced-motion
/// is active, always returns the brightest frame so the bullet still reads as
/// "running" without flicker.
pub fn pulse_glyph(elapsed: Duration) -> &'static str {
    pulse_glyph_with_motion(elapsed, motion_enabled())
}

/// Motion-aware pulse glyph picker. Callers that already resolved the motion
/// preference once per frame should use this to avoid re-reading env/TOML.
pub fn pulse_glyph_with_motion(elapsed: Duration, motion_enabled: bool) -> &'static str {
    if !motion_enabled {
        return PULSE_FRAMES[0];
    }
    let phase = (elapsed.as_millis() / PULSE_PERIOD.as_millis()) as usize;
    PULSE_FRAMES[phase % PULSE_FRAMES.len()]
}

/// Pick a braille spinner frame. Same semantics as `pulse_glyph` but uses the
/// 10-frame braille cycle. ~80ms per frame (12.5 FPS) reads as smooth-but-not-
/// distracting in terminals.
pub fn spinner_glyph(elapsed: Duration) -> &'static str {
    spinner_glyph_with_motion(elapsed, motion_enabled())
}

/// Motion-aware spinner picker. Mirrors [`pulse_glyph_with_motion`].
pub fn spinner_glyph_with_motion(elapsed: Duration, motion_enabled: bool) -> &'static str {
    if !motion_enabled {
        return SPINNER_FRAMES[0];
    }
    let phase = (elapsed.as_millis() / 80) as usize;
    SPINNER_FRAMES[phase % SPINNER_FRAMES.len()]
}
