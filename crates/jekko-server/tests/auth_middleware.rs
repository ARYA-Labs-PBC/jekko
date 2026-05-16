//! Auth middleware tests.

mod common;

use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use common::make_router;
use jekko_server::{AppState, AuthConfig};
use tower::ServiceExt;

fn state_with_key(key: &str) -> Arc<AppState> {
    Arc::new(AppState::new().with_auth(AuthConfig {
        api_key: Some(key.to_string()),
        ..AuthConfig::default()
    }))
}

#[tokio::test]
async fn missing_api_key_401() {
    let app = make_router(state_with_key("topsecret"));
    let req = Request::builder()
        .uri("/api/v1/instance")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn correct_api_key_200() {
    let app = make_router(state_with_key("topsecret"));
    let req = Request::builder()
        .uri("/api/v1/instance")
        .method("GET")
        .header("X-Jekko-API-Key", "topsecret")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn bearer_auth_accepted() {
    let app = make_router(state_with_key("topsecret"));
    let req = Request::builder()
        .uri("/api/v1/instance")
        .method("GET")
        .header(header::AUTHORIZATION, "Bearer topsecret")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn wrong_api_key_401() {
    let app = make_router(state_with_key("topsecret"));
    let req = Request::builder()
        .uri("/api/v1/instance")
        .method("GET")
        .header("X-Jekko-API-Key", "wrong")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn no_auth_required_pass_through() {
    let state = Arc::new(AppState::new());
    let app = make_router(state);
    let req = Request::builder()
        .uri("/api/v1/instance")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}
