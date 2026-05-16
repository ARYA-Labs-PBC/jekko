//! Config GET/PUT round-trip tests.

mod common;

use axum::http::StatusCode;
use common::{body_json, fresh_state, get, make_router, put_json};
use tower::ServiceExt;

#[tokio::test]
async fn config_put_overwrites() {
    let state = fresh_state();
    let app = make_router(state);
    let payload = serde_json::json!({ "model": "anthropic/claude-opus" });
    let resp = app
        .clone()
        .oneshot(put_json("/api/v1/config", &payload))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let resp = app.oneshot(get("/api/v1/config")).await.unwrap();
    let value = body_json(resp).await;
    assert_eq!(value["model"].as_str(), Some("anthropic/claude-opus"));
}

#[tokio::test]
async fn config_providers_returns_object() {
    let state = fresh_state();
    let app = make_router(state);
    let resp = app.oneshot(get("/api/v1/config/providers")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let value = body_json(resp).await;
    assert!(value.is_object());
}
