//! Shared daemon socket protocol for `jekko daemon` and `jekko attach`.
//!
//! The transport is newline-delimited JSON over the Unix socket resolved from
//! `JEKKO_DAEMON_SOCKET` or `~/.jekko/jekko-daemon.sock`. The schema is small
//! on purpose: attach performs a handshake, prompt submission streams frames,
//! and lifecycle commands use request/response messages.

use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Environment override for the daemon Unix socket path.
pub const DAEMON_SOCKET_ENV: &str = "JEKKO_DAEMON_SOCKET";

/// Daemon request frame.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DaemonRequest {
    /// Attach to a daemon-managed session stream.
    Attach {
        /// Session id or `latest`.
        session_id: String,
        /// When true, the server must suppress submit/abort effects.
        read_only: bool,
    },
    /// Submit a prompt to the attached session.
    Submit {
        /// User prompt text.
        prompt: String,
    },
    /// Abort the attached session turn.
    Abort,
    /// Query daemon status.
    Status,
    /// Stop the daemon.
    Stop,
    /// Fetch recent daemon log lines.
    Logs {
        /// Maximum number of trailing lines.
        lines: usize,
    },
}

/// Daemon response or stream frame.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DaemonResponse {
    /// Attach handshake succeeded.
    Attached {
        /// Resolved session id.
        session_id: String,
        /// Whether this attach stream is read-only.
        read_only: bool,
    },
    /// Status payload.
    Status {
        /// Daemon process id.
        pid: u32,
        /// Socket path in use.
        socket: String,
        /// Latest known session id, when any exists.
        latest_session: Option<String>,
    },
    /// Log payload.
    Logs {
        /// Log lines, oldest to newest.
        lines: Vec<String>,
    },
    /// Generic acknowledgement.
    Ack {
        /// Human-readable message.
        message: String,
    },
    /// Streamed chat/session frame.
    Frame(DaemonStreamFrame),
    /// Error payload.
    Error {
        /// Human-readable error.
        message: String,
    },
}

/// Streamed daemon session frame.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DaemonStreamFrame {
    /// Informational notice.
    Notice {
        /// Message text.
        message: String,
    },
    /// Assistant text delta.
    AssistantDelta {
        /// Text delta.
        text: String,
    },
    /// Session lifecycle began.
    SessionStarted {
        /// Session id.
        session_id: String,
        /// Optional title.
        #[serde(default)]
        title: Option<String>,
    },
    /// Session lifecycle ended.
    SessionEnded {
        /// Session id.
        session_id: String,
    },
    /// Daemon status update.
    DaemonStatus {
        /// Whether the daemon is reachable.
        online: bool,
        /// Session id, when the status is scoped.
        #[serde(default)]
        session_id: Option<String>,
        /// Optional human-readable detail.
        #[serde(default)]
        message: Option<String>,
    },
    /// Permission ask.
    PermissionAsked {
        /// Permission request id.
        request_id: String,
        /// Session id.
        session_id: String,
        /// Permission name.
        permission: String,
        /// Target patterns.
        patterns: Vec<String>,
        /// Patterns that may be auto-approved on `always`.
        always: Vec<String>,
    },
    /// Permission reply.
    PermissionReplied {
        /// Permission request id.
        request_id: String,
        /// Session id.
        session_id: String,
        /// User reply.
        reply: String,
    },
    /// Question ask.
    QuestionAsked {
        /// Question id.
        question_id: String,
        /// Session id.
        session_id: String,
        /// Prompt text.
        prompt: String,
        /// Choice list, if any.
        choices: Vec<String>,
    },
    /// Question reply.
    QuestionReplied {
        /// Question id.
        question_id: String,
        /// Session id.
        session_id: String,
        /// Free-form answer.
        answer: String,
    },
    /// Turn completed.
    Completed,
    /// Turn failed or was cancelled.
    Failed {
        /// Human-readable reason.
        error: String,
    },
}

/// Resolve the daemon socket path from env or the default user path.
pub fn socket_path() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os(DAEMON_SOCKET_ENV) {
        return Ok(path.into());
    }
    let base = std::env::var_os("HOME")
        .map(PathBuf::from)
        .context("HOME is required when JEKKO_DAEMON_SOCKET is unset")?
        .join(".jekko");
    Ok(base.join("jekko-daemon.sock"))
}

/// Resolve the daemon log path next to the default socket directory.
pub fn log_path() -> Result<PathBuf> {
    let base = std::env::var_os("HOME")
        .map(PathBuf::from)
        .context("HOME is required for daemon logs")?
        .join(".jekko");
    Ok(base.join("jekko-daemon.log"))
}

/// Resolve the daemon pid path next to the default socket directory.
pub fn pid_path() -> Result<PathBuf> {
    let base = std::env::var_os("HOME")
        .map(PathBuf::from)
        .context("HOME is required for daemon pid file")?
        .join(".jekko");
    Ok(base.join("jekko-daemon.pid"))
}

/// Encode a protocol value as one JSON line.
pub fn encode_line<T: Serialize>(value: &T) -> Result<String> {
    let mut line = serde_json::to_string(value)?;
    line.push('\n');
    Ok(line)
}

/// Decode a protocol value from one JSON line.
pub fn decode_line<T: for<'de> Deserialize<'de>>(line: &str) -> Result<T> {
    Ok(serde_json::from_str(line.trim_end())?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_round_trips_as_json_line() {
        let req = DaemonRequest::Attach {
            session_id: "latest".into(),
            read_only: true,
        };
        let line = encode_line(&req).unwrap();
        assert!(line.ends_with('\n'));
        assert_eq!(decode_line::<DaemonRequest>(&line).unwrap(), req);
    }

    #[test]
    fn response_round_trips_as_json_line() {
        let resp = DaemonResponse::Frame(DaemonStreamFrame::AssistantDelta { text: "hi".into() });
        assert_eq!(
            decode_line::<DaemonResponse>(&encode_line(&resp).unwrap()).unwrap(),
            resp
        );
    }

    #[test]
    fn runtime_frame_round_trips_as_json_line() {
        let resp = DaemonResponse::Frame(DaemonStreamFrame::SessionStarted {
            session_id: "session_1".into(),
            title: Some("hello".into()),
        });
        assert_eq!(
            decode_line::<DaemonResponse>(&encode_line(&resp).unwrap()).unwrap(),
            resp
        );
    }

    #[test]
    fn permission_and_question_frames_round_trip_as_json_line() {
        let frames = [
            DaemonStreamFrame::PermissionAsked {
                request_id: "perm_1".into(),
                session_id: "session_1".into(),
                permission: "bash".into(),
                patterns: vec!["ls".into()],
                always: vec!["ls".into()],
            },
            DaemonStreamFrame::QuestionAsked {
                question_id: "question_1".into(),
                session_id: "session_1".into(),
                prompt: "continue?".into(),
                choices: vec!["yes".into(), "no".into()],
            },
        ];
        for frame in frames {
            let resp = DaemonResponse::Frame(frame.clone());
            assert_eq!(
                decode_line::<DaemonResponse>(&encode_line(&resp).unwrap()).unwrap(),
                resp
            );
        }
    }

    #[test]
    fn decode_line_rejects_invalid_json() {
        assert!(decode_line::<DaemonRequest>("not json").is_err());
    }
}
