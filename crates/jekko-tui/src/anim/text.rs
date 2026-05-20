/// Canonical thinking verbs from tip3.txt §18.2 (lines 1298-1318). These
/// rotate during agent in-flight when no specific phase name (Bash, Read,
/// Edit, …) is being emitted, so long-running turns still feel alive. Order
/// is load-bearing — `rotating_verb` indexes into this slice modulo length,
/// so reordering would change which verb appears at a given elapsed bucket.
pub const THINKING_VERBS: &[&str] = &[
    "Metamorphosing",
    "Discombobulating",
    "Synthesizing",
    "Reconciling",
    "Auditing",
    "Verifying",
    "Untangling",
];

/// Pick a verb from `verbs` based on `elapsed`, rotating every `period`.
/// Deterministic — the same elapsed bucket maps to the same verb, so the
/// renderer stays pure with no extra state. Empty `verbs` returns `""`.
///
/// Reduced-motion mode does NOT collapse this: the verb still rotates so the
/// user knows time is passing. The motion mode only suppresses sub-second
/// glyph/color flicker, not seconds-scale text changes.
///
/// # Panics
///
/// Panics if `period` is `Duration::ZERO`. A zero period would imply rotating
/// faster than time advances, which is meaningless; callers should pass a
/// finite period (the canonical value is `Duration::from_secs(4)` per
/// tip3.txt §18.2).
pub fn rotating_verb(elapsed: Duration, verbs: &[&'static str], period: Duration) -> &'static str {
    if verbs.is_empty() {
        return "";
    }
    let period_secs = period.as_secs();
    assert!(
        period_secs > 0,
        "rotating_verb requires a non-zero period (got {period:?})"
    );
    let bucket = (elapsed.as_secs() / period_secs) as usize;
    verbs[bucket % verbs.len()]
}

/// Format an elapsed duration as a compact human label.
pub fn elapsed_label(elapsed: Duration) -> String {
    let total = elapsed.as_secs();
    if total < 60 {
        return format!("{total}s");
    }
    let (h, rem) = (total / 3600, total % 3600);
    let (m, s) = (rem / 60, rem % 60);
    if h == 0 {
        format!("{m}m {s}s")
    } else if h < 24 {
        format!("{h}h {m}m")
    } else {
        let d = h / 24;
        let h_rem = h % 24;
        format!("{d}d {h_rem}h")
    }
}
