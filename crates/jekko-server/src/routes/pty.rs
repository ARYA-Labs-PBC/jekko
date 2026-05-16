//! `/api/v1/pty/:tty_id` — WebSocket-to-PTY bridge.
//!
//! Spawns a PTY with a default shell on first connect and ferries bytes
//! between the WebSocket and the master side. The shell defaults to
//! `$SHELL` and otherwise uses `/bin/sh`, with an 80x24 terminal size.
//!
//! Because [`jekko_runtime::pty::PtySession`] is `Send` but not `Sync`, the
//! session is owned by a single blocking OS thread; we hand work to it via
//! a Tokio channel and forward output bytes back via another channel.

use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use futures_util::{sink::SinkExt, stream::StreamExt};
use jekko_runtime::pty::{PtySession, PtySpec};
use serde::Deserialize;
use tokio::sync::mpsc;

use crate::state::AppState;

/// Shell executable used when `$SHELL` is unset or unreadable.
const DEFAULT_SHELL: &str = "/bin/sh";

/// Build the PTY router.
pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/:tty_id", get(handler))
}

/// `GET /api/v1/pty/:tty_id` (upgrade).
#[utoipa::path(
    get,
    path = "/api/v1/pty/{tty_id}",
    responses((status = 101, description = "Upgraded"))
)]
pub async fn handler(
    State(_state): State<Arc<AppState>>,
    Path(tty_id): Path<String>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| run(socket, tty_id))
}

/// PTY control frames accepted from the client.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum PtyClientFrame {
    /// Send raw bytes (UTF-8 decoded from text frames).
    Input {
        /// Bytes to write.
        data: String,
    },
    /// Resize the PTY.
    Resize {
        /// Columns.
        cols: u16,
        /// Rows.
        rows: u16,
    },
}

enum PtyCmd {
    Input(Vec<u8>),
    Resize { cols: u16, rows: u16 },
    Kill,
}

async fn run(socket: WebSocket, tty_id: String) {
    let shell = match std::env::var("SHELL") {
        Ok(s) => s,
        Err(_) => DEFAULT_SHELL.to_string(),
    };
    let spec = PtySpec {
        command: shell,
        args: Vec::new(),
        cols: 80,
        rows: 24,
    };

    let (cmd_tx, cmd_rx) = mpsc::channel::<PtyCmd>(32);
    let (out_tx, mut out_rx) = mpsc::channel::<Vec<u8>>(32);

    // Single OS thread owns the PtySession (the master PTY is `Send` but
    // not `Sync`, so the session can only live on one thread at a time).
    std::thread::spawn(move || {
        pty_thread(spec, cmd_rx, out_tx);
    });

    let (mut sender, mut receiver) = socket.split();
    let _ = sender
        .send(Message::Text(
            serde_json::json!({
                "type": "pty.ready",
                "ttyID": tty_id,
            })
            .to_string(),
        ))
        .await;

    let cmd_tx_in = cmd_tx.clone();
    let inbound = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    if let Ok(frame) = serde_json::from_str::<PtyClientFrame>(&text) {
                        match frame {
                            PtyClientFrame::Input { data } => {
                                let _ = cmd_tx_in.send(PtyCmd::Input(data.into_bytes())).await;
                            }
                            PtyClientFrame::Resize { cols, rows } => {
                                let _ = cmd_tx_in.send(PtyCmd::Resize { cols, rows }).await;
                            }
                        }
                    } else {
                        let _ = cmd_tx_in.send(PtyCmd::Input(text.into_bytes())).await;
                    }
                }
                Message::Binary(bytes) => {
                    let _ = cmd_tx_in.send(PtyCmd::Input(bytes)).await;
                }
                Message::Close(_) => break,
                _ => continue,
            }
        }
        let _ = cmd_tx_in.send(PtyCmd::Kill).await;
    });

    let outbound = tokio::spawn(async move {
        while let Some(bytes) = out_rx.recv().await {
            if sender.send(Message::Binary(bytes)).await.is_err() {
                break;
            }
        }
        let _ = sender.send(Message::Close(None)).await;
    });

    tokio::select! {
        _ = inbound => {}
        _ = outbound => {}
        _ = tokio::time::sleep(Duration::from_secs(60 * 60)) => {}
    }
    let _ = cmd_tx.send(PtyCmd::Kill).await;
}

/// Blocking PTY-owning thread. Interleaves a read poll loop with a command
/// channel drain.
fn pty_thread(spec: PtySpec, mut cmd_rx: mpsc::Receiver<PtyCmd>, out_tx: mpsc::Sender<Vec<u8>>) {
    let session = match PtySession::spawn(&spec) {
        Ok(s) => s,
        Err(_) => return,
    };

    loop {
        // Drain any pending commands without blocking.
        match cmd_rx.try_recv() {
            Ok(PtyCmd::Input(bytes)) => {
                let _ = session.write(&bytes);
                continue;
            }
            Ok(PtyCmd::Resize { cols, rows }) => {
                let _ = session.resize(cols, rows);
                continue;
            }
            Ok(PtyCmd::Kill) => break,
            Err(mpsc::error::TryRecvError::Empty) => {}
            Err(mpsc::error::TryRecvError::Disconnected) => break,
        }

        // Poll PTY for output.
        match session.read(8192) {
            Ok(bytes) if bytes.is_empty() => {
                std::thread::sleep(Duration::from_millis(20));
            }
            Ok(bytes) => {
                if out_tx.blocking_send(bytes).is_err() {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    let _ = session.kill();
}
