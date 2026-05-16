//! ZYAL neon palette constants.
//!
//! Mirrors `feature-plugins/sidebar/zyal-palette.ts` so the panel reads as one
//! continuous identity with the sidebar widget.

use ratatui::style::Color;

pub(super) const NEON_LOOPS: Color = Color::Rgb(0xff, 0x40, 0xff);
pub(super) const NEON_TOKENS_TOTAL: Color = Color::Rgb(0x00, 0xff, 0xff);
pub(super) const NEON_TOKENS_IN: Color = Color::Rgb(0xff, 0xd0, 0x00);
pub(super) const NEON_TOKENS_OUT: Color = Color::Rgb(0x00, 0xff, 0x87);
pub(super) const NEON_CACHE: Color = Color::Rgb(0x7d, 0xf9, 0xff);
pub(super) const NEON_WORKERS_ACTIVE: Color = Color::Rgb(0xbb, 0xff, 0x00);
pub(super) const NEON_WORKERS_MAX: Color = Color::Rgb(0xa0, 0xa0, 0xa0);
pub(super) const NEON_UPTIME: Color = Color::Rgb(0xff, 0xd0, 0x00);
pub(super) const NEON_COST: Color = Color::Rgb(0xff, 0x14, 0x93);
pub(super) const NEON_CALLS: Color = Color::Rgb(0x00, 0xff, 0xff);
pub(super) const NEON_WINS: Color = Color::Rgb(0x00, 0xff, 0x87);
pub(super) const NEON_FAILS: Color = Color::Rgb(0xff, 0x40, 0x60);
pub(super) const NEON_LATENCY: Color = Color::Rgb(0xff, 0x99, 0x33);
pub(super) const NEON_HEARTBEAT_LIVE: Color = Color::Rgb(0x00, 0xff, 0x87);
pub(super) const NEON_HEARTBEAT_STALE: Color = Color::Rgb(0xff, 0x40, 0x60);
pub(super) const NEON_SEPARATOR: Color = Color::Rgb(0xa0, 0x60, 0x30);

pub(super) const TEXT: Color = Color::Rgb(0xd8, 0xde, 0xe9);
pub(super) const MUTED: Color = Color::Rgb(0x7d, 0x85, 0x90);

/// Default status label when the live snapshot lacks one.
pub(super) const DEFAULT_STATUS_LABEL: &str = "active";
pub(super) const GOLD: Color = Color::Rgb(0xf5, 0xa6, 0x23);
pub(super) const SUCCESS: Color = Color::Rgb(0x22, 0xc5, 0x5e);
pub(super) const WARNING: Color = Color::Rgb(0xff, 0xd0, 0x00);
pub(super) const ERROR: Color = Color::Rgb(0xff, 0x40, 0x60);

// Keep the heartbeat colors referenced when the status block is compiled out.
#[allow(dead_code)]
pub(super) const _PALETTE_REF: [Color; 4] = [
    NEON_HEARTBEAT_LIVE,
    NEON_HEARTBEAT_STALE,
    NEON_CALLS,
    NEON_WINS,
];

/// Compact thousand/million/billion formatter shared by the panel.
pub(super) fn fmt_n(n: u64) -> String {
    if n >= 1_000_000_000 {
        format!("{:.1}B", n as f64 / 1_000_000_000.0)
    } else if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}
