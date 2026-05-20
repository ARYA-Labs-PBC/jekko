//! Bench: idle-render cost.
//!
//! Hard target (COWBOY.md K5): idle CPU ~0% (< 0.5% per-frame budget at 30 FPS).
//! With a dirty-flag scheduler this should be ~zero cost because we skip
//! `terminal.draw` when `dirty == false`. This bench measures the cost of
//! the early-return path through the render scheduler so any regression in
//! the dirty-check is visible. Once chat_runtime exposes a public
//! `draw_if_dirty(...)` API, swap the synth helper for it.

use std::hint::black_box;
use std::time::Duration;

use criterion::{criterion_group, criterion_main, Criterion};

/// Minimal stand-in for the render-loop dirty-flag check. Caller flips
/// `dirty` in response to events; the scheduler returns early when clean.
struct DirtyScheduler {
    dirty: bool,
    frames_rendered: u64,
}

impl DirtyScheduler {
    fn tick(&mut self) {
        if self.dirty {
            self.dirty = false;
            self.frames_rendered += 1;
        }
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
    }
}

fn bench_idle(c: &mut Criterion) {
    let mut group = c.benchmark_group("idle_cpu");
    group.sample_size(50);
    group.measurement_time(Duration::from_secs(3));

    group.bench_function("dirty_check_only", |b| {
        let mut s = DirtyScheduler {
            dirty: false,
            frames_rendered: 0,
        };
        b.iter(|| {
            for _ in 0..30 {
                s.tick();
            }
            black_box(s.frames_rendered);
        });
    });

    group.bench_function("dirty_with_periodic_mark", |b| {
        let mut s = DirtyScheduler {
            dirty: false,
            frames_rendered: 0,
        };
        b.iter(|| {
            // Simulate one second of idle wall-clock — 30 ticks, mark dirty
            // every 10 ticks (3 redraws per second is a reasonable "barely
            // anything is happening" baseline).
            for i in 0..30 {
                if i % 10 == 0 {
                    s.mark_dirty();
                }
                s.tick();
            }
            black_box(s.frames_rendered);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_idle);
criterion_main!(benches);
