//! Mouse event mapping for the chat transcript.
//!
//! Translates raw `crossterm::event::MouseEvent`s into high-level
//! [`MouseAction`]s consumed by the inline runtime event loop. Keeping the
//! mapping pure (no I/O, no state) means it can be unit-tested without a
//! real TTY.
//!
//! Spec (T2-P5):
//! - Wheel scroll inside the transcript area scrolls 3 lines by default,
//!   10 lines when Shift is held.
//! - Mouse events outside the transcript area are dropped (caller is
//!   responsible for composer click-to-focus etc.).
//! - Left-button down/drag/up emit selection-lifecycle actions; the caller
//!   collects cells and copies on `SelectionEnded`.
//! - Re-engaging stick-to-bottom (when the user scrolls back to within one
//!   row of the bottom) is the caller's responsibility — `map_mouse_event`
//!   only returns the wheel deltas.

use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

/// High-level mouse action emitted by [`map_mouse_event`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseAction {
    /// Event was outside the transcript area or otherwise ignored.
    None,
    /// Scroll the transcript up by N lines.
    ScrollUp(u16),
    /// Scroll the transcript down by N lines.
    ScrollDown(u16),
    /// Caller should toggle stick-to-bottom mode. `true` = re-engage at
    /// bottom; `false` = disengage (user scrolled up).
    ///
    /// `map_mouse_event` itself never emits this — included in the enum so
    /// the runtime wire-up (T2-P5b) can reuse the same dispatch type when
    /// it observes scroll position vs. transcript length.
    StickToBottomToggle(bool),
    /// Left button pressed: start a selection at the cell.
    SelectionStarted { x: u16, y: u16 },
    /// Drag with left button held: extend selection to the cell.
    SelectionDragged { x: u16, y: u16 },
    /// Left button released: commit selection.
    SelectionEnded { x: u16, y: u16 },
}

/// Default wheel scroll step in lines.
pub const WHEEL_LINES_DEFAULT: u16 = 3;
/// Wheel scroll step when Shift is held.
pub const WHEEL_LINES_SHIFTED: u16 = 10;

/// Pure mapping from a raw mouse event to a [`MouseAction`].
///
/// `transcript_area` is the bounding box of the scrollable transcript in
/// terminal cell coordinates. Events outside it return [`MouseAction::None`]
/// so the caller's composer-area handlers can deal with them.
pub fn map_mouse_event(event: MouseEvent, transcript_area: Rect) -> MouseAction {
    if !contains(transcript_area, event.column, event.row) {
        return MouseAction::None;
    }
    let shifted = event.modifiers.contains(KeyModifiers::SHIFT);
    let step = if shifted {
        WHEEL_LINES_SHIFTED
    } else {
        WHEEL_LINES_DEFAULT
    };
    match event.kind {
        MouseEventKind::ScrollUp => MouseAction::ScrollUp(step),
        MouseEventKind::ScrollDown => MouseAction::ScrollDown(step),
        MouseEventKind::Down(MouseButton::Left) => MouseAction::SelectionStarted {
            x: event.column,
            y: event.row,
        },
        MouseEventKind::Drag(MouseButton::Left) => MouseAction::SelectionDragged {
            x: event.column,
            y: event.row,
        },
        MouseEventKind::Up(MouseButton::Left) => MouseAction::SelectionEnded {
            x: event.column,
            y: event.row,
        },
        _ => MouseAction::None,
    }
}

