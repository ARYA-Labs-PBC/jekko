//! TUI bridge route tests.

mod common;

use axum::http::StatusCode;
use common::{body_json, fresh_state, make_router, post_json};
use tower::ServiceExt;

#[tokio::test]
async fn submit_prompt_publishes_command() {
    let state = fresh_state();
    let bus = state.bus.clone();
    let mut sub = bus.subscribe("tui.command").await;
    let app = make_router(state);

    let resp = app
        .oneshot(post_json(
            "/api/v1/tui/submit-prompt",
            &serde_json::json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let env = sub.recv().await.unwrap();
    assert_eq!(env.kind, "tui.command");
    assert_eq!(env.properties["command"].as_str(), Some("prompt.submit"));
}

#[tokio::test]
async fn append_prompt_publishes_payload() {
    let state = fresh_state();
    let bus = state.bus.clone();
    let mut sub = bus.subscribe("tui.prompt.append").await;
    let app = make_router(state);

    let body = serde_json::json!({ "text": "hello world", "submit": false });
    let resp = app
        .oneshot(post_json("/api/v1/tui/append-prompt", &body))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let env = sub.recv().await.unwrap();
    assert_eq!(env.kind, "tui.prompt.append");
    assert_eq!(env.properties["text"].as_str(), Some("hello world"));
}

#[tokio::test]
async fn open_help_publishes_help_command() {
    let state = fresh_state();
    let bus = state.bus.clone();
    let mut sub = bus.subscribe("tui.command").await;
    let app = make_router(state);
    let resp = app
        .oneshot(post_json("/api/v1/tui/open-help", &serde_json::json!({})))
        .await
        .unwrap();
    let v = body_json(resp).await;
    assert_eq!(v, serde_json::json!(true));
    let env = sub.recv().await.unwrap();
    assert_eq!(env.properties["command"].as_str(), Some("help.show"));
}
