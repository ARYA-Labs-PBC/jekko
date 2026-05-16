//! SSE endpoint smoke test.
//!
//! Opens `/api/v1/events`, publishes a bus event, and asserts the stream
//! emits a matching frame.

mod common;

use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use common::make_router;
use http_body_util::BodyExt;
use jekko_server::AppState;
use tokio::time::timeout;
use tower::ServiceExt;

#[tokio::test]
async fn sse_streams_published_event() {
    let state = Arc::new(AppState::new());
    let bus = state.bus.clone();
    let app = make_router(state.clone());

    let req = Request::builder()
        .uri("/api/v1/events")
        .method("GET")
        .header(header::ACCEPT, "text/event-stream")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers()
            .get(header::CONTENT_TYPE)
            .map(|v| v.to_str().unwrap_or("")),
        Some("text/event-stream"),
    );

    // Publish an event after a short delay so the receiver is ready.
    let bus_clone = bus.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        let _ = bus_clone
            .publish("hello.world", serde_json::json!({ "msg": "hi" }))
            .await;
    });

    // Read at most one chunk from the body stream.
    let mut body = resp.into_body();
    let mut buf = Vec::new();
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while std::time::Instant::now() < deadline {
        let frame = timeout(Duration::from_millis(500), body.frame()).await;
        match frame {
            Ok(Some(Ok(frame))) => {
                if let Some(data) = frame.data_ref() {
                    buf.extend_from_slice(data);
                    let s = String::from_utf8_lossy(&buf);
                    if s.contains("hello.world") {
                        return;
                    }
                }
            }
            Ok(Some(Err(_))) => break,
            Ok(None) => break,
            Err(_) => continue,
        }
    }
    let s = String::from_utf8_lossy(&buf);
    panic!("did not receive hello.world event in SSE stream. got: {s}");
}
