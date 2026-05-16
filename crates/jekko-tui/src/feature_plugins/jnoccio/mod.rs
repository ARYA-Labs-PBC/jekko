//! Jnoccio Fusion dashboard panel.
//!
//! Ports `packages/jekko/src/cli/cmd/tui/feature-plugins/jnoccio/` —
//! `dashboard.tsx`, `dashboard-view.tsx`, `state.ts`, `help-overlay.tsx`,
//! tabs 1-6, model status row, server health pill — to a Ratatui widget.
//!
//! Live snapshot data is intentionally static ([`JnoccioSnapshot`]) so this
//! panel compiles without the backend running. Once the orchestrator wires
//! the WS feed into the App, callers will pass a populated snapshot into
//! [`JnoccioPanel::set_snapshot`].
//!
//! The panel owns its local UI state (selected tab, paused flag, search
//! buffer, help overlay, sort mode, drawer cursor) and responds to keystrokes
//! through [`JnoccioPanel::dispatch_key`].
//!
//! Identical semantics to the original TS component: keys `1`-`6` jump to
//! a tab directly, `Tab`/`Right` cycle forward, `Shift+Tab`/`Left` cycle back,
//! `j`/`k`/`Down`/`Up` move the cursor, `g`/`G` jump to top/bottom, `?` toggles
//! help, `/` opens search, `Esc`/`q` exit.
//!
//! The implementation is split per-seam under [`jnoccio`](self) submodules
//! ([`model`] for data types and palette, [`panel`] for the panel struct and
//! state methods, [`keys`] for the key dispatch hierarchy, [`render`] for the
//! Ratatui `Widget` impl and number formatters). Public types are re-exported
//! here so the original `jekko_tui::feature_plugins::jnoccio::*` import paths
//! continue to work unchanged.

mod keys;
mod model;
mod panel;
mod render;

#[cfg(test)]
mod tests;

pub use model::{JnoccioConnection, JnoccioSnapshot, JnoccioTab};
pub use panel::JnoccioPanel;
