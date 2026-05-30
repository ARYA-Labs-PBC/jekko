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
//! - [`session_budget`] — typed Rust client (ARY-2306) for the QO
//!   policy-sidecar's `session_ping` / `session_checkpoint` verbs over
//!   AARA MCP.
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
pub mod autonomy;
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
pub mod session_budget;
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
pub use autonomy::{AutonomyConfig, AutonomyDeny, AutonomyError};
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
    /// Autonomy boundaries loaded from `agent/boundaries.toml` at
    /// runtime init. Falls back to safe defaults if the file is missing
    /// or malformed (see [`AutonomyConfig::load_default`]). Wired here
    /// per ARY-2303 as the one representative load site; future change
    /// efforts will plumb [`AutonomyConfig::is_prohibited`] into each
    /// agent-initiated decision surface.
    pub autonomy: Arc<AutonomyConfig>,
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
        let autonomy = Arc::new(AutonomyConfig::load_default());
        Self {
            bus,
            permissions,
            sessions,
            daemons,
            status,
            agent_executor,
            autonomy,
        }
    }

    /// Construct a runtime with a caller-supplied agent executor.
    pub fn with_agent_executor(agent_executor: Arc<dyn AgentExecutor>) -> Self {
        let bus = Arc::new(Bus::new());
        let permissions = Arc::new(PermissionService::new(bus.clone()));
        let status = Arc::new(StatusService::new(bus.clone()));
        let sessions = Arc::new(SessionService::with_bus(bus.clone()));
        let daemons = daemon::DaemonRegistry::with_bus(bus.clone());
        let autonomy = Arc::new(AutonomyConfig::load_default());
        Self {
            bus,
            permissions,
            sessions,
            daemons,
            status,
            agent_executor,
            autonomy,
        }
    }

    /// Reusable per-surface autonomy gate (ARY-2305).
    ///
    /// Returns `Ok(())` when `action` is NOT on the
    /// `prohibited_autonomous_actions` list and `Err(AutonomyDeny)` when it
    /// is. Every new agent-initiated decision surface in this crate (e.g.
    /// "launch a training run", "create or resize a VM", "push to HF or
    /// GCS") MUST call this gate at the top of the function and propagate
    /// the deny as a typed error.
    ///
    /// ## Why this is a Runtime method and not a free function
    ///
    /// Future surfaces will likely need to thread the *full* autonomy
    /// budget — checkpoint timing, actions remaining — alongside the
    /// prohibition check. Centralising the gate here lets us add those
    /// concerns without rewriting every callsite when ARY-2306 wires the
    /// session-budget MCP client and ARY-2308 exposes the QO tool surface.
    ///
    /// ## Why no surfaces are gated by per-surface wiring yet
    ///
    /// As of ARY-2305 the Jekko runtime does not host any operation that
    /// maps to the 7 `prohibited_autonomous_actions` labels in
    /// `agent/boundaries.toml` (`launch_training_run`,
    /// `synthesize_atoms`, `push_to_hf_or_gcs`, `create_or_resize_vm`,
    /// `run_gp_search`, `modify_atoms_bundled`,
    /// `run_multi_hour_experiment`). The tool surfaces this crate ships
    /// (bash/read/write/edit/grep/glob/webfetch/websearch) live one layer
    /// below the prohibited-action labels, which describe *agent
    /// intentions* not *file ops*. Per-surface wiring lands as those
    /// intention-level surfaces are built. Until then this helper is the
    /// reference call-shape any future surface should mirror, plus the
    /// single integration point QO sidecar callers (ARY-2306) hit when
    /// they want to refuse before crossing the IPC boundary.
    pub fn gate_action(&self, action: &str) -> Result<(), AutonomyDeny> {
        if self.autonomy.is_prohibited(action) {
            Err(AutonomyDeny::prohibited(action))
        } else {
            Ok(())
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
            .field("autonomy", &self.autonomy)
            .finish()
    }
}

#[cfg(test)]
mod gate_action_tests {
    use super::*;
    use crate::autonomy::AutonomyConfig;

    /// Build a Runtime with a hand-crafted [`AutonomyConfig`] so the test
    /// does not depend on the on-disk `agent/boundaries.toml`.
    fn runtime_with(config: AutonomyConfig) -> Runtime {
        let mut rt = Runtime::new();
        rt.autonomy = Arc::new(config);
        rt
    }

    #[test]
    fn gate_action_allows_when_action_not_prohibited() {
        let cfg = AutonomyConfig {
            prohibited_autonomous_actions: vec!["launch_training_run".into()],
            ..AutonomyConfig::defaults()
        };
        let rt = runtime_with(cfg);
        // A label that isn't on the list is allowed.
        assert!(rt.gate_action("read_file").is_ok());
        assert!(rt.gate_action("").is_ok());
    }

    #[test]
    fn gate_action_denies_when_action_prohibited() {
        let cfg = AutonomyConfig {
            prohibited_autonomous_actions: vec![
                "launch_training_run".into(),
                "push_to_hf_or_gcs".into(),
            ],
            ..AutonomyConfig::defaults()
        };
        let rt = runtime_with(cfg);
        let err = rt.gate_action("launch_training_run").unwrap_err();
        assert_eq!(err.action, "launch_training_run");
        assert_eq!(err.reason, AutonomyDeny::REASON_PROHIBITED);

        let err2 = rt.gate_action("push_to_hf_or_gcs").unwrap_err();
        assert_eq!(err2.action, "push_to_hf_or_gcs");
    }

    #[test]
    fn gate_action_is_case_sensitive() {
        // Mirrors AutonomyConfig::is_prohibited semantics — labels match
        // the TOML key exactly, no normalisation.
        let cfg = AutonomyConfig {
            prohibited_autonomous_actions: vec!["launch_training_run".into()],
            ..AutonomyConfig::defaults()
        };
        let rt = runtime_with(cfg);
        assert!(rt.gate_action("LAUNCH_TRAINING_RUN").is_ok());
        assert!(rt.gate_action("launch_training_run").is_err());
    }

    #[test]
    fn gate_action_with_empty_prohibition_list_always_allows() {
        let rt = runtime_with(AutonomyConfig::defaults());
        // Every one of the 7 canonical labels passes — there's no policy
        // to enforce when the list is empty.
        for action in [
            "launch_training_run",
            "synthesize_atoms",
            "push_to_hf_or_gcs",
            "create_or_resize_vm",
            "run_gp_search",
            "modify_atoms_bundled",
            "run_multi_hour_experiment",
        ] {
            assert!(
                rt.gate_action(action).is_ok(),
                "action {action} should be allowed when no prohibitions are configured"
            );
        }
    }
}
