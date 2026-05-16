//! Permission queue tests.

mod common;

use axum::http::StatusCode;
use common::{body_json, fresh_state, get, make_router, post_json};
use jekko_runtime::permission::{new_request_id, PermissionRequest};
use tower::ServiceExt;

#[tokio::test]
async fn list_empty_by_default() {
    let state = fresh_state();
    let app = make_router(state);
    let resp = app.oneshot(get("/api/v1/permission")).await.unwrap();
    let v = body_json(resp).await;
    assert!(v.is_array());
    assert_eq!(v.as_array().map(|a| a.len()), Some(0));
}

#[tokio::test]
async fn reply_to_pending_request() {
    let state = fresh_state();
    let perms = state.permissions.clone();
    let app = make_router(state);

    // Spawn an ask that will block until the reply arrives.
    let request_id = new_request_id();
    let req = PermissionRequest {
        id: request_id.clone(),
        session_id: "ses_test".into(),
        permission: "bash".into(),
        patterns: vec!["echo".into()],
        metadata: serde_json::json!({}),
        always: vec![],
    };
    let perms_clone = perms.clone();
    let ask_handle = tokio::spawn(async move { perms_clone.ask(req, vec![]).await });

    // Give the ask a chance to register before replying.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let resp = app
        .oneshot(post_json(
            &format!("/api/v1/permission/{request_id}"),
            &serde_json::json!({ "reply": "once" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let reply = ask_handle.await.unwrap().unwrap();
    assert!(matches!(
        reply,
        jekko_runtime::permission::PermissionReply::Once
    ));
}
