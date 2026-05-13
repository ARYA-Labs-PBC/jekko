//! FSRS-style spaced-repetition for cells and topics.
//!
//! Determinism: no clock reads; the host passes `BENCH_NOW`. All math is
//! `f32` and stays in `[0, 1]` for strength.

/// Default per-cell half-life baseline (hours).
pub const CELL_BASE_HOURS: f32 = 24.0;
/// Default per-topic half-life baseline (hours).
pub const TOPIC_BASE_HOURS: f32 = 24.0;

#[inline]
pub fn ln1p(x: f32) -> f32 {
    (1.0 + x).ln()
}

pub fn cell_half_life_hours(strength: f32, success_rate: f32, recall_count: u32) -> f32 {
    let s_factor = 1.0 + 3.0 * success_rate;
    let r_factor = 1.0 + 0.4 * ln1p(recall_count as f32);
    let strength_factor = 1.0 + 1.5 * strength.clamp(0.0, 1.0);
    CELL_BASE_HOURS * s_factor * r_factor * strength_factor
}

pub fn topic_half_life_hours(
    strength: f32,
    success_count: u32,
    failure_count: u32,
    recall_count: u32,
) -> f32 {
    let denom = (success_count + failure_count + 1) as f32;
    let s_factor = 1.0 + 4.0 * (success_count as f32 / denom);
    let r_factor = 1.0 + 0.5 * ln1p(recall_count as f32);
    let strength_factor = 1.0 + 2.0 * strength.clamp(0.0, 1.0);
    TOPIC_BASE_HOURS * s_factor * r_factor * strength_factor
}

/// `Δt` in hours from prior recall.
pub fn hours_between(prior: &str, now: &str) -> f32 {
    // ISO compare: if `now <= prior` return 0 (don't reward future-leak).
    if now <= prior {
        return 0.0;
    }
    let pd = parse_iso(prior).unwrap_or(0);
    let nd = parse_iso(now).unwrap_or(0);
    ((nd - pd).max(0)) as f32 / 3600.0
}

fn parse_iso(s: &str) -> Option<i64> {
    // YYYY-MM-DDThh:mm:ssZ → seconds since 2000-01-01 (rough; deterministic).
    // Accepts shorter prefixes by zero-padding implicitly.
    if s.len() < 19 {
        return None;
    }
    let year: i64 = s.get(0..4)?.parse().ok()?;
    let month: i64 = s.get(5..7)?.parse().ok()?;
    let day: i64 = s.get(8..10)?.parse().ok()?;
    let hour: i64 = s.get(11..13)?.parse().ok()?;
    let minute: i64 = s.get(14..16)?.parse().ok()?;
    let second: i64 = s.get(17..19)?.parse().ok()?;
    let days = (year - 2000) * 365 + (month - 1) * 30 + (day - 1);
    Some(days * 86400 + hour * 3600 + minute * 60 + second)
}

pub fn decay(strength: f32, hours: f32, half_life_hours: f32) -> f32 {
    if half_life_hours <= 0.0 {
        return strength;
    }
    let decay_factor = (-hours / half_life_hours).exp();
    (strength * decay_factor).clamp(0.0, 1.0)
}

pub fn strengthen_cell(prev: f32, success_rate: f32, source_quality: f32, utility: f32) -> f32 {
    let bump = 0.25 * success_rate + 0.10 * utility + 0.10 * source_quality;
    (prev + bump).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hours_between_is_monotonic() {
        assert_eq!(
            hours_between("2026-01-01T00:00:00Z", "2026-01-01T00:00:00Z"),
            0.0
        );
        let h = hours_between("2026-01-01T00:00:00Z", "2026-01-02T00:00:00Z");
        assert!((h - 24.0).abs() < 0.1);
    }

    #[test]
    fn decay_shrinks_strength() {
        let s = decay(1.0, 24.0, 24.0);
        assert!(s > 0.3 && s < 0.4);
    }

    #[test]
    fn strengthen_cell_clamps() {
        let s = strengthen_cell(0.9, 1.0, 1.0, 1.0);
        assert!(s <= 1.0);
        assert!(s > 0.9);
    }
}
