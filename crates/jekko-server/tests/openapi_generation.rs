//! OpenAPI generation smoke test.

mod common;

use axum::http::StatusCode;
use common::{body_json, fresh_state, get, make_router};
use tower::ServiceExt;

#[tokio::test]
async fn openapi_doc_includes_expected_paths() {
    let state = fresh_state();
    let app = make_router(state);
    let resp = app.oneshot(get("/api/openapi.json")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let doc = body_json(resp).await;

    let paths = doc
        .get("paths")
        .and_then(|v| v.as_object())
        .expect("paths object");
    let expected = [
        "/api/v1/instance",
        "/api/v1/config",
        "/api/v1/session",
        "/api/v1/file/content",
        "/api/v1/daemon",
        "/api/v1/sync",
        "/api/v1/tui/append-prompt",
        "/api/v1/provider",
        "/api/v1/permission",
        "/api/v1/question",
        "/api/v1/mcp",
        "/api/v1/workspace",
        "/api/v1/experimental",
        "/api/v1/events",
    ];
    for path in expected {
        assert!(
            paths.contains_key(path),
            "expected OpenAPI doc to contain path {path}; got keys: {:?}",
            paths.keys().collect::<Vec<_>>()
        );
    }
}

#[tokio::test]
async fn openapi_doc_is_valid_object() {
    let state = fresh_state();
    let app = make_router(state);
    let resp = app.oneshot(get("/api/openapi.json")).await.unwrap();
    let doc = body_json(resp).await;
    assert!(doc.is_object());
    assert!(doc.get("openapi").is_some());
    assert!(doc.get("info").is_some());
}
