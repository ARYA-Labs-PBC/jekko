use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::Terminal;

use crate::theme::codex;

use super::*;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn shift(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::SHIFT)
}

fn sample_lines(n: usize) -> Vec<String> {
    (0..n).map(|i| format!("line {i}")).collect()
}

#[test]
fn scroll_clamps_to_bounds() {
    let mut s = PagerState::new(sample_lines(10));
    s.scroll_up(5);
    assert_eq!(s.scroll, 0, "scroll_up never goes below 0");
    s.scroll_down(100);
    assert_eq!(s.scroll, 9, "scroll_down clamps to len - 1");
    s.scroll_down(50);
    assert_eq!(s.scroll, 9, "further scroll_down stays clamped");

    let mut empty = PagerState::new(Vec::new());
    empty.scroll_down(10);
    assert_eq!(empty.scroll, 0, "empty buffer never moves the cursor");
}

#[test]
fn page_down_advances_by_viewport() {
    let mut s = PagerState::new(sample_lines(100));
    s.page_down(10);
    assert_eq!(s.scroll, 10);
    s.page_down(10);
    assert_eq!(s.scroll, 20);
    s.page_up(5);
    assert_eq!(s.scroll, 15);
}

#[test]
fn home_and_end_jump() {
    let mut s = PagerState::new(sample_lines(50));
    s.scroll_down(20);
    s.home();
    assert_eq!(s.scroll, 0);
    s.end(10);
    assert_eq!(s.scroll, 40);

    let mut shorter = PagerState::new(sample_lines(3));
    shorter.end(10);
    assert_eq!(shorter.scroll, 0, "shorter than viewport clamps to top");
}

#[test]
fn start_search_transitions_mode() {
    let mut s = PagerState::new(sample_lines(5));
    s.search_query = "stale".into();
    s.start_search();
    assert_eq!(s.mode, PagerMode::Search);
    assert_eq!(s.search_query, "", "start_search resets the query");
}

#[test]
fn push_and_pop_search_chars() {
    let mut s = PagerState::new(sample_lines(5));
    s.start_search();
    s.push_search_char('a');
    s.push_search_char('b');
    s.push_search_char('c');
    assert_eq!(s.search_query, "abc");
    s.pop_search_char();
    assert_eq!(s.search_query, "ab");
    s.pop_search_char();
    s.pop_search_char();
    s.pop_search_char();
    assert_eq!(s.search_query, "", "pop on empty is a no-op");
}

#[test]
fn commit_search_finds_matches_and_jumps() {
    let mut s = PagerState::new(vec![
        "alpha bravo".into(),
        "charlie".into(),
        "bravo delta bravo".into(),
    ]);
    s.start_search();
    for c in "bravo".chars() {
        s.push_search_char(c);
    }
    s.commit_search();

    assert_eq!(s.mode, PagerMode::Highlight);
    assert_eq!(s.matches.len(), 3, "found three 'bravo' substrings");
    assert_eq!(
        s.matches[0],
        MatchRef {
            line: 0,
            byte_start: 6,
            byte_end: 11,
        }
    );
    assert_eq!(s.current_match, Some(0));
}

#[test]
fn next_match_wraps_around() {
    let mut s = PagerState::new(vec!["a".into(), "a".into(), "a".into()]);
    s.start_search();
    s.push_search_char('a');
    s.commit_search();
    assert_eq!(s.current_match, Some(0));
    s.next_match(5);
    assert_eq!(s.current_match, Some(1));
    s.next_match(5);
    assert_eq!(s.current_match, Some(2));
    s.next_match(5);
    assert_eq!(s.current_match, Some(0), "wraps from last to first");
}

#[test]
fn prev_match_wraps_around() {
    let mut s = PagerState::new(vec!["a".into(), "a".into(), "a".into()]);
    s.start_search();
    s.push_search_char('a');
    s.commit_search();
    s.prev_match(5);
    assert_eq!(s.current_match, Some(2), "wraps from first to last");
    s.prev_match(5);
    assert_eq!(s.current_match, Some(1));
}

