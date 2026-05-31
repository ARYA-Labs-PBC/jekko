//! Session-budget MCP client skeleton (ARY-2306).
//!
//! The QO sidecar at `apps/safety_kernel/policy_sidecar.py` exposes two
//! per-session budget verbs over its Unix-domain socket today:
//!
//! - `session_ping` — agent calls this before each major action; sidecar
//!   debits the action budget and may demand a checkpoint.
//! - `session_checkpoint` — user provides a confirmation token to reset
//!   the budget.
//!
//! Per ADR-020, Jekko is NOT allowed to touch the kernel's UDS directly.
//! The cross-process boundary from Jekko goes through AARA MCP. This
//! module ships the typed Rust client with wire shapes that match the QO
//! handlers (`_handle_session_ping` / `_handle_session_checkpoint`) and a
//! [`McpTransport`] trait so the *actual* transport implementation
//! (Anthropic SDK MCP, custom JSON-RPC adapter, in-process mock, …) can
//! be plugged in independently.
//!
//! ## Scope for this PR (ARY-2306)
//!
//! - Request / response structs.
//! - [`McpTransport`] trait + [`SessionBudgetClient`] using it.
//! - [`McpError`] hierarchy with a typed `Unreachable` variant the
//!   fail-open path can match on.
//! - Tests against a [`MockMcpTransport`].
//!
//! Exposing `aara_session_ping` / `aara_session_checkpoint` as MCP tools
//! on the QO side is intentionally out of scope; that work is filed
//! separately as ARY-2308 (or noted as TBD-upstream in the PR body if the
//! ticket has not been opened yet).
//!
//! ## Fail-open semantics
//!
//! [`SessionBudgetClient::session_ping`] mirrors QO
//! `packages/safety/autonomy.py::_sidecar_ping` behaviour: when the
//! transport returns [`McpError::Unreachable`], the client does NOT
//! propagate the error. Instead it returns a synthetic
//! `SessionPingResponse { allowed: true, reason:
//! Some("sidecar_unreachable_fail_open") }` and logs a WARN.
//!
//! Rationale: the kernel `pre_launch_check` is still the *authoritative*
//! gate on the QO side. The sidecar ping is an optimisation that lets
//! callers refuse early without crossing the kernel boundary; refusing
//! every action when the sidecar is briefly unreachable would be a
//! denial-of-service against the cognitive loop. Fail-safe on the
//! inference path matches `docs/architecture/safety-kernel-consolidation.md`.
//!
//! Checkpoint failures are NOT fail-open — a missing sidecar genuinely
//! cannot reset budget, so [`SessionBudgetClient::session_checkpoint`]
//! propagates the error.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;

/// Canonical MCP tool name for the `session_ping` verb.
///
/// Exposed as a constant so future renames stay lockstep with the QO
/// side. ARY-2308 (or its successor) will register the same string as the
/// tool name in `apps/mcp_server/tools/handlers.py`.
pub const TOOL_SESSION_PING: &str = "aara_session_ping";

/// Canonical MCP tool name for the `session_checkpoint` verb.
pub const TOOL_SESSION_CHECKPOINT: &str = "aara_session_checkpoint";

/// Stable reason string set on a synthetic ping response when the
/// transport is unreachable. Matches the QO-side audit-log marker so
/// downstream correlation works without a translation table.
pub const REASON_SIDECAR_UNREACHABLE_FAIL_OPEN: &str = "sidecar_unreachable_fail_open";

/// Wire-shape request for `aara_session_ping`.
///
/// Field names mirror the JSON payload accepted by
/// `policy_sidecar.py::_handle_session_ping` so serialising this struct
/// produces a payload the QO sidecar can read byte-for-byte.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionPingRequest {
    /// Opaque session identifier minted by the runtime / orchestrator.
    pub session_id: String,
    /// Action label the agent is about to attempt. Recorded for audit
    /// even though `_handle_session_ping` does not currently key the
    /// budget on it. Matches the `action_type` field in QO callers.
    pub action_type: String,
    /// Wall-clock timestamp (seconds since UNIX epoch, float for sub-
    /// second precision). The sidecar uses its own clock so this is
    /// audit/correlation only — but it ships in the same JSON shape QO
    /// emits.
    pub timestamp: f64,
}

