use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use tracing::{error, info};

use crate::action::FIRST_FRAME_WATCHDOG;

/// First-frame watchdog. Mirrors `installFirstFrameWatchdog()` from
/// `packages/jekko/src/cli/cmd/tui/app.tsx`.
///
/// Logs a `tui first frame` info event on the first frame, or an error if no
/// frame is observed within `FIRST_FRAME_WATCHDOG`. Returns an uninstall handle.
pub struct FirstFrameWatchdog {
    seen: Arc<AtomicBool>,
    started_at: Instant,
    cancelled: Arc<AtomicBool>,
}

impl FirstFrameWatchdog {
    pub fn install(started_at: Instant) -> Self {
        let seen = Arc::new(AtomicBool::new(false));
        let cancelled = Arc::new(AtomicBool::new(false));
        let seen_for_thread = Arc::clone(&seen);
        let cancelled_for_thread = Arc::clone(&cancelled);
        thread::spawn(move || {
            let deadline = started_at + FIRST_FRAME_WATCHDOG;
            while Instant::now() < deadline {
                if cancelled_for_thread.load(Ordering::Acquire) {
                    return;
                }
                if seen_for_thread.load(Ordering::Acquire) {
                    return;
                }
                thread::sleep(Duration::from_millis(50));
            }
            if !seen_for_thread.load(Ordering::Acquire)
                && !cancelled_for_thread.load(Ordering::Acquire)
            {
                error!(
                    duration_ms = started_at.elapsed().as_millis() as u64,
                    "tui first frame timeout"
                );
            }
        });
        Self {
            seen,
            started_at,
            cancelled,
        }
    }

    /// Call from inside the first successful draw. Idempotent.
    pub fn mark_seen(&self) {
        if !self.seen.swap(true, Ordering::AcqRel) {
            info!(
                duration_ms = self.started_at.elapsed().as_millis() as u64,
                "tui first frame"
            );
        }
    }

    /// Cancel the watchdog (used on early shutdown). Idempotent.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }
}

impl Drop for FirstFrameWatchdog {
    fn drop(&mut self) {
        self.cancel();
    }
}
