//! Deterministic background pulse math ported from `bg-pulse.tsx`.

use ratatui::style::Color;

pub const PERIOD: f64 = 4600.0;
pub const RINGS: usize = 3;
pub const WIDTH: f64 = 3.8;
pub const TAIL: f64 = 9.5;
pub const AMP: f64 = 0.55;
pub const TAIL_AMP: f64 = 0.16;
pub const BREATH_AMP: f64 = 0.05;
pub const BREATH_SPEED: f64 = 0.0008;
pub const PHASE_OFFSET: f64 = 0.29;

#[derive(Clone, Copy, Debug)]
pub struct PulseSample {
    pub strength: f64,
    pub color: Color,
}

/// Sample the pulse for one terminal cell. `tick` is app-owned and
/// deterministic; callers can snapshot fixed ticks without depending on time.
pub fn sample(
    tick: u64,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
    center_x: f64,
    center_y: f64,
) -> PulseSample {
    let t = tick as f64 * (1000.0 / 60.0);
    let w = width.max(1) as f64;
    let h = height.max(1) as f64;
    let reach = center_x
        .max(w - center_x)
        .hypot((center_y.max(h - center_y)) * 2.0)
        + TAIL;
    let dx = x as f64 + 0.5 - center_x;
    let dy = (y as f64 + 0.5 - center_y) * 2.0;
    let dist = dx.hypot(dy);
    let mut level = 0.0;

    for i in 0..RINGS {
        let offset = i as f64 / RINGS as f64;
        let phase = (t / PERIOD + offset - PHASE_OFFSET + 1.0) % 1.0;
        let envelope = (phase * std::f64::consts::PI).sin();
        let eased = envelope * envelope * (3.0 - 2.0 * envelope);
        let head = phase * reach;
        let delta = dist - head;
        let crest = if delta.abs() < WIDTH {
            0.5 + 0.5 * ((delta / WIDTH) * std::f64::consts::PI).cos()
        } else {
            0.0
        };
        let tail = if delta < 0.0 && delta > -TAIL {
            (1.0 + delta / TAIL).powf(2.3)
        } else {
            0.0
        };
        level += (crest * AMP + tail * TAIL_AMP) * eased;
    }

    let edge_falloff = (1.0 - (dist / (reach * 0.85)).powi(2)).max(0.0);
    let breath = (0.5 + 0.5 * (t * BREATH_SPEED).sin()) * BREATH_AMP;
    let strength =
        (((level / RINGS as f64) * edge_falloff) + breath * edge_falloff).clamp(0.0, 1.0);

    PulseSample {
        strength,
        color: tint(
            Color::Rgb(0x11, 0x18, 0x20),
            Color::Rgb(0xf4, 0xc5, 0x42),
            strength * 0.7,
        ),
    }
}

fn tint(base: Color, accent: Color, strength: f64) -> Color {
    match (base, accent) {
        (Color::Rgb(br, bg, bb), Color::Rgb(ar, ag, ab)) => {
            let s = strength.clamp(0.0, 1.0);
            Color::Rgb(lerp(br, ar, s), lerp(bg, ag, s), lerp(bb, ab, s))
        }
        _ => base,
    }
}

fn lerp(a: u8, b: u8, t: f64) -> u8 {
    (a as f64 + (b as f64 - a as f64) * t)
        .round()
        .clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_tick_is_deterministic() {
        let a = sample(42, 10, 5, 80, 24, 40.0, 12.0);
        let b = sample(42, 10, 5, 80, 24, 40.0, 12.0);
        assert_eq!(a.color, b.color);
        assert!((a.strength - b.strength).abs() < f64::EPSILON);
    }

    #[test]
    fn pulse_changes_across_ticks() {
        let a = sample(1, 40, 12, 80, 24, 40.0, 12.0);
        let b = sample(180, 40, 12, 80, 24, 40.0, 12.0);
        assert_ne!(a.color, b.color);
    }
}
