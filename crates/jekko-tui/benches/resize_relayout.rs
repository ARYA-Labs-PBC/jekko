//! Bench: terminal-resize relayout cost.
//!
//! Hard target (COWBOY.md K6): < 50 ms to rewrap on terminal-resize.
//! Today this measures the cost of rewrapping a `Vec<String>` event log to a
//! new width using a simple word-wrap pass. Once the real `Transcript`
//! model exists (R4), the lazy-rewrap path can be benched end-to-end here.

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

const TOTAL_EVENTS: usize = 5_000;

fn synthesize_events() -> Vec<String> {
    (0..TOTAL_EVENTS)
        .map(|i| {
            format!(
                "event {i:05}: streamed line of moderate length that crosses common terminal widths so the wrap pass has to do work {i}",
            )
        })
        .collect()
}

fn rewrap_to_width(events: &[String], width: usize) -> Vec<String> {
    let mut out = Vec::with_capacity(events.len() * 2);
    for ev in events {
        let mut cursor = 0;
        while cursor < ev.len() {
            let end = (cursor + width).min(ev.len());
            out.push(ev[cursor..end].to_string());
            cursor = end;
        }
    }
    out
}

fn bench_resize(c: &mut Criterion) {
    let events = synthesize_events();

    let mut group = c.benchmark_group("resize_relayout");
    group.throughput(Throughput::Elements(TOTAL_EVENTS as u64));
    group.sample_size(20);

    for &width in &[60usize, 80, 100, 120, 160, 200] {
        group.bench_with_input(BenchmarkId::from_parameter(width), &width, |b, &w| {
            b.iter(|| {
                let wrapped = rewrap_to_width(&events, w);
                black_box(wrapped.len());
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_resize);
criterion_main!(benches);
