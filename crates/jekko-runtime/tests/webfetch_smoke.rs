//! Smoke tests for `WebFetchTool`.
//!
//! Spins up a tiny `tokio` TCP listener that speaks just enough HTTP/1.1
//! to answer one request per connection, then drives `WebFetchTool` at
//! that local endpoint. No external network access required.

use std::sync::Arc;

use jekko_runtime::tool::{Tool, ToolContext, WebFetchTool};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

const PAGE_BODY: &str = "<html><body><h1>hello jekko</h1></body></html>";

async fn spawn_mock_server(repeat_body_kib: usize) -> (u16, oneshot::Sender<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let port = listener.local_addr().unwrap().port();
    let body_owned: String = if repeat_body_kib > 0 {
        // ~1 KiB per repeat — used to exercise truncation.
        "x".repeat(1024 * repeat_body_kib)
    } else {
        PAGE_BODY.to_string()
    };
    let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();
    let body = Arc::new(body_owned);

    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = &mut shutdown_rx => break,
                accept = listener.accept() => {
                    let Ok((mut stream, _)) = accept else { break };
                    let body = body.clone();
                    tokio::spawn(async move {
                        let mut buf = [0u8; 2048];
                        let _ = stream.read(&mut buf).await;
                        let response = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                            body.len(),
                            body
                        );
                        let _ = stream.write_all(response.as_bytes()).await;
                        let _ = stream.shutdown().await;
                    });
                }
            }
        }
    });
    (port, shutdown_tx)
}

#[tokio::test]
async fn fetches_html_body() {
    let (port, shutdown) = spawn_mock_server(0).await;
    let out = WebFetchTool
        .execute(
            serde_json::json!({
                "url": format!("http://127.0.0.1:{port}/page"),
                "timeout_ms": 5000,
            }),
            ToolContext::bare("."),
        )
        .await
        .expect("webfetch should succeed");
    assert!(
        out.output.contains("hello jekko"),
        "expected body in output, got: {}",
        out.output
    );
    assert_eq!(out.metadata["status"], serde_json::json!(200));
    let ct = out.metadata["content_type"].as_str().unwrap_or("");
    assert!(
        ct.contains("text/html"),
        "expected text/html content_type, got {ct}"
    );
    assert_eq!(out.metadata["truncated"], serde_json::json!(false));
    let _ = shutdown.send(());
}

#[tokio::test]
async fn truncates_body_at_max_bytes() {
    // Server returns ~4 KiB; we cap at 1 KiB.
    let (port, shutdown) = spawn_mock_server(4).await;
    let out = WebFetchTool
        .execute(
            serde_json::json!({
                "url": format!("http://127.0.0.1:{port}/big"),
                "max_bytes": 1024,
                "timeout_ms": 5000,
            }),
            ToolContext::bare("."),
        )
        .await
        .expect("webfetch should succeed");
    assert_eq!(out.metadata["truncated"], serde_json::json!(true));
    assert!(
        out.output.len() as u64 <= 1024,
        "expected body trimmed to <=1024 bytes, got {}",
        out.output.len()
    );
    let bytes = out.metadata["bytes"].as_u64().unwrap_or(0);
    assert!(
        bytes >= 4096,
        "expected `bytes` to reflect actual download size (>=4096), got {bytes}"
    );
    let _ = shutdown.send(());
}

#[tokio::test]
async fn rejects_non_http_scheme() {
    let err = WebFetchTool
        .execute(
            serde_json::json!({ "url": "ftp://example.com/foo" }),
            ToolContext::bare("."),
        )
        .await
        .expect_err("ftp should be rejected");
    let msg = format!("{err}");
    assert!(
        msg.contains("http") || msg.to_lowercase().contains("scheme"),
        "msg: {msg}"
    );
}
