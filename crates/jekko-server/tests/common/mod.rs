//! Shared test helpers.
#![allow(dead_code)]

use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, Request};
use axum::Router;
use jekko_server::{router, AppState};

/// Build a fresh default `AppState` for one test.
pub fn fresh_state() -> Arc<AppState> {
    Arc::new(AppState::new())
}

/// Build the full router on top of `state`.
pub fn make_router(state: Arc<AppState>) -> Router {
    router(state)
}

/// Construct a JSON-typed GET request.
pub fn get(uri: &str) -> Request<Body> {
    Request::builder()
        .uri(uri)
        .method("GET")
        .header(header::ACCEPT, "application/json")
        .body(Body::empty())
        .unwrap()
}

/// Construct a JSON-typed POST request.
pub fn post_json(uri: &str, body: &serde_json::Value) -> Request<Body> {
    Request::builder()
        .uri(uri)
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

/// Construct a JSON-typed PUT request.
pub fn put_json(uri: &str, body: &serde_json::Value) -> Request<Body> {
    Request::builder()
        .uri(uri)
        .method("PUT")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

/// Construct a DELETE request.
pub fn delete(uri: &str) -> Request<Body> {
    Request::builder()
        .uri(uri)
        .method("DELETE")
        .body(Body::empty())
        .unwrap()
}

/// Drain a response body into bytes.
pub async fn body_bytes(resp: axum::response::Response) -> Vec<u8> {
    use http_body_util::BodyExt;
    let collected = resp.into_body().collect().await.unwrap();
    collected.to_bytes().to_vec()
}

/// Drain a response body into a JSON Value.
pub async fn body_json(resp: axum::response::Response) -> serde_json::Value {
    let bytes = body_bytes(resp).await;
    serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
}
