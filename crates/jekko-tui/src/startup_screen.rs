use ratatui::layout::Rect;
use ratatui::Frame;

use crate::components::SplashState;

/// Render the loading screen rendered while the app boots.
///
/// Delegates to [`SplashState::render`], which paints the 2-pane NEVERHUMAN
/// streaming splash on wide terminals and falls back to a single-pane log on
/// narrow ones. The `stage` and `log_path` parameters are kept for backwards
/// compatibility with earlier call sites but are no longer surfaced — the
/// production splash communicates progress via the streaming boot log instead.
pub fn draw_startup_screen(
    frame: &mut Frame,
    splash: &SplashState,
    area: Rect,
    _stage: &str,
    _log_path: Option<&str>,
) {
    splash.render(frame, area);
}
