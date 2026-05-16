//! Engagement state machine for the Shell route's empty-state.
//!
//! Phase A of the splash → shell collapse: the JEKKO logo sits centered in
//! the empty activity feed until the user "engages" (presses Enter on an
//! empty prompt or submits their first prompt). On engage, the logo slides
//! up + off over [`LOGO_SLIDE_DURATION`]; once the slide completes, the
//! empty-state body suppresses both the logo and the hint copy and only
//! the transcript chrome remains.
//!
//! This module is intentionally pure data + time math — no rendering, no
//! key dispatch — so it can be unit-tested in isolation.

use std::time::{Duration, Instant};

/// How long the logo takes to slide off the empty-state.
///
/// The progress curve is linear (`elapsed / duration`) and saturates at
/// `1.0` once `LOGO_SLIDE_DURATION` has elapsed. The cadence is chosen to
/// read as "a deliberate beat" without holding the user up — long enough
/// that the slide is visually obvious on a 60-fps redraw loop, short enough
/// that it never feels in the way.
pub const LOGO_SLIDE_DURATION: Duration = Duration::from_millis(900);

/// Tri-state engagement machine driven by the App's run loop.
///
/// Transitions are one-way: `Idle → Engaging → Engaged`. The state only
/// moves on explicit dispatch (`Action::EngageSession`, `Action::PromptSubmit`)
/// or via [`EngagementState::tick`] at frame cadence (the only way to
/// promote `Engaging → Engaged`).
#[derive(Debug, Clone, Copy, Default)]
pub enum EngagementState {
    /// Pre-engagement: empty-state shows the logo + hint text.
    #[default]
    Idle,
    /// User has engaged; the logo slide animation is in flight.
    Engaging { started_at: Instant },
    /// Slide complete; empty-state body suppresses the logo + hint.
    Engaged,
}

impl EngagementState {
    /// True while the state machine is in [`EngagementState::Idle`].
    pub fn is_idle(&self) -> bool {
        matches!(self, Self::Idle)
    }

    /// True while the slide animation is in flight.
    pub fn is_engaging(&self) -> bool {
        matches!(self, Self::Engaging { .. })
    }

    /// True once the slide has finished and the empty-state body is bare.
    pub fn is_engaged(&self) -> bool {
        matches!(self, Self::Engaged)
    }

    /// Slide progress in `[0.0, 1.0]`. `Idle` is always `0.0`; `Engaged`
    /// is always `1.0`; `Engaging` clamps `elapsed / LOGO_SLIDE_DURATION`.
    pub fn slide_progress(&self) -> f32 {
        match self {
            Self::Idle => 0.0,
            Self::Engaged => 1.0,
            Self::Engaging { started_at } => {
                let elapsed = started_at.elapsed().as_secs_f32();
                let duration = LOGO_SLIDE_DURATION.as_secs_f32();
                if duration <= 0.0 {
                    return 1.0;
                }
                (elapsed / duration).clamp(0.0, 1.0)
            }
        }
    }

    /// Frame-cadence promoter: when the slide window has fully elapsed,
    /// flip `Engaging → Engaged`. Other states are a no-op. Safe to call
    /// from the App's per-frame tick block.
    pub fn tick(&mut self) {
        if let Self::Engaging { started_at } = self {
            if started_at.elapsed() >= LOGO_SLIDE_DURATION {
                *self = Self::Engaged;
            }
        }
    }

    /// Begin engaging. Transitions `Idle → Engaging`; other states are a
    /// no-op (the slide does not restart once it has begun).
    pub fn engage_now(&mut self) {
        if matches!(self, Self::Idle) {
            *self = Self::Engaging {
                started_at: Instant::now(),
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_idle() {
        let state = EngagementState::default();
        assert!(state.is_idle());
        assert!(!state.is_engaging());
        assert!(!state.is_engaged());
    }

    #[test]
    fn engage_now_from_idle_transitions_to_engaging() {
        let mut state = EngagementState::Idle;
        state.engage_now();
        assert!(state.is_engaging());
    }

    #[test]
    fn engage_now_does_not_restart_in_engaging() {
        let mut state = EngagementState::Engaging {
            started_at: Instant::now() - Duration::from_millis(500),
        };
        // Capture the existing progress.
        let before = state.slide_progress();
        state.engage_now();
        let after = state.slide_progress();
        assert!(state.is_engaging());
        // engage_now() on a state that's already Engaging must not reset
        // started_at — the slide progress should not decrease.
        assert!(
            after >= before,
            "engage_now must not restart the slide (before={before}, after={after})"
        );
    }

    #[test]
    fn engage_now_is_noop_when_engaged() {
        let mut state = EngagementState::Engaged;
        state.engage_now();
        assert!(state.is_engaged());
    }

    #[test]
    fn slide_progress_clamped_idle_engaged() {
        assert_eq!(EngagementState::Idle.slide_progress(), 0.0);
        assert_eq!(EngagementState::Engaged.slide_progress(), 1.0);
    }

    #[test]
    fn slide_progress_clamped_for_engaging_past_duration() {
        // Force elapsed well past the slide window; progress must still be 1.0.
        let state = EngagementState::Engaging {
            started_at: Instant::now() - (LOGO_SLIDE_DURATION * 4),
        };
        assert_eq!(state.slide_progress(), 1.0);
    }

    #[test]
    fn slide_progress_stays_in_0_to_1() {
        // Sample a handful of synthetic ages and check the clamp invariant.
        for ms in [0u64, 100, 200, 450, 899, 900, 1500, 5000] {
            let state = EngagementState::Engaging {
                started_at: Instant::now() - Duration::from_millis(ms),
            };
            let p = state.slide_progress();
            assert!(
                (0.0..=1.0).contains(&p),
                "progress {p} out of range at {ms}ms"
            );
        }
    }

    #[test]
    fn tick_promotes_engaging_to_engaged_after_window() {
        let mut state = EngagementState::Engaging {
            started_at: Instant::now() - (LOGO_SLIDE_DURATION + Duration::from_millis(10)),
        };
        state.tick();
        assert!(state.is_engaged());
    }

    #[test]
    fn tick_does_not_promote_while_engaging_active() {
        let mut state = EngagementState::Engaging {
            started_at: Instant::now(),
        };
        state.tick();
        assert!(state.is_engaging());
    }

    #[test]
    fn tick_idle_and_engaged_are_noop() {
        let mut idle = EngagementState::Idle;
        idle.tick();
        assert!(idle.is_idle());

        let mut engaged = EngagementState::Engaged;
        engaged.tick();
        assert!(engaged.is_engaged());
    }
}
