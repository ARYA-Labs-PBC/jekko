//! CORS preflight behaviour.

mod common;

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use common::{fresh_state, make_router};
use tower::ServiceExt;

#[tokio::test]
async fn options_allows_localhost() {
    let app = make_router(fresh_state());
    let req = Request::builder()
        .uri("/api/v1/instance")
        .method(Method::OPTIONS)
        .header(header::ORIGIN, "http://localhost:5173")
        .header("access-control-request-method", "GET")
        .header("access-control-request-headers", "content-type")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert!(
        resp.status() == StatusCode::OK || resp.status() == StatusCode::NO_CONTENT,
        "got {}",
        resp.status()
    );
    assert!(
        resp.headers().contains_key("access-control-allow-origin"),
        "missing CORS header in response"
    );
}

#[tokio::test]
async fn jekko_subdomain_allowed() {
    let app = make_router(fresh_state());
    let req = Request::builder()
        .uri("/api/v1/instance")
        .method(Method::OPTIONS)
        .header(header::ORIGIN, "https://app.jekko.ai")
        .header("access-control-request-method", "GET")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert!(
        resp.headers().contains_key("access-control-allow-origin"),
        "missing CORS header"
    );
}

#[tokio::test]
async fn unknown_origin_blocked() {
    let app = make_router(fresh_state());
    let req = Request::builder()
        .uri("/api/v1/instance")
        .method(Method::OPTIONS)
        .header(header::ORIGIN, "https://attacker.example.com")
        .header("access-control-request-method", "GET")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    // tower-http CORS responds 200 to preflight but omits the allow header
    // when the origin is rejected.
    let acao = resp.headers().get("access-control-allow-origin");
    assert!(
        acao.is_none(),
        "allow-origin header should be absent for blocked origin"
    );
}
