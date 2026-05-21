//! Cancellation state machine (COWBOY.md F5).
//!
//! Captures the user's escalating intent ("interrupt this", "really stop",
//! "kill it now") without coupling to any specific runner. The state machine
//! transitions on every Esc/Ctrl+C key + on `/stop` slash command. Runners
//! poll `desired_signal()` periodically and act on the result.
//!
//! Esc                       → Soft  (SIGINT)
//! Esc within 2s of prior    → Hard  (SIGTERM)
//! `/stop` slash command     → Hard immediately
//! 5s after Hard, no exit    → Force (SIGKILL)
//!
//! Tests cover the timing arithmetic; the actual signal delivery happens in
//! the runners (a few lines each).

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

const ESC_DOUBLE_TAP_WINDOW: Duration = Duration::from_millis(2_000);
const HARD_TO_FORCE_GRACE: Duration = Duration::from_millis(5_000);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CancelLevel {
    None = 0,
    Soft = 1,
    Hard = 2,
    Force = 3,
}

impl From<u8> for CancelLevel {
    fn from(v: u8) -> Self {
        match v {
            1 => Self::Soft,
            2 => Self::Hard,
            3 => Self::Force,
            _ => Self::None,
        }
    }
}

/// Lock-free shared cancellation handle. Cloneable; all clones see the same
/// level. Runners hold a `CancellationToken` and poll `level()` between
/// reads. UI holds an [`Escalator`] and calls `on_esc`/`on_stop`.
#[derive(Clone, Debug, Default)]
pub struct CancellationToken {
    level: Arc<AtomicU8>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn level(&self) -> CancelLevel {
        self.level.load(Ordering::SeqCst).into()
    }

    pub fn is_cancelled(&self) -> bool {
        self.level() != CancelLevel::None
    }

    pub(crate) fn raise_to(&self, level: CancelLevel) {
        let v = level as u8;
        // Monotonic raise — never downgrade.
        let _ = self
            .level
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |cur| {
                if v > cur {
                    Some(v)
                } else {
                    None
                }
            });
    }

    /// Reset to None — used when starting a new turn.
    pub fn reset(&self) {
        self.level.store(0, Ordering::SeqCst);
    }

    /// Shorthand for raising the token directly to `Hard` — useful for
    /// programmatic cancellation (e.g. `/stop` slash command without a
    /// keyboard escalator) and for runner tests.
    pub fn cancel_hard(&self) {
        self.raise_to(CancelLevel::Hard);
    }
}

/// UI-side escalator. Track last Esc time so a double-tap auto-promotes.
/// Owns its own clock for tests via `escalate_at`/`stop_at` helpers.
pub struct Escalator {
    token: CancellationToken,
    last_esc_at: Option<Instant>,
    hard_at: Option<Instant>,
}

impl Escalator {
    pub fn new(token: CancellationToken) -> Self {
        Self {
            token,
            last_esc_at: None,
            hard_at: None,
        }
    }

    pub fn on_esc(&mut self) -> CancelLevel {
        self.on_esc_at(Instant::now())
    }

    pub fn on_esc_at(&mut self, now: Instant) -> CancelLevel {
        let level = match self.last_esc_at {
            Some(prev) if now.duration_since(prev) <= ESC_DOUBLE_TAP_WINDOW => CancelLevel::Hard,
            _ => CancelLevel::Soft,
        };
        self.last_esc_at = Some(now);
        if level == CancelLevel::Hard {
            self.hard_at = Some(now);
        }
        self.token.raise_to(level);
        level
    }

    pub fn on_stop(&mut self) -> CancelLevel {
        self.on_stop_at(Instant::now())
    }

    pub fn on_stop_at(&mut self, now: Instant) -> CancelLevel {
        self.hard_at = Some(now);
        self.token.raise_to(CancelLevel::Hard);
        CancelLevel::Hard
    }

    /// Called on every render tick. If the child has been Hard-cancelled
    /// longer than `HARD_TO_FORCE_GRACE`, escalates to Force.
    pub fn tick(&mut self) -> CancelLevel {
        self.tick_at(Instant::now())
    }

    pub fn tick_at(&mut self, now: Instant) -> CancelLevel {
        if let Some(hard) = self.hard_at {
            if now.duration_since(hard) >= HARD_TO_FORCE_GRACE {
                self.token.raise_to(CancelLevel::Force);
            }
        }
        self.token.level()
    }

    pub fn token(&self) -> CancellationToken {
        self.token.clone()
    }

    pub fn reset(&mut self) {
        self.token.reset();
        self.last_esc_at = None;
        self.hard_at = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_token_is_none() {
        let t = CancellationToken::new();
        assert_eq!(t.level(), CancelLevel::None);
        assert!(!t.is_cancelled());
    }

    #[test]
    fn token_clones_share_state() {
        let a = CancellationToken::new();
        let b = a.clone();
        a.raise_to(CancelLevel::Soft);
        assert_eq!(b.level(), CancelLevel::Soft);
    }

    #[test]
    fn raise_is_monotonic() {
        let t = CancellationToken::new();
        t.raise_to(CancelLevel::Hard);
        t.raise_to(CancelLevel::Soft); // downgrade attempt
        assert_eq!(t.level(), CancelLevel::Hard);
    }

    #[test]
    fn first_esc_is_soft() {
        let mut e = Escalator::new(CancellationToken::new());
        let now = Instant::now();
        assert_eq!(e.on_esc_at(now), CancelLevel::Soft);
    }

    #[test]
    fn double_esc_within_window_is_hard() {
        let mut e = Escalator::new(CancellationToken::new());
        let t0 = Instant::now();
        let t1 = t0 + Duration::from_millis(500);
        assert_eq!(e.on_esc_at(t0), CancelLevel::Soft);
        assert_eq!(e.on_esc_at(t1), CancelLevel::Hard);
    }

    #[test]
    fn double_esc_outside_window_stays_soft() {
        let mut e = Escalator::new(CancellationToken::new());
        let t0 = Instant::now();
        let t1 = t0 + Duration::from_millis(3_000);
        assert_eq!(e.on_esc_at(t0), CancelLevel::Soft);
        assert_eq!(e.on_esc_at(t1), CancelLevel::Soft);
    }

    #[test]
    fn stop_is_hard_immediately() {
        let mut e = Escalator::new(CancellationToken::new());
        assert_eq!(e.on_stop_at(Instant::now()), CancelLevel::Hard);
    }

    #[test]
    fn tick_escalates_to_force_after_grace() {
        let mut e = Escalator::new(CancellationToken::new());
        let t0 = Instant::now();
        e.on_stop_at(t0);
        let t1 = t0 + Duration::from_millis(2_000);
        assert_eq!(e.tick_at(t1), CancelLevel::Hard);
        let t2 = t0 + Duration::from_millis(5_500);
        assert_eq!(e.tick_at(t2), CancelLevel::Force);
    }

    #[test]
    fn reset_returns_to_none() {
        let mut e = Escalator::new(CancellationToken::new());
        e.on_stop();
        assert_eq!(e.token().level(), CancelLevel::Hard);
        e.reset();
        assert_eq!(e.token().level(), CancelLevel::None);
    }

    #[test]
    fn level_ordering_holds() {
        assert!((CancelLevel::Soft as u8) < (CancelLevel::Hard as u8));
        assert!((CancelLevel::Hard as u8) < (CancelLevel::Force as u8));
    }
}
