//! Bench: scrolling through 100k transcript events.
//!
//! Hard target (COWBOY.md K2): p95 < 8 ms per scroll-step frame.
//!
//! Approach: synthesize 100k transcript lines, load them into the new cached
//! transcript model, and time the cost of asking for a 40-row viewport at
//! sequential and random scroll offsets.
//!
//! `jekko-tui` is mid-refactor (R3 legacy purge + R4 scrollback rewrite), so
//! this bench is intentionally self-contained: it does not import any
//! renderer functions. Once R4 lands, the synth helper should be swapped to
//! drive the new `Transcript`/`Fenwick` model so this bench measures the
//! real scroll-step path end-to-end.

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use jekko_tui::transcript::Transcript;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

const TOTAL_EVENTS: usize = 100_000;
const VIEWPORT_HEIGHT: usize = 40;

fn synth_user(i: usize) -> Line<'static> {
    Line::from(vec![
        Span::styled("│ ", Style::default().fg(Color::Gray)),
        Span::styled(
            format!("user msg {i}"),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
    ])
}

fn synth_assistant(i: usize) -> Line<'static> {
    Line::from(vec![
        Span::styled("│ ", Style::default().fg(Color::Yellow)),
        Span::styled(
            format!("assistant reply {i}"),
            Style::default().fg(Color::White),
        ),
    ])
}

fn synth_tool_header() -> Line<'static> {
    Line::from(vec![
        Span::styled("●", Style::default().fg(Color::Green)),
        Span::raw(" "),
        Span::styled(
            "Bash",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("(", Style::default().fg(Color::Gray)),
        Span::styled("git status --short", Style::default().fg(Color::White)),
        Span::styled(")", Style::default().fg(Color::Gray)),
    ])
}

fn synth_tool_output_row(text: &str, first: bool) -> Line<'static> {
    let glyph = if first { "  └ " } else { "    " };
    Line::from(vec![
        Span::styled(glyph, Style::default().fg(Color::Gray)),
        Span::styled(text.to_string(), Style::default().fg(Color::White)),
    ])
}

fn synth_diff_header(path: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled("●", Style::default().fg(Color::Green)),
        Span::raw(" "),
        Span::styled(
            "Edit",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("(", Style::default().fg(Color::Gray)),
        Span::styled(path.to_string(), Style::default().fg(Color::Blue)),
        Span::styled(")", Style::default().fg(Color::Gray)),
    ])
}

fn synth_diff_added(lineno: usize, text: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled("  ", Style::default().fg(Color::Gray)),
        Span::styled(
            format!("{lineno:>4}"),
            Style::default().fg(Color::Gray).bg(Color::Black),
        ),
        Span::styled(" ", Style::default().fg(Color::Gray).bg(Color::Black)),
        Span::styled("+", Style::default().fg(Color::Green).bg(Color::Black)),
        Span::styled(" ", Style::default().fg(Color::White).bg(Color::Black)),
        Span::styled(
            text.to_string(),
            Style::default().fg(Color::White).bg(Color::Black),
        ),
    ])
}

fn synth_diff_context(lineno: usize, text: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled("  ", Style::default().fg(Color::Gray)),
        Span::styled(format!("{lineno:>4}"), Style::default().fg(Color::Gray)),
        Span::raw(" "),
        Span::raw(" "),
        Span::raw(" "),
        Span::styled(text.to_string(), Style::default().fg(Color::White)),
    ])
}

fn synthesize_lines() -> Vec<Line<'static>> {
    let mut out: Vec<Line<'static>> = Vec::with_capacity(TOTAL_EVENTS * 4);
    let bash_output = [
        "M  crates/jekko-tui/src/lifecycle.rs",
        "M  crates/jekko-tui/src/theme.rs",
        "?? crates/jekko-tui/benches/scroll_100k.rs",
    ];
    for i in 0..TOTAL_EVENTS {
        match i % 4 {
            0 => out.push(synth_user(i)),
            1 => out.push(synth_assistant(i)),
            2 => {
                out.push(synth_tool_header());
                for (idx, row) in bash_output.iter().enumerate() {
                    out.push(synth_tool_output_row(row, idx == 0));
                }
            }
            _ => {
                out.push(synth_diff_header("crates/foo/src/lib.rs"));
                out.push(synth_diff_context(485, "db,"));
                out.push(synth_diff_added(
                    486,
                    "max_connections: self.max_connections,",
                ));
            }
        }
    }
    out
}

fn bench_scroll(c: &mut Criterion) {
    let lines = synthesize_lines();
    let mut transcript = Transcript::default();
    for line in &lines {
        transcript.push(std::slice::from_ref(line));
    }
    let width = 80u16;
    let total_rows = transcript.row_count(width);

    let mut group = c.benchmark_group("scroll_100k");
    group.throughput(Throughput::Elements(VIEWPORT_HEIGHT as u64));
    group.sample_size(60);

    group.bench_function("scroll_down_40row_viewport", |b| {
        let mut offset = 0usize;
        b.iter(|| {
            let view = transcript.visible_rows(width, VIEWPORT_HEIGHT as u16, offset);
            black_box(view.len());
            for line in &view {
                black_box(line.spans.len());
            }
            offset = (offset + 1) % total_rows.saturating_sub(VIEWPORT_HEIGHT);
        });
    });

    group.bench_function("scroll_random_40row_viewport", |b| {
        let mut seed = 0x9E3779B97F4A7C15u64;
        b.iter(|| {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let offset = (seed as usize) % total_rows.saturating_sub(VIEWPORT_HEIGHT);
            let view = transcript.visible_rows(width, VIEWPORT_HEIGHT as u16, offset);
            black_box(view.len());
            for line in &view {
                black_box(line.spans.len());
            }
        });
    });

    group.finish();
}

criterion_group!(benches, bench_scroll);
criterion_main!(benches);
