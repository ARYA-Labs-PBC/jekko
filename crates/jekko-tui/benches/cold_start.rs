//! Bench: cold-start cost — time to produce the first renderable frame.
//!
//! Hard target (COWBOY.md K4): cold start < 80 ms. Today this measures the
//! cost of building the boot block + first frame's worth of placeholder lines.
//! Once chat_runtime exposes a headless `boot()` API, swap the synth helper
//! to drive that instead so this measures the actual cold-start path.

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

fn build_first_frame() -> Vec<Line<'static>> {
    let mut lines = Vec::with_capacity(8);
    // 4-line boot block (top rule + header + bottom rule + hint).
    let rule = "─".repeat(80);
    lines.push(Line::from(Span::styled(
        rule.clone(),
        Style::default().fg(Color::Rgb(0x6a, 0x6c, 0x6e)),
    )));
    lines.push(Line::from(vec![
        Span::styled("⚡ ", Style::default().fg(Color::Rgb(0xff, 0xb0, 0x45))),
        Span::styled("JEKKO", Style::default().fg(Color::Rgb(0xff, 0xff, 0xff))),
        Span::styled(" v0.1.0", Style::default().fg(Color::Rgb(0x89, 0x89, 0x89))),
        Span::styled(" · ", Style::default().fg(Color::Rgb(0x75, 0x75, 0x75))),
        Span::styled(
            "~/code/jekko",
            Style::default().fg(Color::Rgb(0x9a, 0xd3, 0xff)),
        ),
        Span::styled(" · ", Style::default().fg(Color::Rgb(0x75, 0x75, 0x75))),
        Span::styled(
            "branch: main",
            Style::default().fg(Color::Rgb(0x00, 0xd7, 0xdf)),
        ),
    ]));
    lines.push(Line::from(Span::styled(
        rule,
        Style::default().fg(Color::Rgb(0x6a, 0x6c, 0x6e)),
    )));
    lines.push(Line::from(Span::styled(
        " ready — type / for commands, @ to mention files",
        Style::default().fg(Color::Rgb(0x89, 0x89, 0x89)),
    )));
    // Composer chrome (top rule + composer + bottom rule + shortcuts).
    let composer_rule = "─".repeat(80);
    lines.push(Line::from(Span::styled(
        composer_rule.clone(),
        Style::default().fg(Color::Rgb(0x6a, 0x6c, 0x6e)),
    )));
    lines.push(Line::from(vec![Span::styled(
        "› ",
        Style::default().fg(Color::Rgb(0xff, 0xb0, 0x45)),
    )]));
    lines.push(Line::from(Span::styled(
        composer_rule,
        Style::default().fg(Color::Rgb(0x6a, 0x6c, 0x6e)),
    )));
    lines.push(Line::from(vec![
        Span::styled(
            " / commands  ",
            Style::default().fg(Color::Rgb(0x89, 0x89, 0x89)),
        ),
        Span::styled(
            "@ files  ",
            Style::default().fg(Color::Rgb(0x89, 0x89, 0x89)),
        ),
        Span::styled("⏎ send", Style::default().fg(Color::Rgb(0x89, 0x89, 0x89))),
    ]));
    lines
}

fn bench_cold(c: &mut Criterion) {
    let mut group = c.benchmark_group("cold_start");
    group.sample_size(50);

    group.bench_function("build_first_frame", |b| {
        b.iter(|| {
            let lines = build_first_frame();
            black_box(lines.len());
            for line in &lines {
                black_box(line.spans.len());
            }
        });
    });

    group.finish();
}

criterion_group!(benches, bench_cold);
criterion_main!(benches);
