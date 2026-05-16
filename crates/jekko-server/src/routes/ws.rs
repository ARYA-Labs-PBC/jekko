//! `/api/v1/ws/:session_id` — WebSocket bridge to a single session.
//!
//! On connect we subscribe to the bus wildcard channel and forward every
//! event whose payload references `session_id`. Incoming text frames are
//! parsed as `{ "type": "submit", "prompt": "…" }` and a new prompt
//! message is appended to the session (other types are ignored for now).

use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use futures_util::{sink::SinkExt, stream::StreamExt};
use jekko_core::session::SessionId;
use jekko_runtime::session::AppendMessageInput;
use serde::Deserialize;
use tokio::sync::broadcast::error::RecvError;

use crate::state::AppState;

/// Build the WebSocket router.
pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/:session_id", get(handler))
}

/// `GET /api/v1/ws/:session_id` (upgrade).
#[utoipa::path(
    get,
    path = "/api/v1/ws/{session_id}",
    responses((status = 101, description = "Upgraded"))
)]
pub async fn handler(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| run(socket, session_id, state))
}

/// Schema accepted by inbound text frames.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ClientFrame {
    /// Submit a new prompt.
    Submit {
        /// Prompt text.
        prompt: String,
    },
    /// Cancel the in-flight prompt.
    Abort,
    /// Resize (used by the PTY bridge but accepted here as a no-op for shared clients).
    Resize {
        /// Columns.
        cols: u16,
        /// Rows.
        rows: u16,
    },
}

async fn run(socket: WebSocket, session_id: String, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();
    let mut bus_sub = state.bus.subscribe_all();

    // Send hello frame so a client can verify the upgrade.
    let _ = sender
        .send(Message::Text(
            serde_json::json!({
                "type": "hello",
                "sessionID": session_id,
            })
            .to_string(),
        ))
        .await;

    let state_in = state.clone();
    let session_in = session_id.clone();
    let inbound = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Text(text) = msg {
                let parsed: Result<ClientFrame, _> = serde_json::from_str(&text);
                match parsed {
                    Ok(ClientFrame::Submit { prompt }) => {
                        let sid = SessionId::new(session_in.clone());
                        let _ = state_in
                            .sessions
                            .append(AppendMessageInput {
                                session_id: sid,
                                role: "user".into(),
                                data: serde_json::json!({ "text": prompt }),
                            })
                            .await;
                    }
                    Ok(ClientFrame::Abort) => {
                        let _ = state_in
                            .bus
                            .publish(
                                "session.aborted",
                                serde_json::json!({ "sessionID": session_in }),
                            )
                            .await;
                    }
                    Ok(ClientFrame::Resize { cols, rows }) => {
                        let _ = state_in
                            .bus
                            .publish(
                                "tui.resize",
                                serde_json::json!({
                                    "sessionID": session_in,
                                    "cols": cols,
                                    "rows": rows,
                                }),
                            )
                            .await;
                    }
                    Err(_) => continue,
                }
            }
        }
    });

    let outbound = tokio::spawn(async move {
        loop {
            match bus_sub.recv().await {
                Ok(envelope) => {
                    if !envelope_matches_session(&envelope.properties, &session_id) {
                        continue;
                    }
                    let payload = serde_json::json!({
                        "id": envelope.id,
                        "type": envelope.kind,
                        "properties": envelope.properties,
                    });
                    if sender
                        .send(Message::Text(payload.to_string()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                Err(RecvError::Lagged(_)) => continue,
                Err(RecvError::Closed) => break,
            }
        }
        let _ = sender.send(Message::Close(None)).await;
    });

    // Wait for either side to drop; then ensure both are cancelled.
    tokio::select! {
        _ = inbound => {}
        _ = outbound => {}
        _ = tokio::time::sleep(Duration::from_secs(60 * 60)) => {}
    }
}

/// JSON keys used to identify the session a bus envelope belongs to, in
/// priority order. The TS bus emits `sessionID`; older Rust producers used
/// `session_id`. We accept either spelling.
const SESSION_ID_JSON_KEYS: &[&str] = &["sessionID", "session_id"];

/// Look up the first matching key on `payload` and return its string value.
fn lookup_session_id(payload: &serde_json::Value) -> Option<&str> {
    for key in SESSION_ID_JSON_KEYS {
        if let Some(value) = payload.get(*key).and_then(|v| v.as_str()) {
            return Some(value);
        }
    }
    None
}

fn envelope_matches_session(payload: &serde_json::Value, session_id: &str) -> bool {
    match lookup_session_id(payload) {
        Some(s) => s == session_id,
        None => true, // global / unscoped events broadcast to everyone
    }
}
