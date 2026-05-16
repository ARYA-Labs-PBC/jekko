//! Experimental flag tests.

mod common;

use axum::http::StatusCode;
use common::{body_json, fresh_state, get, make_router, put_json};
use tower::ServiceExt;

#[tokio::test]
async fn flag_round_trip() {
    let state = fresh_state();
    let app = make_router(state);
    let body = serde_json::json!({ "value": true });
    let resp = app
        .clone()
        .oneshot(put_json("/api/v1/experimental/batch_tool", &body))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let resp = app
        .oneshot(get("/api/v1/experimental/batch_tool"))
        .await
        .unwrap();
    let v = body_json(resp).await;
    assert_eq!(v, serde_json::json!(true));
}

#[tokio::test]
async fn flag_list_empty_by_default() {
    let state = fresh_state();
    let app = make_router(state);
    let resp = app.oneshot(get("/api/v1/experimental")).await.unwrap();
    let v = body_json(resp).await;
    assert!(v.is_object());
    assert_eq!(v.as_object().map(|o| o.len()), Some(0));
}