/// Wire-shape response for `aara_session_ping`.
///
/// Field names + optionality mirror the dict returned by
/// `policy_sidecar.py::_handle_session_ping`. A budget-exhausted /
/// checkpoint-required response sets `allowed = false` and populates
/// `reason` + `checkpoint_token_required`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionPingResponse {
    /// `true` when the action may proceed.
    pub allowed: bool,
    /// Remaining action budget after this ping was debited. May be
    /// absent when the response is a refusal that did not touch the
    /// counter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actions_remaining: Option<i32>,
    /// Minutes remaining until the next mandatory checkpoint. Float to
    /// match QO's `max(0.0, …)` return.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minutes_remaining: Option<f64>,
    /// Machine-readable reason string. Set when `allowed = false`, or
    /// when the client synthesises a fail-open response.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// `true` when the agent MUST obtain a fresh user-provided
    /// checkpoint token before retrying. Defaults to `false` for happy
    /// paths.
    #[serde(default)]
    pub checkpoint_token_required: bool,
}

/// Wire-shape request for `aara_session_checkpoint`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionCheckpointRequest {
    /// Opaque session identifier (same shape as on the ping side).
    pub session_id: String,
    /// User-supplied confirmation token. Empty string is rejected by
    /// the sidecar.
    pub user_token: String,
    /// Wall-clock timestamp at submission. See [`SessionPingRequest`].
    pub timestamp: f64,
}

/// Wire-shape response for `aara_session_checkpoint`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionCheckpointResponse {
    /// `true` when the checkpoint was accepted and the budget reset.
    pub acknowledged: bool,
    /// Value the action-remaining counter was reset to. Absent on
    /// refusal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actions_reset_to: Option<i32>,
    /// Value the checkpoint-interval-minutes counter was reset to.
    /// Absent on refusal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minutes_reset_to: Option<i32>,
    /// Machine-readable reason on refusal (e.g. `missing_session_or_token`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Errors surfaced by an [`McpTransport`] implementation.
#[derive(Debug, Error)]
pub enum McpError {
    /// The transport could not reach the QO sidecar / MCP server. The
    /// ping path matches on this variant to drive fail-open behaviour.
    #[error("MCP transport unreachable: {0}")]
    Unreachable(String),

    /// The MCP tool returned a structured error or non-2xx status.
    #[error("MCP invocation failed: {0}")]
    Invocation(String),

    /// The response payload could not be deserialised into the expected
    /// shape. Carries the underlying serde error.
    #[error("MCP response decode error: {0}")]
    Decode(#[from] serde_json::Error),
}

/// Abstraction over the transport that carries MCP tool invocations.
///
/// In production this will be implemented by the AARA MCP adapter that
/// proxies calls to the QO sidecar (ARY-2308). In tests the
/// [`MockMcpTransport`] returns canned responses without touching the
/// network. The synchronous signature keeps the surface small for
/// this skeleton; if a future implementation needs async it can wrap a
/// `tokio::task::block_in_place` or be lifted to `async_trait` with a
/// minor breaking change.
pub trait McpTransport {
    /// Invoke the named MCP tool with the supplied JSON argument
    /// object. Returns the raw JSON value the server emitted, or an
    /// [`McpError`] on transport / invocation failure.
    fn invoke(&self, tool: &str, args: Value) -> Result<Value, McpError>;
}

/// Typed client wrapping an [`McpTransport`] with the
/// session-budget verbs.
///
/// The client is intentionally cheap to construct and `Send + Sync` when
/// the inner transport is. It owns the transport so callers can stash it
/// inside [`crate::Runtime`] under an `Arc` if/when a real transport
/// lands.
pub struct SessionBudgetClient<T: McpTransport> {
    transport: T,
}

impl<T: McpTransport> SessionBudgetClient<T> {
    /// Construct a new client wrapping `transport`.
    pub fn new(transport: T) -> Self {
        Self { transport }
    }

    /// Borrow the underlying transport. Exposed primarily for tests
    /// that need to assert transport behaviour after a call.
    pub fn transport(&self) -> &T {
        &self.transport
    }

