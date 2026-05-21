//! Multi-agent rail data model (COWBOY.md L1, per tips/fucktui/tip9.txt).
//!
//! `AgentRun` represents one running local-or-remote agent shown in the rail
//! below the composer. `AgentPanelState` is the registry the chat runtime
//! mutates as agents start/stop.

pub mod panel;

use std::time::{Duration, Instant};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AgentId(pub String);

impl AgentId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentKind {
    Main,
    GeneralPurpose,
    Build,
    Review,
    Patch,
    Worker,
    Custom(String),
}

impl AgentKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Main => "main",
            Self::GeneralPurpose => "general-purpose",
            Self::Build => "build",
            Self::Review => "review",
            Self::Patch => "patch",
            Self::Worker => "worker",
            Self::Custom(_) => "custom",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentStatus {
    Queued,
    Running,
    Waiting,
    Idle,
    Done,
    Failed,
    Cancelled,
}

impl AgentStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Waiting => "waiting",
            Self::Idle => "idle",
            Self::Done => "done",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentLocality {
    Local,
    Remote,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TokenStats {
    pub input: u64,
    pub output: u64,
}

impl TokenStats {
    pub fn total(self) -> u64 {
        self.input + self.output
    }

    pub fn add_input(&mut self, n: u64) {
        self.input = self.input.saturating_add(n);
    }

    pub fn add_output(&mut self, n: u64) {
        self.output = self.output.saturating_add(n);
    }
}

#[derive(Clone, Debug)]
pub struct AgentRun {
    pub id: AgentId,
    pub name: String,
    pub kind: AgentKind,
    pub summary: String,
    pub status: AgentStatus,
    pub locality: AgentLocality,
    pub started_at: Instant,
    pub last_active_at: Instant,
    pub tokens: TokenStats,
}

impl AgentRun {
    pub fn new_main(name: impl Into<String>, summary: impl Into<String>) -> Self {
        // WHY: collapse `name` to String once so we can derive both the id and
        // the display name from a single owned value (Into<String> consumes).
        let name = name.into();
        let now = Instant::now();
        Self {
            id: AgentId::new(name.clone()),
            name,
            kind: AgentKind::Main,
            summary: summary.into(),
            status: AgentStatus::Running,
            locality: AgentLocality::Local,
            started_at: now,
            last_active_at: now,
            tokens: TokenStats::default(),
        }
    }

    pub fn label(&self) -> &'static str {
        self.kind.clone().label()
    }

    pub fn runtime(&self, now: Instant) -> Duration {
        now.saturating_duration_since(self.started_at)
    }
}

#[derive(Default, Debug)]
pub struct AgentPanelState {
    pub agents: Vec<AgentRun>,
    pub selected_index: usize,
    pub scroll_offset: usize,
    pub max_visible_rows: usize,
    pub visible: bool,
    pub focused: bool,
}

impl AgentPanelState {
    pub fn new() -> Self {
        Self {
            agents: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
            max_visible_rows: 8,
            visible: true,
            focused: false,
        }
    }

    pub fn upsert(&mut self, agent: AgentRun) {
        if let Some(existing) = self.agents.iter_mut().find(|a| a.id == agent.id) {
            *existing = agent;
        } else {
            self.agents.push(agent);
        }
        self.clamp_selection();
    }

    pub fn local_running_count(&self) -> usize {
        self.agents
            .iter()
            .filter(|a| a.locality == AgentLocality::Local && a.status == AgentStatus::Running)
            .count()
    }

    pub fn selected_agent(&self) -> Option<&AgentRun> {
        self.agents.get(self.selected_index)
    }

    pub fn set_focus(&mut self, focused: bool) {
        self.focused = focused;
        if focused {
            self.clamp_selection();
        }
    }

    pub fn set_viewport_rows(&mut self, rows: usize) {
        self.max_visible_rows = rows.max(1);
        self.clamp_selection();
    }

    pub fn select_next(&mut self) {
        if self.agents.is_empty() {
            self.selected_index = 0;
            return;
        }
        if !self.focused {
            self.focused = true;
            self.selected_index = 0;
            self.sync_scroll();
            return;
        }
        self.selected_index = (self.selected_index + 1).min(self.agents.len().saturating_sub(1));
        self.sync_scroll();
    }