#[test]
fn cancel_search_returns_to_browse() {
    let mut s = PagerState::new(sample_lines(5));
    s.start_search();
    s.push_search_char('x');
    s.cancel_search();
    assert_eq!(s.mode, PagerMode::Browse);
    assert_eq!(s.search_query, "");
    assert!(s.matches.is_empty());
    assert_eq!(s.current_match, None);
}

#[test]
fn yank_current_line_returns_action() {
    let mut s = PagerState::new(sample_lines(10));
    s.scroll_down(3);
    let action = handle_key(&mut s, key(KeyCode::Char('y')), 5);
    assert_eq!(action, PagerAction::Yank("line 3".to_string()));
}

#[test]
fn yank_current_line_empty_pager_returns_none() {
    let mut s = PagerState::new(Vec::new());
    let action = handle_key(&mut s, key(KeyCode::Char('y')), 5);
    assert_eq!(action, PagerAction::None);
}

#[test]
fn yank_current_line_prefers_selected_match() {
    let mut s = PagerState::new(vec![
        "alpha".into(),
        "bravo".into(),
        "charlie".into(),
        "bravo".into(),
    ]);
    s.start_search();
    s.push_search_char('b');
    s.push_search_char('r');
    s.commit_search();
    s.next_match(5);
    let action = handle_key(&mut s, key(KeyCode::Char('y')), 5);
    assert_eq!(action, PagerAction::Yank("bravo".to_string()));
}

#[test]
fn yank_all_returns_action_with_joined_lines() {
    let mut s = PagerState::new(sample_lines(10));
    s.scroll_down(2);
    let action = handle_key(&mut s, key(KeyCode::Char('Y')), 3);
    assert_eq!(
        action,
        PagerAction::Yank("line 2\nline 3\nline 4".to_string()),
        "joins the visible window with newlines"
    );
}

#[test]
fn browse_escape_and_q_request_overlay_close() {
    let mut s = PagerState::new(sample_lines(5));
    let action = handle_key(&mut s, key(KeyCode::Esc), 5);
    assert_eq!(action, PagerAction::Exit);
    let action = handle_key(&mut s, key(KeyCode::Char('q')), 5);
    assert_eq!(action, PagerAction::Exit);
}

#[test]
fn slash_in_browse_enters_search_mode() {
    let mut s = PagerState::new(sample_lines(5));
    let _ = handle_key(&mut s, key(KeyCode::Char('/')), 5);
    assert_eq!(s.mode, PagerMode::Search);
}

#[test]
fn enter_in_search_commits_and_transitions() {
    let mut s = PagerState::new(vec!["foo bar".into()]);
    let _ = handle_key(&mut s, key(KeyCode::Char('/')), 5);
    let _ = handle_key(&mut s, key(KeyCode::Char('b')), 5);
    let _ = handle_key(&mut s, key(KeyCode::Char('a')), 5);
    let _ = handle_key(&mut s, key(KeyCode::Enter), 5);
    assert_eq!(s.mode, PagerMode::Highlight);
    assert_eq!(s.matches.len(), 1);
}

#[test]
fn esc_in_search_cancels_returns_to_browse() {
    let mut s = PagerState::new(sample_lines(3));
    let _ = handle_key(&mut s, key(KeyCode::Char('/')), 5);
    let _ = handle_key(&mut s, key(KeyCode::Char('x')), 5);
    let action = handle_key(&mut s, key(KeyCode::Esc), 5);
    assert_eq!(action, PagerAction::None);
    assert_eq!(s.mode, PagerMode::Browse);
}

#[test]
fn shift_n_in_highlight_navigates_prev() {
    let mut s = PagerState::new(vec!["a".into(), "a".into(), "a".into()]);
    let _ = handle_key(&mut s, key(KeyCode::Char('/')), 5);
    let _ = handle_key(&mut s, key(KeyCode::Char('a')), 5);
    let _ = handle_key(&mut s, key(KeyCode::Enter), 5);
    let _ = handle_key(&mut s, shift(KeyCode::Char('N')), 5);
    assert_eq!(s.current_match, Some(2));
}

