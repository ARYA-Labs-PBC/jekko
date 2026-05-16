//! End-to-end test: spin up a local TCP server that pretends to be an
//! Anthropic SSE endpoint and assert that [`AnthropicAdapter::stream`]
//! decodes the canned response correctly.

use std::collections::BTreeMap;
use std::io::Write as _;
use std::time::Duration;

use futures_util::StreamExt;
use jekko_provider::adapter::{ProviderAdapter, ProviderCredential, ProviderRequest};
use jekko_provider::providers::anthropic::AnthropicAdapter;
use jekko_provider::stream::ProviderEventKind;
use serde_json::{json, Map};
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

const ANTHROPIC_TEXT: &[u8] = include_bytes!("fixtures/anthropic_text_stream.sse");

#[tokio::test(flavor = "current_thread")]
async fn anthropic_adapter_decodes_canned_response_over_local_tcp() {
    // Bind to a random port.
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{}", addr);

    // Spawn one-shot handler.
    let handler = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let (mut reader, mut writer) = socket.split();
        // Consume the request bytes (best-effort: read until \r\n\r\n).
        let mut buf = vec![0u8; 8192];
        let mut total = String::new();
        loop {
            match tokio::time::timeout(Duration::from_secs(2), reader.read(&mut buf)).await {
                Ok(Ok(n)) if n > 0 => {
                    total.push_str(&String::from_utf8_lossy(&buf[..n]));
                    if total.contains("\r\n\r\n") {
                        break;
                    }
                }
                _ => break,
            }
        }

        // Send a complete HTTP/1.1 200 + SSE body.
        let mut response: Vec<u8> = Vec::new();
        response.extend_from_slice(b"HTTP/1.1 200 OK\r\n");
        response.extend_from_slice(b"Content-Type: text/event-stream\r\n");
        response
            .extend_from_slice(format!("Content-Length: {}\r\n", ANTHROPIC_TEXT.len()).as_bytes());
        response.extend_from_slice(b"Connection: close\r\n\r\n");
        response.extend_from_slice(ANTHROPIC_TEXT);
        use tokio::io::AsyncWriteExt;
        writer.write_all(&response).await.unwrap();
        writer.flush().await.unwrap();
        writer.shutdown().await.ok();
    });

    // Drive the adapter.
    let adapter = AnthropicAdapter::new();
    let req = ProviderRequest {
        model: "anthropic/claude-sonnet-4-5".into(),
        api_model_id: "claude-sonnet-4-5".into(),
        session_id: "sess-1".into(),
        system: vec![],
        messages: vec![json!({ "role": "user", "content": "hi" })],
        tools: vec![],
        tool_choice: None,
        options: Map::new(),
        headers: BTreeMap::new(),
        max_output_tokens: 1024,
        temperature: None,
        top_p: None,
        top_k: None,
        credential: Some(ProviderCredential::ApiKey {
            key: "provider-sample-key".into(),
        }),
        base_url: Some(base_url),
    };
    let abort = CancellationToken::new();
    let mut stream = adapter
        .stream(req, abort)
        .await
        .expect("stream should succeed");
    let mut text = String::new();
    while let Some(ev) = stream.next().await {
        let ev = ev.expect("stream event");
        match ev.kind {
            ProviderEventKind::TextDelta { text: t } => text.push_str(&t),
            ProviderEventKind::StreamEnd { .. } => break,
            _ => {}
        }
    }
    handler.await.unwrap();
    assert_eq!(text, "Hello, world!");
}

#[allow(dead_code)]
fn _force_use(_w: &mut Vec<u8>) {
    // Silence unused import warnings if `std::io::Write` isn't otherwise used.
    let mut v: Vec<u8> = Vec::new();
    let _ = v.write_all(b"x");
}