    /// Send a `session_ping`. See module-level docs for the fail-open
    /// semantics on [`McpError::Unreachable`].
    pub fn session_ping(
        &self,
        request: SessionPingRequest,
    ) -> Result<SessionPingResponse, McpError> {
        let args = json!({
            "session_id": request.session_id,
            "action_type": request.action_type,
            "timestamp": request.timestamp,
        });
        match self.transport.invoke(TOOL_SESSION_PING, args) {
            Ok(value) => {
                let response: SessionPingResponse = serde_json::from_value(value)?;
                Ok(response)
            }
            Err(McpError::Unreachable(detail)) => {
                tracing::warn!(
                    target: "jekko_runtime::session_budget",
                    detail = %detail,
                    session_id = %request.session_id,
                    action_type = %request.action_type,
                    "policy sidecar unreachable; failing OPEN per ADR-020 / \
                     safety-kernel-consolidation. Kernel pre_launch_check \
                     still applies on the QO side."
                );
                Ok(SessionPingResponse {
                    allowed: true,
                    actions_remaining: None,
                    minutes_remaining: None,
                    reason: Some(REASON_SIDECAR_UNREACHABLE_FAIL_OPEN.to_string()),
                    checkpoint_token_required: false,
                })
            }
            Err(other) => Err(other),
        }
    }

    /// Send a `session_checkpoint`. Errors from the transport propagate
    /// — there is no fail-open path for checkpointing because a missing
    /// sidecar genuinely cannot reset budget.
    pub fn session_checkpoint(
        &self,
        request: SessionCheckpointRequest,
    ) -> Result<SessionCheckpointResponse, McpError> {
        let args = json!({
            "session_id": request.session_id,
            "user_token": request.user_token,
            "timestamp": request.timestamp,
        });
        let value = self.transport.invoke(TOOL_SESSION_CHECKPOINT, args)?;
        let response: SessionCheckpointResponse = serde_json::from_value(value)?;
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    /// Mock transport that returns whatever closure is stored in
    /// `responder`. Records every invocation in `calls`.
    struct MockMcpTransport {
        responder: Box<dyn Fn(&str, &Value) -> Result<Value, McpError>>,
        calls: RefCell<Vec<(String, Value)>>,
    }

    impl MockMcpTransport {
        fn new<F>(responder: F) -> Self
        where
            F: Fn(&str, &Value) -> Result<Value, McpError> + 'static,
        {
            Self {
                responder: Box::new(responder),
                calls: RefCell::new(Vec::new()),
            }
        }

        fn calls(&self) -> Vec<(String, Value)> {
            self.calls.borrow().clone()
        }
    }

    impl McpTransport for MockMcpTransport {
        fn invoke(&self, tool: &str, args: Value) -> Result<Value, McpError> {
            self.calls
                .borrow_mut()
                .push((tool.to_string(), args.clone()));
            (self.responder)(tool, &args)
        }
    }

    fn ping_request() -> SessionPingRequest {
        SessionPingRequest {
            session_id: "sess-abc".into(),
            action_type: "launch_training_run".into(),
            timestamp: 1_700_000_000.0,
        }
    }

    #[test]
    fn session_ping_happy_path_returns_allowed() {
        let transport = MockMcpTransport::new(|tool, _args| {
            assert_eq!(tool, TOOL_SESSION_PING);
            Ok(json!({
                "allowed": true,
                "actions_remaining": 19,
                "minutes_remaining": 29.5,
            }))
        });
        let client = SessionBudgetClient::new(transport);
        let resp = client.session_ping(ping_request()).expect("ping ok");
        assert!(resp.allowed);
        assert_eq!(resp.actions_remaining, Some(19));
        assert_eq!(resp.minutes_remaining, Some(29.5));
        assert_eq!(resp.reason, None);
        assert!(!resp.checkpoint_token_required);
        // Verify the request was shaped per the wire contract.
        let calls = client.transport().calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, TOOL_SESSION_PING);
        assert_eq!(calls[0].1["session_id"], "sess-abc");
        assert_eq!(calls[0].1["action_type"], "launch_training_run");
        assert_eq!(calls[0].1["timestamp"], 1_700_000_000.0);
    }

