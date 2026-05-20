//! Bench: append-throughput on a growing transcript.
//!
//! Hard target (COWBOY.md K3): appending 10k events produces no frame > 16 ms
//! after batching. This bench measures the per-event push cost on a `Vec<Line>`
//! at scale — once the real `Transcript` model lands in R4, swap the `Vec`
//! for the new structure.

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

const TOTAL_EVENTS: usize = 10_000;

fn make_event(i: usize) -> Vec<Line<'static>> {
    let prefix = Span::styled("│ ", Style::default().fg(Color::Rgb(0x89, 0x89, 0x89)));
    let body = format!("event {i}: streamed line of moderate length so wrapping matters");
    vec![Line::from(vec![
        prefix,
        Span::styled(body, Style::default().fg(Color::Rgb(0xd7, 0xd7, 0xd7))),
    ])]
}

fn bench_append(c: &mut Criterion) {
    let mut group = c.benchmark_group("append_10k");
    group.throughput(Throughput::Elements(TOTAL_EVENTS as u64));
    group.sample_size(20);

    group.bench_function("push_10k_events", |b| {
        b.iter(|| {
            let mut store: Vec<Line<'static>> = Vec::with_capacity(TOTAL_EVENTS);
            for i in 0..TOTAL_EVENTS {
                let mut lines = make_event(i);
                store.append(&mut lines);
            }
            black_box(store.len());
        });
    });

    group.bench_function("push_10k_with_intermittent_slice", |b| {
        b.iter(|| {
            let mut store: Vec<Line<'static>> = Vec::with_capacity(TOTAL_EVENTS);
            for i in 0..TOTAL_EVENTS {
                let mut lines = make_event(i);
                store.append(&mut lines);
                if i % 100 == 99 {
                    // Simulate the render path reading the tail every 100 events.
                    let start = store.len().saturating_sub(40);
                    for line in &store[start..] {
                        black_box(line.spans.len());
                    }
                }
            }
            black_box(store.len());
        });
    });

    group.finish();
}

criterion_group!(benches, bench_append);
criterion_main!(benches);
