//! Workspace CRUD tests.

mod common;

use axum::http::StatusCode;
use common::{body_json, delete, fresh_state, get, make_router, post_json};
use tower::ServiceExt;

#[tokio::test]
async fn workspace_create_list_delete() {
    let state = fresh_state();
    let app = make_router(state);
    let payload = serde_json::json!({
        "id": "ws_1",
        "name": "main",
        "data": {}
    });
    let resp = app
        .clone()
        .oneshot(post_json("/api/v1/workspace", &payload))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let resp = app.clone().oneshot(get("/api/v1/workspace")).await.unwrap();
    let list = body_json(resp).await;
    assert_eq!(list.as_array().map(|a| a.len()), Some(1));

    let resp = app.oneshot(delete("/api/v1/workspace/ws_1")).await.unwrap();
    let v = body_json(resp).await;
    assert_eq!(v, serde_json::json!(true));
}

#[tokio::test]
async fn workspace_initial_list_empty() {
    let state = fresh_state();
    let app = make_router(state);
    let resp = app.oneshot(get("/api/v1/workspace")).await.unwrap();
    let list = body_json(resp).await;
    assert_eq!(list.as_array().map(|a| a.len()), Some(0));
}
