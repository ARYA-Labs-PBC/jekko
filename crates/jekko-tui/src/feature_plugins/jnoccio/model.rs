//! Core data types for the Jnoccio Fusion panel.
//!
//! Holds the tab enum, connection state, snapshot struct, and the colour /
//! sort-mode constants shared by the rendering helpers. Split out of the
//! original single-file module so each concern lives in ~150 LOC.

use ratatui::style::Color;

/// One Jnoccio dashboard tab. Numeric shortcut keys map 1..=6 onto these in
/// declaration order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JnoccioTab {
    /// Leaderboard tab — top models by score.
    Board,
    /// Speed tab — latency histogram.
    Speed,
    /// Token vault tab — token budget burn-down.
    Vault,
    /// Limits tab — rate limits and capacity.
    Limits,
    /// Live event feed tab.
    Feed,
    /// Active agent roster tab.
    Agents,
}

impl JnoccioTab {
    /// All tabs in their display order. Position is also the (1-indexed)
    /// keyboard shortcut.
    pub const ALL: &'static [JnoccioTab] = &[
        JnoccioTab::Board,
        JnoccioTab::Speed,
        JnoccioTab::Vault,
        JnoccioTab::Limits,
        JnoccioTab::Feed,
        JnoccioTab::Agents,
    ];

    /// Short display label.
    pub fn label(self) -> &'static str {
        match self {
            JnoccioTab::Board => "Board",
            JnoccioTab::Speed => "Speed",
            JnoccioTab::Vault => "Vault",
            JnoccioTab::Limits => "Limits",
            JnoccioTab::Feed => "Feed",
            JnoccioTab::Agents => "Agents",
        }
    }

    /// Single-character icon shown next to the label.
    pub fn icon(self) -> &'static str {
        match self {
            JnoccioTab::Board => "T",
            JnoccioTab::Speed => "S",
            JnoccioTab::Vault => "V",
            JnoccioTab::Limits => "L",
            JnoccioTab::Feed => "F",
            JnoccioTab::Agents => "A",
        }
    }

    /// 1-indexed shortcut digit.
    pub fn shortcut(self) -> char {
        match self {
            JnoccioTab::Board => '1',
            JnoccioTab::Speed => '2',
            JnoccioTab::Vault => '3',
            JnoccioTab::Limits => '4',
            JnoccioTab::Feed => '5',
            JnoccioTab::Agents => '6',
        }
    }

    /// 0-indexed offset into [`Self::ALL`].
    pub fn index(self) -> usize {
        Self::ALL.iter().position(|t| *t == self).unwrap_or(0)
    }

    /// Empty-state copy shown when a tab has no data to display.
    pub fn empty_state_label(self) -> &'static str {
        match self {
            JnoccioTab::Board => "No runs yet.",
            JnoccioTab::Speed => "No latency data yet.",
            JnoccioTab::Vault => "No token data yet.",
            JnoccioTab::Limits => "Connect to view limits.",
            JnoccioTab::Feed => "No events yet.",
            JnoccioTab::Agents => "No active agents.",
        }
    }
}

/// Connection state for the Jnoccio WS feed. Mirrors the
/// `connection` memo in `dashboard.tsx`.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum JnoccioConnection {
    /// No data and no connection attempt yet.
    #[default]
    Loading,
    /// Boot probe in progress.
    Connecting,
    /// WS open, fresh heartbeat within tolerance.
    Live,
    /// WS or boot probe failed.
    Error,
}

impl JnoccioConnection {
    pub(super) fn label(self) -> &'static str {
        match self {
            JnoccioConnection::Loading => "Loading...",
            JnoccioConnection::Connecting => "Connecting...",
            JnoccioConnection::Live => "Live",
            JnoccioConnection::Error => "Error",
        }
    }

    #[allow(dead_code)]
    pub(super) fn color(self) -> Color {
        match self {
            JnoccioConnection::Loading => MUTED,
            JnoccioConnection::Connecting => PINK,
            JnoccioConnection::Live => GREEN,
            JnoccioConnection::Error => RED,
        }
    }
}

/// Snapshot data passed to the panel. The original TS panel pulls these
/// fields from `useJnoccioSnapshot()`; this static shape lets us render the
/// chrome without a live WS feed.
#[derive(Clone, Debug, Default)]
pub struct JnoccioSnapshot {
    /// Models that have reported at least one call.
    pub enabled_models: u32,
    /// Total registered models.
    pub total_models: u32,
    /// Active agent count.
    pub agents: u32,
    /// Max concurrent agents allowed.
    pub max_agents: u32,
    /// Number of gateway instances.
    pub instances: u32,
    /// Aggregate calls across all models.
    pub calls: u64,
    /// Aggregate successful responses.
    pub wins: u64,
    /// Aggregate failures.
    pub failures: u64,
    /// Aggregate token usage.
    pub total_tokens: u64,
    /// Median 24h token rate (in millions/day).
    pub tokens_per_24h_m: f64,
    /// Mean latency in milliseconds.
    pub avg_latency_ms: f64,
    /// Capacity used as a 0..1 ratio.
    pub capacity_used: f64,
}

pub(super) const GOLD: Color = Color::Rgb(0xf5, 0xa6, 0x23);
pub(super) const MUTED: Color = Color::Rgb(0x7d, 0x85, 0x90);
pub(super) const TEXT: Color = Color::Rgb(0xd8, 0xde, 0xe9);
pub(super) const RED: Color = Color::Rgb(0xff, 0x47, 0x57);
pub(super) const GREEN: Color = Color::Rgb(0x22, 0xc5, 0x5e);
pub(super) const PINK: Color = Color::Rgb(0xff, 0x00, 0xb8);
pub(super) const BG: Color = Color::Rgb(0x0b, 0x0f, 0x14);

pub(super) const SORT_MODES: &[&str] = &["latest", "wins", "score", "latency"];