fn buffer_to_symbols(buf: &Buffer) -> String {
    let area = buf.area();
    let mut out = String::new();
    for y in 0..area.height {
        for x in 0..area.width {
            let cell = &buf[(area.x + x, area.y + y)];
            out.push_str(cell.symbol());
        }
        if y + 1 < area.height {
            out.push('\n');
        }
    }
    out.lines()
        .map(|l| l.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn render_pager_shows_status_line() {
    let s = PagerState::new(sample_lines(20));
    let backend = TestBackend::new(60, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            render_pager(frame.buffer_mut(), Rect::new(0, 0, 60, 10), &s);
        })
        .unwrap();
    let out = buffer_to_symbols(terminal.backend().buffer());
    let first_line = out.lines().next().unwrap();
    assert!(
        first_line.contains("Pager · 0/20 lines · 0 matches"),
        "got: {first_line:?}"
    );
    assert!(
        first_line.starts_with('─'),
        "starts with rule: {first_line:?}"
    );
}

#[test]
fn render_pager_highlights_current_match() {
    let mut s = PagerState::new(vec!["first".into(), "needle inline".into(), "third".into()]);
    s.start_search();
    for c in "needle".chars() {
        s.push_search_char(c);
    }
    s.commit_search();

    let backend = TestBackend::new(60, 6);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            render_pager(frame.buffer_mut(), Rect::new(0, 0, 60, 6), &s);
        })
        .unwrap();
    let buf = terminal.backend().buffer().clone();
    let out = buffer_to_symbols(&buf);
    assert!(out.contains("1 matches"), "status missing: {out:?}");
    assert!(out.contains("needle inline"), "body missing: {out:?}");

    let y = 2u16;
    let needle_x = 0u16;
    for offset in 0..6 {
        let cell = &buf[(needle_x + offset, y)];
        assert_eq!(
            cell.bg,
            codex::YELLOW,
            "cell ({},{}) symbol={:?} bg={:?} expected YELLOW",
            needle_x + offset,
            y,
            cell.symbol(),
            cell.bg,
        );
    }
}

#[test]
fn render_pager_in_search_mode_shows_prompt() {
    let mut s = PagerState::new(sample_lines(5));
    s.start_search();
    s.push_search_char('n');
    s.push_search_char('e');
    s.push_search_char('e');
    s.push_search_char('d');

    let backend = TestBackend::new(40, 6);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            render_pager(frame.buffer_mut(), Rect::new(0, 0, 40, 6), &s);
        })
        .unwrap();
    let out = buffer_to_symbols(terminal.backend().buffer());
    let last = out.lines().last().unwrap();
    assert!(
        last.starts_with("/ need"),
        "search prompt missing: {last:?}"
    );
    assert!(last.contains('█'), "cursor glyph missing: {last:?}");
}

#[test]
fn render_pager_zero_area_is_noop() {
    let s = PagerState::new(sample_lines(5));
    let backend = TestBackend::new(20, 5);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            render_pager(frame.buffer_mut(), Rect::new(0, 0, 0, 0), &s);
        })
        .unwrap();
    let buf = terminal.backend().buffer();
    for y in 0..5u16 {
        for x in 0..20u16 {
            assert_eq!(buf[(x, y)].symbol(), " ", "({x},{y}) painted");
        }
    }
}

#[test]
fn next_match_with_no_matches_is_noop() {
    let mut s = PagerState::new(sample_lines(5));
    s.next_match(5);
    s.prev_match(5);
    assert!(s.matches.is_empty());
    assert_eq!(s.current_match, None);
}

#[test]
fn selected_match_returns_current() {
    let mut s = PagerState::new(vec!["aaa".into()]);
    s.start_search();
    s.push_search_char('a');
    s.commit_search();
    assert_eq!(s.matches.len(), 3);
    let m = s.selected_match().expect("has selection");
    assert_eq!(m.line, 0);
    assert_eq!(m.byte_start, 0);
}
