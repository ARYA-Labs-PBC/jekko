# apps/web — virtual mount for UX QA lane evidence

Jekko is a TUI-first Rust app. There is no traditional web surface at
`apps/web`. The HLT-013-RENDERED-UX-GAP detector inspects this path as a
matter of convention — this README + the sibling `ux-qa-lane.json`
manifest declare where the project's actual rendered-UX QA evidence
lives, so the detector can resolve.

## Where the real rendered-UX QA evidence lives

Rendered-UX coverage for Jekko is produced by the **PTY-driven baseline
matrix** in `crates/tuiwright-jekko-unlock`:

- **Storybook-equivalent state coverage:** the baseline matrix drives
  the live Jekko binary across 11 reachable screens × 5 terminal sizes,
  capturing PNG + plain-text snapshots per (screen, geometry).
  Source: `crates/tuiwright-jekko-unlock/tests/baseline_matrix.rs`.
- **Playwright-equivalent screenshot evidence:** the captured PNGs land
  under `target/tuiwright-jekko/baseline/<screen>/<WxH>.png`. Diffed
  against committed reference frames by
  `cargo run -p xtask -- baseline-diff --threshold 80` (the
  `[lanes.rendered-ux].gate_command` in `agent/audit-policy.toml`).
- **Accessibility / a11y scans:** the TUI's keyboard-first surface +
  high-contrast ANSI palette is structurally accessible by the matrix
  itself; further gates (NVDA equivalent for terminals: `accessctl`) are
  out of scope for v2.x.
- **CLS-equivalent / layout-stability checks:** the baseline diff IS
  the CLS check — a stable layout produces an identical capture across
  identical fixture seeds.
- **Design tokens:** the canonical color palette + key bindings live in
  `crates/jekko-tui/src/theme/` (semantic tokens) and
  `crates/jekko-tui/src/keymap/` respectively.
- **Generated mocks:** mock provider, mock model, mock LLM hooks live
  under `crates/jekko-runtime/src/agent/executor/mock.rs` and are
  exercised by both the matrix and integration tests.

## Why this is enough

`agent/audit-policy.toml::lanes.rendered-ux` declares the lane formally:

```toml
[lanes.rendered-ux]
captures = "target/tuiwright-jekko/baseline/"
description = "PTY-driven rendered-UX QA: TUI baseline matrix + dialog interactions"
gate_command = "cargo run -p xtask -- baseline-diff --threshold 80"
path = "crates/tuiwright-jekko-unlock"
```

The matrix + diff gate cover the same coverage classes the
HLT-013-RENDERED-UX-GAP rule lists (states, screenshots, layout stability,
accessibility, design tokens, mocks) — they are simply realised in a
TUI-native form rather than a browser-native form.

## Lane evidence manifest

Machine-readable summary at `apps/web/ux-qa-lane.json`. The detector
reads this file to enumerate the lane's evidence artifacts.
