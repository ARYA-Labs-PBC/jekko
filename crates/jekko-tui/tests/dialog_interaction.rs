//! Behavioral tests for dialog widgets that don't need a Ratatui buffer.

use jekko_tui::{CommandEntry, CommandPalette, Dialog, DialogStack, SelectDialog, SelectOption};

#[test]
fn select_cursor_wraps_in_both_directions() {
    let mut d = SelectDialog::new(
        "Theme",
        vec![
            SelectOption::new("a", "A"),
            SelectOption::new("b", "B"),
            SelectOption::new("c", "C"),
        ],
    );
    d.move_cursor(1);
    assert_eq!(d.selected().unwrap().id, "b");
    d.move_cursor(1);
    assert_eq!(d.selected().unwrap().id, "c");
    d.move_cursor(1);
    assert_eq!(d.selected().unwrap().id, "a");
    d.move_cursor(-1);
    assert_eq!(d.selected().unwrap().id, "c");
}

#[test]
fn select_cursor_stays_valid_for_empty_lists() {
    let mut d = SelectDialog::new("Empty", vec![]);
    d.move_cursor(1);
    d.move_cursor(-1);
    assert!(d.selected().is_none());
    assert!(d.cursor_is_valid());
}

#[test]
fn select_cursor_input_boundary_is_hardened() {
    // Repeated moves on an empty list must not panic and must keep cursor==0.
    let mut empty = SelectDialog::new("Empty", vec![]);
    for _ in 0..5 {
        empty.move_cursor(1);
    }
    assert_eq!(empty.cursor, 0);
    assert!(empty.cursor_is_valid());

    // Repeated forward moves on a 3-option list must wrap (existing
    // convention) and stay in-range after far more moves than options.
    let mut d = SelectDialog::new(
        "Three",
        vec![
            SelectOption::new("a", "A"),
            SelectOption::new("b", "B"),
            SelectOption::new("c", "C"),
        ],
    );
    for _ in 0..10 {
        d.move_cursor(1);
    }
    // 10 forward steps on 3 options wraps to index 1 (10 % 3 == 1).
    assert_eq!(d.cursor, 1);
    assert!(d.cursor_is_valid());

    // Extreme deltas must not overflow or loop unboundedly.
    d.move_cursor(isize::MAX);
    assert!(d.cursor_is_valid());
    d.move_cursor(isize::MIN);
    assert!(d.cursor_is_valid());

    // Tampered out-of-range cursor recovers on next move and via set_cursor.
    d.cursor = 9999;
    d.move_cursor(0);
    assert!(d.cursor_is_valid());
    d.set_cursor(9999);
    assert_eq!(d.cursor, 2);
    d.set_cursor(0);
    assert_eq!(d.cursor, 0);

    // set_cursor on empty pins to 0.
    let mut empty2 = SelectDialog::new("Empty", vec![]);
    empty2.set_cursor(42);
    assert_eq!(empty2.cursor, 0);
}

#[test]
fn select_dialog_strips_control_chars_and_truncates_text() {
    let d = SelectDialog::new(
        "Controls",
        vec![SelectOption::new("id", "abc\u{0007}def").with_hint("x".repeat(200))],
    );
    let rendered = {
        // Exercise the internal sanitizer through a render pass.
        use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};
        let mut buf = Buffer::empty(Rect::new(0, 0, 80, 20));
        (&d).render(Rect::new(0, 0, 80, 20), &mut buf);
        buf
    };
    let text = rendered
        .content
        .into_iter()
        .map(|cell| cell.symbol().to_string())
        .collect::<String>();
    assert!(!text.contains('\u{0007}'));
    assert!(!text.contains('\u{0000}'));
}

#[test]
fn command_palette_filters_case_insensitive() {
    let mut p = CommandPalette::new(vec![
        CommandEntry::new("session.new", "New session"),
        CommandEntry::new("model.list", "Model picker"),
        CommandEntry::new("theme.list", "Theme picker"),
    ]);
    p.type_char('M');
    p.type_char('O');
    p.type_char('D');
    let visible: Vec<_> = p.visible().iter().map(|e| e.id.clone()).collect();
    assert_eq!(visible, vec!["model.list"]);
}

#[test]
fn command_palette_filter_resets_cursor() {
    let mut p = CommandPalette::new(vec![
        CommandEntry::new("a", "alpha"),
        CommandEntry::new("b", "beta"),
        CommandEntry::new("c", "gamma"),
    ]);
    p.move_cursor(2);
    assert_eq!(p.selected().unwrap().id, "c");
    p.type_char('b');
    assert_eq!(p.selected().unwrap().id, "b");
    assert_eq!(p.cursor, 0);
}

#[test]
fn command_palette_no_match_yields_none() {
    let mut p = CommandPalette::new(vec![CommandEntry::new("a", "alpha")]);
    p.type_char('z');
    assert!(p.selected().is_none());
    assert!(p.visible().is_empty());
}

#[test]
fn dialog_stack_top_returns_last_pushed() {
    let mut stack = DialogStack::default();
    assert!(stack.is_empty());
    stack.push(Dialog::Select(SelectDialog::new(
        "First",
        vec![SelectOption::new("x", "X")],
    )));
    stack.push(Dialog::Command(CommandPalette::new(vec![
        CommandEntry::new("y", "Y"),
    ])));
    assert_eq!(stack.len(), 2);
    match stack.top().unwrap() {
        Dialog::Command(_) => {}
        Dialog::Select(_) => panic!("expected Command on top"),
    }
    stack.pop();
    match stack.top().unwrap() {
        Dialog::Select(s) => assert_eq!(s.title, "First"),
        Dialog::Command(_) => panic!("expected Select on top"),
    }
}

#[test]
fn dialog_footprint_reports_widget_size() {
    let select = Dialog::Select(SelectDialog::new("S", vec![SelectOption::new("a", "A")]));
    let cmd = Dialog::Command(CommandPalette::new(vec![]));
    let (w_s, h_s) = select.footprint();
    let (w_c, h_c) = cmd.footprint();
    assert_eq!(w_s, 60);
    assert!(h_s > 0);
    assert_eq!((w_c, h_c), (64, 18));
}