    #[test]
    fn session_ping_checkpoint_required_propagates_reason() {
        let transport = MockMcpTransport::new(|_tool, _args| {
            Ok(json!({
                "allowed": false,
                "reason": "session_checkpoint_required",
                "checkpoint_token_required": true,
                "actions_remaining": 0,
                "minutes_remaining": 0.0,
            }))
        });
        let client = SessionBudgetClient::new(transport);
        let resp = client.session_ping(ping_request()).expect("ping ok");
        assert!(!resp.allowed);
        assert_eq!(resp.reason.as_deref(), Some("session_checkpoint_required"));
        assert!(resp.checkpoint_token_required);
        assert_eq!(resp.actions_remaining, Some(0));
    }

    #[test]
    fn session_ping_fail_open_on_unreachable_transport() {
        let transport = MockMcpTransport::new(|_tool, _args| {
            Err(McpError::Unreachable("connection refused".into()))
        });
        let client = SessionBudgetClient::new(transport);
        let resp = client.session_ping(ping_request()).expect("fail-open ok");
        assert!(resp.allowed, "fail-open must return allowed=true");
        assert_eq!(
            resp.reason.as_deref(),
            Some(REASON_SIDECAR_UNREACHABLE_FAIL_OPEN)
        );
        assert!(!resp.checkpoint_token_required);
        // We still recorded the attempt — the underlying transport saw it.
        assert_eq!(client.transport().calls().len(), 1);
    }

    #[test]
    fn session_checkpoint_acknowledged_resets_budget() {
        let transport = MockMcpTransport::new(|tool, _args| {
            assert_eq!(tool, TOOL_SESSION_CHECKPOINT);
            Ok(json!({
                "acknowledged": true,
                "actions_reset_to": 20,
                "minutes_reset_to": 30,
            }))
        });
        let client = SessionBudgetClient::new(transport);
        let resp = client
            .session_checkpoint(SessionCheckpointRequest {
                session_id: "sess-abc".into(),
                user_token: "ck-token-xyz".into(),
                timestamp: 1_700_000_100.0,
            })
            .expect("checkpoint ok");
        assert!(resp.acknowledged);
        assert_eq!(resp.actions_reset_to, Some(20));
        assert_eq!(resp.minutes_reset_to, Some(30));
        // Confirm the request shape was correct.
        let calls = client.transport().calls();
        assert_eq!(calls[0].1["user_token"], "ck-token-xyz");
    }

    #[test]
    fn session_checkpoint_unreachable_does_not_fail_open() {
        // Unlike ping, checkpoint MUST propagate transport errors — a
        // synthetic "ok" would silently let a budget-exhausted agent keep
        // going. The caller has to handle the error explicitly.
        let transport =
            MockMcpTransport::new(|_tool, _args| Err(McpError::Unreachable("no socket".into())));
        let client = SessionBudgetClient::new(transport);
        let err = client
            .session_checkpoint(SessionCheckpointRequest {
                session_id: "sess-abc".into(),
                user_token: "ck-token-xyz".into(),
                timestamp: 1_700_000_100.0,
            })
            .unwrap_err();
        assert!(matches!(err, McpError::Unreachable(_)));
    }

    #[test]
    fn ping_request_serializes_to_wire_compatible_json() {
        // Lock the wire shape: the JSON keys MUST match the QO sidecar's
        // `_handle_session_ping` payload reader. If this test fails, the
        // QO side will silently treat the field as missing.
        let req = ping_request();
        let value = serde_json::to_value(&req).expect("serialize");
        assert_eq!(value["session_id"], "sess-abc");
        assert_eq!(value["action_type"], "launch_training_run");
        assert_eq!(value["timestamp"], 1_700_000_000.0);
    }

    #[test]
    fn checkpoint_response_optional_fields_round_trip() {
        // A refusal payload only carries `acknowledged` + `reason`; the
        // `*_reset_to` fields must remain `None` rather than panicking
        // on missing keys.
        let value = json!({
            "acknowledged": false,
            "reason": "missing_session_or_token",
        });
        let resp: SessionCheckpointResponse = serde_json::from_value(value).expect("decode");
        assert!(!resp.acknowledged);
        assert_eq!(resp.reason.as_deref(), Some("missing_session_or_token"));
        assert_eq!(resp.actions_reset_to, None);
        assert_eq!(resp.minutes_reset_to, None);
    }
}
