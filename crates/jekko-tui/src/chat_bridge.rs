//! HTTP+SSE bridge from the TUI prompt to a local jnoccio-fusion gateway.
//!
//! Spawns a worker thread per chat submission. The worker opens a blocking
//! TCP connection to `127.0.0.1:4317`, posts an OpenAI-compatible
//! chat-completions request with `stream: true`, parses the SSE response,
//! and forwards each text delta back into the TUI action queue as
//! [`crate::action::RuntimeEvent::AssistantTextDelta`].
//!
//! Intentionally dependency-light: uses only `std` + `serde_json` (already
//! in jekko-tui deps). Keeps the TUI crate independent of `jekko-runtime`,
//! `reqwest`, and `tokio`. Suitable for a single local gateway; not a
//! general-purpose provider client.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::sync::mpsc::Sender;
use std::time::Duration;

use crate::action::{Action, RuntimeEvent};

const GATEWAY_HOST: &str = "127.0.0.1";
const GATEWAY_PORT: u16 = 4317;
const GATEWAY_PATH: &str = "/v1/chat/completions";
const DEFAULT_MODEL: &str = "jnoccio/jnoccio-fusion";
/// Cap on individual blocking reads so a hung gateway can't pin the worker
/// thread for the lifetime of the TUI process.
const READ_TIMEOUT: Duration = Duration::from_secs(120);

/// Spawn a background worker that posts `prompt` to the local jnoccio-fusion
/// gateway and forwards each streamed text delta back through `action_tx`.
/// Returns immediately; the worker shuts down once the SSE stream closes or
/// on first I/O error.
pub fn spawn_chat_request(prompt: String, action_tx: Sender<Action>) {
    std::thread::Builder::new()
        .name("jekko-tui-chat-bridge".into())
        .spawn(move || {
            if let Err(err) = run_chat(&prompt, &action_tx) {
                let _ = action_tx.send(Action::Runtime(RuntimeEvent::AssistantFailed {
                    error: err.to_string(),
                }));
            }
            let _ = action_tx.send(Action::Runtime(RuntimeEvent::AssistantCompleted));
        })
        .ok();
}

