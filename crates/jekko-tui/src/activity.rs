//! Shared long-running activity tracker used by the TUI chrome and live feed.

use std::time::Instant;

use ratatui::style::Color;

use crate::theme;

/// Kinds of active operations we surface in the prompt sweep and activity
/// feed.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ActivityKind {
    Model,
    Reasoning,
    JankuraiAudit,
    JankuraiCycle,
    Bash,
    Agent,
    Jnoccio,
    Zyal,
}

impl ActivityKind {
    /// Stable label used in compact activity rows.
    pub fn label(self) -> &'static str {
        match self {
            ActivityKind::Model => "model",
            ActivityKind::Reasoning => "reasoning",
            ActivityKind::JankuraiAudit => "audit",
            ActivityKind::JankuraiCycle => "cycle",
            ActivityKind::Bash => "bash",
            ActivityKind::Agent => "agent",
            ActivityKind::Jnoccio => "jnoccio",
            ActivityKind::Zyal => "zyal",
        }
    }

    /// Accent color used for the bottom sweep and compact feed rails.
    pub fn accent(self) -> Color {
        match self {
            ActivityKind::Model | ActivityKind::Reasoning => theme::INFO,
            ActivityKind::JankuraiAudit | ActivityKind::JankuraiCycle => theme::WARNING,
            ActivityKind::Bash => Color::Rgb(0x7c, 0xc4, 0x78),
            ActivityKind::Agent => Color::Rgb(0xb4, 0x8d, 0xf6),
            ActivityKind::Jnoccio => Color::Rgb(0xd8, 0x72, 0xf0),
            ActivityKind::Zyal => Color::Rgb(0x7b, 0xe0, 0xd4),
        }
    }
}

/// One active operation tracked by the app.
#[derive(Clone, Debug)]
pub struct ActiveOperation {
    pub id: String,
    pub kind: ActivityKind,
    pub label: String,
    pub started_at: Instant,
    pub last_update: Instant,
    pub progress: Option<(u64, u64)>,
    pub status: Option<String>,
}

impl ActiveOperation {
    fn new(id: String, kind: ActivityKind, label: String) -> Self {
        let now = Instant::now();
        Self {
            id,
            kind,
            label,
            started_at: now,
            last_update: now,
            progress: None,
            status: None,
        }
    }
}

/// Active-operation tracker used by the shell prompt sweep and activity rows.
#[derive(Clone, Debug, Default)]
pub struct ActivityTracker {
    active: Vec<ActiveOperation>,
}

impl ActivityTracker {
    /// True when at least one operation is still running.
    pub fn any_running(&self) -> bool {
        !self.active.is_empty()
    }

    /// Number of active operations.
    pub fn len(&self) -> usize {
        self.active.len()
    }

    /// Best-effort primary kind for the prompt sweep.
    pub fn primary_kind(&self) -> Option<ActivityKind> {
        self.active.last().map(|op| op.kind)
    }

    /// Start or replace an operation.
    pub fn start(&mut self, id: impl Into<String>, kind: ActivityKind, label: impl Into<String>) {
        let id = id.into();
        let label = label.into();
        if let Some(existing) = self.active.iter_mut().find(|op| op.id == id) {
            existing.kind = kind;
            existing.label = label;
            existing.last_update = Instant::now();
            existing.progress = None;
            existing.status = None;
            return;
        }
        self.active.push(ActiveOperation::new(id, kind, label));
    }

    /// Update a running operation, starting it if needed.
    pub fn update(
        &mut self,
        id: impl Into<String>,
        kind: ActivityKind,
        label: Option<String>,
        status: Option<String>,
        progress: Option<(u64, u64)>,
    ) {
        let id = id.into();
        if let Some(existing) = self.active.iter_mut().find(|op| op.id == id) {
            existing.kind = kind;
            if let Some(label) = label {
                existing.label = label;
            }
            existing.status = status;
            existing.progress = progress;
            existing.last_update = Instant::now();
            return;
        }
        let mut op = ActiveOperation::new(id, kind, label.unwrap_or_else(|| kind.label().into()));
        op.status = status;
        op.progress = progress;
        self.active.push(op);
    }

    /// Finish an operation and return it if it existed.
    pub fn finish(
        &mut self,
        id: impl Into<String>,
        kind: ActivityKind,
        label: Option<String>,
        status: Option<String>,
    ) -> Option<ActiveOperation> {
        let id = id.into();
        let idx = self.active.iter().position(|op| op.id == id)?;
        let mut op = self.active.remove(idx);
        op.kind = kind;
        if let Some(label) = label {
            op.label = label;
        }
        op.status = status;
        op.last_update = Instant::now();
        Some(op)
    }
}