    pub fn select_prev(&mut self) {
        if self.agents.is_empty() {
            self.selected_index = 0;
            return;
        }
        if !self.focused {
            self.focused = true;
            self.selected_index = 0;
            self.sync_scroll();
            return;
        }
        self.selected_index = self.selected_index.saturating_sub(1);
        self.sync_scroll();
    }

    pub fn sync_scroll(&mut self) {
        self.clamp_selection();
        let visible_agents = self.visible_agent_slots();
        if visible_agents == 0 || self.agents.is_empty() {
            self.scroll_offset = 0;
            return;
        }
        let top = self.scroll_offset;
        let bottom = top.saturating_add(visible_agents);
        if self.selected_index < top {
            self.scroll_offset = self.selected_index;
        } else if self.selected_index >= bottom {
            self.scroll_offset = self
                .selected_index
                .saturating_add(1)
                .saturating_sub(visible_agents);
        }
        let max_top = self.agents.len().saturating_sub(visible_agents);
        self.scroll_offset = self.scroll_offset.min(max_top);
    }

    pub fn visible_agent_slots(&self) -> usize {
        self.max_visible_rows.saturating_sub(1) / 2
    }

    fn clamp_selection(&mut self) {
        if self.agents.is_empty() {
            self.selected_index = 0;
            self.scroll_offset = 0;
            return;
        }
        if self.selected_index >= self.agents.len() {
            self.selected_index = self.agents.len() - 1;
        }
        let max_top = self.agents.len().saturating_sub(self.visible_agent_slots());
        self.scroll_offset = self.scroll_offset.min(max_top);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration as StdDuration;

    #[test]
    fn local_running_count() {
        let mut p = AgentPanelState::new();
        p.agents.push(AgentRun::new_main("main", "do thing"));
        let mut idle = AgentRun::new_main("other", "idle thing");
        idle.status = AgentStatus::Idle;
        p.agents.push(idle);
        assert_eq!(p.local_running_count(), 1);
    }

    #[test]
    fn upsert_replaces_by_id() {
        let mut p = AgentPanelState::new();
        let mut a = AgentRun::new_main("main", "v1");
        a.id = AgentId::new("same");
        p.upsert(a);
        let mut a2 = AgentRun::new_main("main2", "v2");
        a2.id = AgentId::new("same");
        a2.status = AgentStatus::Done;
        p.upsert(a2);
        assert_eq!(p.agents.len(), 1);
        assert_eq!(p.agents[0].status, AgentStatus::Done);
        assert_eq!(p.agents[0].summary, "v2");
    }

    #[test]
    fn select_next_prev_clamps() {
        let mut p = AgentPanelState::new();
        for n in 0..3 {
            let mut a = AgentRun::new_main(format!("a{n}"), "x");
            a.id = AgentId::new(format!("a{n}"));
            p.agents.push(a);
        }
        p.select_next();
        assert_eq!(p.selected_agent().unwrap().id.as_str(), "a0");
        p.select_next();
        assert_eq!(p.selected_agent().unwrap().id.as_str(), "a1");
        p.select_prev();
        assert_eq!(p.selected_agent().unwrap().id.as_str(), "a0");
        p.select_prev();
        assert_eq!(p.selected_agent().unwrap().id.as_str(), "a0"); // clamped
    }

    #[test]
    fn token_stats_total() {
        let mut t = TokenStats::default();
        t.add_input(10);
        t.add_output(25);
        assert_eq!(t.total(), 35);
    }

    #[test]
    fn runtime_uses_started_at() {
        let a = AgentRun::new_main("a", "x");
        let now = a.started_at + StdDuration::from_secs(7);
        assert_eq!(a.runtime(now), StdDuration::from_secs(7));
    }

    #[test]
    fn set_focus_picks_first_when_no_selection() {
        let mut p = AgentPanelState::new();
        p.agents.push(AgentRun::new_main("only", "x"));
        p.set_focus(true);
        assert!(p.focused);
        assert_eq!(p.selected_agent().unwrap().id.as_str(), "only");
    }

    #[test]
    fn visible_slots_reflect_rows() {
        let mut p = AgentPanelState::new();
        p.set_viewport_rows(7);
        assert_eq!(p.visible_agent_slots(), 3);
    }
}