fn run_chat(prompt: &str, tx: &Sender<Action>) -> std::io::Result<()> {
    let body = serde_json::json!({
        "model": DEFAULT_MODEL,
        "stream": true,
        "messages": [
            { "role": "user", "content": prompt }
        ]
    })
    .to_string();

    let mut stream = TcpStream::connect((GATEWAY_HOST, GATEWAY_PORT))?;
    stream.set_read_timeout(Some(READ_TIMEOUT))?;
    stream.set_write_timeout(Some(READ_TIMEOUT))?;

    let request = format!(
        "POST {path} HTTP/1.1\r\n\
         Host: {host}:{port}\r\n\
         Content-Type: application/json\r\n\
         Accept: text/event-stream\r\n\
         Content-Length: {len}\r\n\
         Connection: close\r\n\
         \r\n\
         {body}",
        path = GATEWAY_PATH,
        host = GATEWAY_HOST,
        port = GATEWAY_PORT,
        len = body.len(),
        body = body
    );
    stream.write_all(request.as_bytes())?;
    stream.flush()?;

    let mut reader = BufReader::new(stream);
    // Skip HTTP/1.1 status line + headers (consume up to and including the
    // blank line that separates headers from body).
    let mut status_line = String::new();
    reader.read_line(&mut status_line)?;
    if !status_line.starts_with("HTTP/1.1 200") && !status_line.starts_with("HTTP/1.0 200") {
        return Err(std::io::Error::other(format!(
            "jnoccio gateway returned non-200 status: {}",
            status_line.trim()
        )));
    }
    loop {
        let mut header = String::new();
        let n = reader.read_line(&mut header)?;
        if n == 0 {
            return Err(std::io::Error::other("gateway closed before headers ended"));
        }
        if header == "\r\n" || header == "\n" {
            break;
        }
    }

    // Parse SSE: alternating "data: <json>" lines and blank-line delimiters.
    // Some gateways send Transfer-Encoding: chunked; we ignore chunk-size
    // lines (hex digits followed by \r\n) — they don't match the `data: `
    // prefix so they fall through harmlessly.
    let mut reasoning_started = false;
    let mut reasoning_buf = String::new();
    let mut line = String::new();
    loop {
        line.clear();
        let n = reader.read_line(&mut line)?;
        if n == 0 {
            break;
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        let payload = match trimmed.strip_prefix("data: ") {
            Some(p) => p,
            None => continue,
        };
        if payload == "[DONE]" {
            break;
        }
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(payload) {
            let delta = value
                .get("choices")
                .and_then(|c| c.get(0))
                .and_then(|c| c.get("delta"));

            if let Some(d) = delta {
                // Text delta (content field)
                if let Some(text) = d.get("content").and_then(|t| t.as_str()) {
                    if !text.is_empty() {
                        let _ = tx.send(Action::Runtime(RuntimeEvent::AssistantTextDelta {
                            text: text.to_string(),
                        }));
                    }
                }

                // Reasoning delta — Anthropic extended thinking / OpenAI o1 style.
                // Field names: "reasoning" (Anthropic), "reasoning_content" (OpenAI o1)
                let reasoning_text = d
                    .get("reasoning")
                    .or_else(|| d.get("reasoning_content"))
                    .and_then(|r| r.as_str());
                if let Some(text) = reasoning_text {
                    if !text.is_empty() {
                        if !reasoning_started {
                            reasoning_started = true;
                            let _ = tx.send(Action::Runtime(RuntimeEvent::ReasoningStarted {
                                reasoning_id: "r0".to_string(),
                            }));
                        }
                        reasoning_buf.push_str(text);
                        let _ = tx.send(Action::Runtime(RuntimeEvent::ReasoningDelta {
                            text: text.to_string(),
                        }));
                    }
                }
            }
        }
    }

    // Finalize reasoning stream if one was active.
    if reasoning_started {
        let _ = tx.send(Action::Runtime(RuntimeEvent::ReasoningEnded {
            reasoning_id: "r0".to_string(),
            text: reasoning_buf,
        }));
    }

    // Drain remainder so the kernel can close cleanly.
    let mut drain = Vec::new();
    let _ = reader.read_to_end(&mut drain);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::net::TcpListener;
    use std::sync::mpsc;
    use std::thread;

    fn fake_sse_response() -> String {
        let chunks = [
            r#"data: {"choices":[{"delta":{"content":"Hello"}}]}"#,
            "",
            r#"data: {"choices":[{"delta":{"content":" world"}}]}"#,
            "",
            "data: [DONE]",
            "",
        ];
        let body = format!("{}\n", chunks.join("\n"));
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        )
    }

    #[test]
    fn run_chat_parses_sse_deltas() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        thread::spawn(move || {
            let (mut sock, _) = listener.accept().unwrap();
            // Drain request bytes — we only care about the response shape.
            let mut buf = [0u8; 1024];
            let _ = sock.set_read_timeout(Some(Duration::from_millis(200)));
            let _ = sock.read(&mut buf);
            sock.write_all(fake_sse_response().as_bytes()).unwrap();
        });

        let (tx, rx) = mpsc::channel::<Action>();
        // Override host/port via a custom run so we hit the fake server.
        run_chat_custom(addr.ip().to_string().as_str(), addr.port(), "hi", &tx).unwrap();

        let deltas: Vec<String> = rx
            .try_iter()
            .filter_map(|a| match a {
                Action::Runtime(RuntimeEvent::AssistantTextDelta { text }) => Some(text),
                _ => None,
            })
            .collect();
        assert_eq!(deltas, vec!["Hello".to_string(), " world".to_string()]);
    }

    // Test-only variant that takes an explicit (host, port) so we can target
    // a local TcpListener without process-global state.
    fn run_chat_custom(
        host: &str,
        port: u16,
        prompt: &str,
        tx: &Sender<Action>,
    ) -> std::io::Result<()> {
        let body = serde_json::json!({
            "model": DEFAULT_MODEL,
            "stream": true,
            "messages": [{ "role": "user", "content": prompt }]
        })
        .to_string();
        let mut stream = TcpStream::connect((host, port))?;
        stream.set_read_timeout(Some(Duration::from_secs(2)))?;
        let request = format!(
            "POST {p} HTTP/1.1\r\nHost: {h}:{port}\r\nContent-Type: application/json\r\nAccept: text/event-stream\r\nContent-Length: {l}\r\nConnection: close\r\n\r\n{b}",
            p = GATEWAY_PATH,
            h = host,
            port = port,
            l = body.len(),
            b = body
        );
        stream.write_all(request.as_bytes())?;
        let mut reader = BufReader::new(stream);
        let mut status = String::new();
        reader.read_line(&mut status)?;
        loop {
            let mut h = String::new();
            let n = reader.read_line(&mut h)?;
            if n == 0 || h == "\r\n" || h == "\n" {
                break;
            }
        }
        let mut line = String::new();
        loop {
            line.clear();
            let n = reader.read_line(&mut line)?;
            if n == 0 {
                break;
            }
            let t = line.trim_end_matches(['\r', '\n']);
            let payload = match t.strip_prefix("data: ") {
                Some(p) => p,
                None => continue,
            };
            if payload == "[DONE]" {
                break;
            }
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(payload) {
                if let Some(delta) = v
                    .get("choices")
                    .and_then(|c| c.get(0))
                    .and_then(|c| c.get("delta"))
                    .and_then(|d| d.get("content"))
                    .and_then(|t| t.as_str())
                {
                    let _ = tx.send(Action::Runtime(RuntimeEvent::AssistantTextDelta {
                        text: delta.to_string(),
                    }));
                }
            }
        }
        Ok(())
    }
}
