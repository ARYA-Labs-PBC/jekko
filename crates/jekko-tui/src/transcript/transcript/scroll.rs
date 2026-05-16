//! Scroll state, intent, and key-hold acceleration.

use std::time::{Duration, Instant};

/// Direction the scroll handlers acknowledge.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ScrollIntent {
    /// Scroll up by some delta.
    Up,
    /// Scroll down by some delta.
    Down,
}

/// Sticky-bottom indicator state.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct ScrollState {
    /// Current scroll offset (rows from the top).
    pub offset: u16,
    /// True while the viewport is pinned to the bottom of the transcript.
    pub sticky_bottom: bool,
    /// Last viewport height observed.
    pub viewport_rows: u16,
}

impl Default for ScrollState {
    fn default() -> Self {
        Self {
            offset: 0,
            sticky_bottom: true,
            viewport_rows: 0,
        }
    }
}

/// Hold-down acceleration. Mirrors the JS `getScrollAcceleration` curve:
/// consecutive ticks within the velocity window grow the step from 1 row
/// toward a soft cap.
#[derive(Clone, Debug)]
pub struct ScrollAcceleration {
    /// Latest velocity (rows per scroll event).
    pub velocity: u16,
    /// Last event time.
    pub last_tick: Option<Instant>,
    /// Max rows allowed per single scroll event.
    pub max_velocity: u16,
    /// Time window in which a held key counts as a streak.
    pub window: Duration,
}

impl Default for ScrollAcceleration {
    fn default() -> Self {
        Self {
            velocity: 1,
            last_tick: None,
            max_velocity: 12,
            window: Duration::from_millis(180),
        }
    }
}

impl ScrollAcceleration {
    /// Record one tick now and return the resulting velocity.
    pub fn tick(&mut self) -> u16 {
        self.tick_at(Instant::now())
    }

    /// Record one tick at `now` (test-friendly). Within the streak window the
    /// velocity grows; outside it the curve resets to 1.
    pub fn tick_at(&mut self, now: Instant) -> u16 {
        match self.last_tick {
            Some(last) if now.duration_since(last) <= self.window => {
                self.velocity = (self.velocity + 1).min(self.max_velocity);
            }
            _ => {
                self.velocity = 1;
            }
        }
        self.last_tick = Some(now);
        self.velocity
    }

    /// Reset to a cold-start step of 1.
    pub fn reset(&mut self) {
        self.velocity = 1;
        self.last_tick = None;
    }
}