#[inline]
fn contains(rect: Rect, col: u16, row: u16) -> bool {
    if rect.width == 0 || rect.height == 0 {
        return false;
    }
    col >= rect.x
        && col < rect.x.saturating_add(rect.width)
        && row >= rect.y
        && row < rect.y.saturating_add(rect.height)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area() -> Rect {
        Rect::new(0, 0, 80, 24)
    }

    fn mouse(kind: MouseEventKind, column: u16, row: u16, modifiers: KeyModifiers) -> MouseEvent {
        MouseEvent {
            kind,
            column,
            row,
            modifiers,
        }
    }

    #[test]
    fn map_scroll_up_in_transcript_returns_scroll_up_3() {
        let evt = mouse(MouseEventKind::ScrollUp, 5, 5, KeyModifiers::NONE);
        assert_eq!(map_mouse_event(evt, area()), MouseAction::ScrollUp(3));
    }

    #[test]
    fn map_scroll_up_with_shift_returns_scroll_up_10() {
        let evt = mouse(MouseEventKind::ScrollUp, 5, 5, KeyModifiers::SHIFT);
        assert_eq!(map_mouse_event(evt, area()), MouseAction::ScrollUp(10));
    }

    #[test]
    fn map_scroll_down_with_shift_returns_scroll_down_10() {
        let evt = mouse(MouseEventKind::ScrollDown, 5, 5, KeyModifiers::SHIFT);
        assert_eq!(map_mouse_event(evt, area()), MouseAction::ScrollDown(10));
    }

    #[test]
    fn map_scroll_outside_transcript_returns_none() {
        let area = Rect::new(0, 0, 80, 10);
        let evt = mouse(MouseEventKind::ScrollUp, 5, 50, KeyModifiers::NONE);
        assert_eq!(map_mouse_event(evt, area), MouseAction::None);
    }

    #[test]
    fn map_left_button_down_starts_selection() {
        let evt = mouse(
            MouseEventKind::Down(MouseButton::Left),
            7,
            9,
            KeyModifiers::NONE,
        );
        assert_eq!(
            map_mouse_event(evt, area()),
            MouseAction::SelectionStarted { x: 7, y: 9 }
        );
    }

    #[test]
    fn map_drag_returns_selection_dragged() {
        let evt = mouse(
            MouseEventKind::Drag(MouseButton::Left),
            12,
            14,
            KeyModifiers::NONE,
        );
        assert_eq!(
            map_mouse_event(evt, area()),
            MouseAction::SelectionDragged { x: 12, y: 14 }
        );
    }

    #[test]
    fn map_left_button_up_returns_selection_ended() {
        let evt = mouse(
            MouseEventKind::Up(MouseButton::Left),
            20,
            18,
            KeyModifiers::NONE,
        );
        assert_eq!(
            map_mouse_event(evt, area()),
            MouseAction::SelectionEnded { x: 20, y: 18 }
        );
    }

    #[test]
    fn map_right_button_down_returns_none() {
        let evt = mouse(
            MouseEventKind::Down(MouseButton::Right),
            5,
            5,
            KeyModifiers::NONE,
        );
        assert_eq!(map_mouse_event(evt, area()), MouseAction::None);
    }

    #[test]
    fn map_event_at_top_left_corner_is_inside() {
        let evt = mouse(MouseEventKind::ScrollUp, 0, 0, KeyModifiers::NONE);
        assert_eq!(map_mouse_event(evt, area()), MouseAction::ScrollUp(3));
    }

    #[test]
    fn map_event_at_bottom_right_edge_is_outside() {
        // Rect 0,0 width=80 height=24 -> last valid cell is (79, 23).
        let evt = mouse(MouseEventKind::ScrollUp, 80, 23, KeyModifiers::NONE);
        assert_eq!(map_mouse_event(evt, area()), MouseAction::None);
        let evt = mouse(MouseEventKind::ScrollUp, 79, 24, KeyModifiers::NONE);
        assert_eq!(map_mouse_event(evt, area()), MouseAction::None);
    }

    #[test]
    fn zero_sized_area_drops_everything() {
        let area = Rect::new(10, 10, 0, 0);
        let evt = mouse(MouseEventKind::ScrollUp, 10, 10, KeyModifiers::NONE);
        assert_eq!(map_mouse_event(evt, area), MouseAction::None);
    }

    #[test]
    fn wheel_lines_constants_are_sane() {
        const {
            assert!(WHEEL_LINES_DEFAULT > 0);
            assert!(WHEEL_LINES_SHIFTED > WHEEL_LINES_DEFAULT);
        }
    }
}
