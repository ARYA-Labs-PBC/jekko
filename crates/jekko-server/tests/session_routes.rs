//! Session CRUD round-trip tests against the in-memory store.

mod common;

use axum::http::StatusCode;
use common::{body_json, delete, fresh_state, get, make_router, post_json};
use tower::ServiceExt;

#[tokio::test]
async fn create_and_get_session() {
    let state = fresh_state();
    let app = make_router(state);
    let body = serde_json::json!({
        "project_id": "proj_1",
        "directory": "/tmp",
        "title": "test"
    });
    let resp = app
        .clone()
        .oneshot(post_json("/api/v1/session", &body))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let created = body_json(resp).await;
    let id = created["id"].as_str().expect("id present").to_string();

    let resp = app
        .oneshot(get(&format!("/api/v1/session/{id}")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let fetched = body_json(resp).await;
    assert_eq!(fetched["id"].as_str(), Some(id.as_str()));
    assert_eq!(fetched["title"].as_str(), Some("test"));
}

#[tokio::test]
async fn append_and_list_messages() {
    let state = fresh_state();
    let app = make_router(state);
    let body = serde_json::json!({
        "project_id": "proj_1",
        "directory": "/tmp",
        "title": "msgs"
    });
    let resp = app
        .clone()
        .oneshot(post_json("/api/v1/session", &body))
        .await
        .unwrap();
    let created = body_json(resp).await;
    let id = created["id"].as_str().unwrap().to_string();

    let msg = serde_json::json!({ "role": "user", "data": { "text": "hi" } });
    let resp = app
        .clone()
        .oneshot(post_json(&format!("/api/v1/session/{id}/message"), &msg))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let resp = app
        .oneshot(get(&format!("/api/v1/session/{id}/message")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let listed = body_json(resp).await;
    assert!(listed.is_array());
    assert_eq!(listed.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn delete_session_returns_true() {
    let state = fresh_state();
    let app = make_router(state);
    let body = serde_json::json!({
        "project_id": "proj_1",
        "directory": "/tmp"
    });
    let resp = app
        .clone()
        .oneshot(post_json("/api/v1/session", &body))
        .await
        .unwrap();
    let created = body_json(resp).await;
    let id = created["id"].as_str().unwrap().to_string();
    let resp = app
        .oneshot(delete(&format!("/api/v1/session/{id}")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v = body_json(resp).await;
    assert_eq!(v, serde_json::json!(true));
}

#[tokio::test]
async fn missing_session_404() {
    let state = fresh_state();
    let app = make_router(state);
    let resp = app
        .oneshot(get("/api/v1/session/nonexistent"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
