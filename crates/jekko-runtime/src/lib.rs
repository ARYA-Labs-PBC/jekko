//! Jekko runtime services (Phase 4 port of the JS runtime layer).
//!
//! This crate hosts the in-process services that turn the pure-data types
//! exported by [`jekko_core`] into a working agent runtime:
//!
//! - [`bus`] — async pub/sub over typed events.
//! - [`project`] / [`workspace`] — project and workspace state.
//! - [`auth`] / [`account`] — credential storage.
//! - [`permission`] / [`question`] — permission/question flows.
//! - [`mod@file`] / [`watcher`] / [`ripgrep`] — filesystem services.
//! - [`lsp`] / [`mcp`] — language-server and MCP integration shells.
//! - [`shell`] / [`pty`] — process spawning helpers.
//! - [`session`] / [`message`] / [`prompt`] / [`processor`] / [`status`]
//!   / [`compaction`] — session lifecycle.
//! - [`agent`] — one-shot runtime entrypoint and workflow request/response
//!   boundary.
//! - [`daemon`] — daemon orchestration scaffolding.
//! - [`tool`] — the tool trait + ported tool implementations.
//! - [`skill`] — skill loader.
//! - [`snapshot`] — git-aware workspace snapshotter.
//!
//! Internal API may diverge from the TypeScript runtime; what must stay
//! observable from the outside (DB rows, bus events, tool schemas,
//! permission decisions, session state-machine transitions) is preserved.
#![deny(rust_2018_idioms)]
#![warn(missing_docs)]

pub mod account;
pub mod agent;
pub mod auth;
pub mod bus;
pub mod compaction;
pub mod daemon;
pub mod daemon_transport;
pub mod error;
pub mod file;
pub mod key_balancer;
pub mod lsp;
pub mod mcp;
pub mod message;
pub mod permission;
pub mod processor;
pub mod project;
pub mod prompt;
pub mod pty;
pub mod question;
pub mod ripgrep;
pub mod session;
pub mod shell;
pub mod skill;
pub mod snapshot;
pub mod status;
pub mod tool;
pub mod watcher;
pub mod workspace;

pub use agent::{
    AgentExecutor, AgentTurnRequest, AgentTurnResult, ProviderAgentExecutor, RunRequest, RunResult,
};
pub use bus::{Bus, BusEvent, EventEnvelope};
pub use error::{RuntimeError, RuntimeResult};
pub use permission::{PermissionDecision, PermissionRequest, PermissionService};
pub use session::{CreateSessionInput, SessionService};
pub use status::{Status, StatusService};
pub use tool::{Tool, ToolContext, ToolOutput};

// ─────────────────────────────────────────────────────────────────────────────
// Top-level entry type
// ─────────────────────────────────────────────────────────────────────────────

use std::sync::Arc;

/// Runtime root: bundles the long-lived services for a single project
/// instance. Construct via [`Runtime::new`]; clone the handle freely.
#[derive(Clone)]
pub struct Runtime {
    /// Shared event bus.
    pub bus: Arc<Bus>,
    /// Permission service.
    pub permissions: Arc<PermissionService>,
    /// Session service.
    pub sessions: Arc<SessionService>,
    /// Daemon registry.
    pub daemons: Arc<daemon::DaemonRegistry>,
    /// Session status tracker.
    pub status: Arc<StatusService>,
    /// Agent executor used for one-shot runs.
    pub agent_executor: Arc<dyn AgentExecutor>,
}

impl Runtime {
    /// Construct a runtime with default in-memory services.
    pub fn new() -> Self {
        let bus = Arc::new(Bus::new());
        let permissions = Arc::new(PermissionService::new(bus.clone()));
        let status = Arc::new(StatusService::new(bus.clone()));
        let sessions = Arc::new(SessionService::with_bus(bus.clone()));
        let daemons = daemon::DaemonRegistry::with_bus(bus.clone());
        let agent_executor = Arc::new(crate::agent::ProviderAgentExecutor::new(
            permissions.clone(),
            sessions.clone(),
        ));
        Self {
            bus,
            permissions,
            sessions,
            daemons,
            status,
            agent_executor,
        }
    }

    /// Construct a runtime with a caller-supplied agent executor.
    pub fn with_agent_executor(agent_executor: Arc<dyn AgentExecutor>) -> Self {
        let bus = Arc::new(Bus::new());
        let permissions = Arc::new(PermissionService::new(bus.clone()));
        let status = Arc::new(StatusService::new(bus.clone()));
        let sessions = Arc::new(SessionService::with_bus(bus.clone()));
        let daemons = daemon::DaemonRegistry::with_bus(bus.clone());
        Self {
            bus,
            permissions,
            sessions,
            daemons,
            status,
            agent_executor,
        }
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for Runtime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Runtime")
            .field("bus", &self.bus)
            .field("permissions", &self.permissions)
            .field("sessions", &self.sessions)
            .field("daemons", &self.daemons)
            .field("status", &self.status)
            .field("agent_executor", &"<dyn AgentExecutor>")
            .finish()
    }
}
