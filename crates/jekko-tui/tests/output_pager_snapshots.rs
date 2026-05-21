//! COWBOY T2-P3 — golden snapshots for the pager renderer.
//!
//! Two fixtures at 60×10:
//! - `pager_browse_mode` — fresh pager with no search, viewport at the top.
//! - `pager_highlight_mode` — after committing `/bravo` so we render with
//!   `Highlight` mode active.
//!
//! Both fixtures strip styling via [`buffer_to_symbols`]; colour/bg checks
//! live in `components::output_pager::tests` to keep these snapshots stable
//! when the palette changes.

use insta::assert_snapshot;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::Terminal;

use jekko_tui::components::output_pager::{render_pager, PagerState};

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

fn fixture_lines() -> Vec<String> {
    vec![
        "alpha line zero".into(),
        "beta line one with bravo inside".into(),
        "charlie line two".into(),
        "delta line three has bravo too".into(),
        "echo line four".into(),
        "foxtrot line five".into(),
        "golf line six".into(),
        "hotel line seven".into(),
    ]
}

fn render_snapshot(state: &PagerState) -> String {
    let backend = TestBackend::new(60, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            render_pager(frame.buffer_mut(), Rect::new(0, 0, 60, 10), state);
        })
        .unwrap();
    buffer_to_symbols(terminal.backend().buffer())
}

#[test]
fn pager_browse_mode_60x10() {
    let state = PagerState::new(fixture_lines());
    assert_snapshot!("pager_browse_mode_60x10", render_snapshot(&state));
}

#[test]
fn pager_highlight_mode_60x10() {
    let mut state = PagerState::new(fixture_lines());
    state.start_search();
    for c in "bravo".chars() {
        state.push_search_char(c);
    }
    state.commit_search();
    assert_snapshot!("pager_highlight_mode_60x10", render_snapshot(&state));
}
