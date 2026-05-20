/// Sine-wave color interpolation between `from` and `to`, at `hz` Hz. Use this
/// for the "Working" verb that oscillates cyan↔white. Reduced-motion mode
/// returns `from` flat.
pub fn oscillate_color(elapsed: Duration, hz: f32, from: Color, to: Color) -> Color {
    oscillate_color_with_motion(elapsed, hz, from, to, motion_enabled())
}

/// Motion-aware color oscillator. Prefer when the motion preference is
/// already known for the current frame.
pub fn oscillate_color_with_motion(
    elapsed: Duration,
    hz: f32,
    from: Color,
    to: Color,
    motion_enabled: bool,
) -> Color {
    if !motion_enabled {
        return from;
    }
    let t = elapsed.as_secs_f32();
    let k = 0.5 + 0.5 * (t * std::f32::consts::TAU * hz).sin();
    lerp_color(from, to, k)
}

/// Convenience: oscillate between codex `CYAN_TAB` and `FG_STRONG` at the
/// default 1Hz. Inlined RGB values match `theme::codex::{CYAN_TAB, FG_STRONG}`
/// so this stays compile-safe while the legacy purge churns the theme module.
pub fn oscillate_codex_default(elapsed: Duration) -> Color {
    const CYAN_TAB: Color = Color::Rgb(0x00, 0xd7, 0xdf);
    const FG_STRONG: Color = Color::Rgb(0xff, 0xff, 0xff);
    oscillate_color(elapsed, OSCILLATE_DEFAULT_HZ, CYAN_TAB, FG_STRONG)
}

/// Motion-aware version of [`oscillate_codex_default`].
pub fn oscillate_codex_default_with_motion(elapsed: Duration, motion_enabled: bool) -> Color {
    const CYAN_TAB: Color = Color::Rgb(0x00, 0xd7, 0xdf);
    const FG_STRONG: Color = Color::Rgb(0xff, 0xff, 0xff);
    oscillate_color_with_motion(
        elapsed,
        OSCILLATE_DEFAULT_HZ,
        CYAN_TAB,
        FG_STRONG,
        motion_enabled,
    )
}

/// Linearly interpolate between two RGB colors. Non-RGB inputs return `from`
/// untouched.
pub fn lerp_color(from: Color, to: Color, k: f32) -> Color {
    let k = k.clamp(0.0, 1.0);
    match (from, to) {
        (Color::Rgb(fr, fg, fb), Color::Rgb(tr, tg, tb)) => {
            Color::Rgb(lerp_u8(fr, tr, k), lerp_u8(fg, tg, k), lerp_u8(fb, tb, k))
        }
        _ => from,
    }
}

fn lerp_u8(a: u8, b: u8, k: f32) -> u8 {
    let a = a as f32;
    let b = b as f32;
    (a + (b - a) * k).round().clamp(0.0, 255.0) as u8
}
