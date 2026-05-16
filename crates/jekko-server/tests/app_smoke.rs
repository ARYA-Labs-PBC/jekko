//! Smoke tests for the top-level Axum router.

mod common;

use axum::http::StatusCode;
use common::{body_json, fresh_state, get, make_router};
use tower::ServiceExt;

#[tokio::test]
async fn instance_endpoint_returns_200() {
    let app = make_router(fresh_state());
    let resp = app.oneshot(get("/api/v1/instance")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert!(body.get("id").is_some());
    assert!(body.get("path").is_some());
}

#[tokio::test]
async fn path_endpoint_includes_home() {
    let app = make_router(fresh_state());
    let resp = app.oneshot(get("/api/v1/instance/path")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert!(body.get("home").is_some());
    assert!(body.get("directory").is_some());
}

#[tokio::test]
async fn unknown_route_404() {
    let app = make_router(fresh_state());
    let resp = app.oneshot(get("/api/v1/no-such-thing")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn config_get_returns_object() {
    let app = make_router(fresh_state());
    let resp = app.oneshot(get("/api/v1/config")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert!(body.is_object());
}
