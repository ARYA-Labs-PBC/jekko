//! File route smoke tests.

mod common;

use axum::http::StatusCode;
use common::{body_json, fresh_state, get, make_router};
use tower::ServiceExt;

#[tokio::test]
async fn file_content_reads_existing_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("hello.txt");
    std::fs::write(&path, b"hi there").unwrap();
    let state = fresh_state();
    let app = make_router(state);
    let uri = format!(
        "/api/v1/file/content?path={}",
        urlencoding(&path.display().to_string())
    );
    let resp = app.oneshot(get(&uri)).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let text = body_json(resp).await;
    assert_eq!(text.as_str(), Some("hi there"));
}

#[tokio::test]
async fn file_list_returns_entries() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.txt"), b"").unwrap();
    std::fs::write(dir.path().join("b.txt"), b"").unwrap();
    let state = fresh_state();
    let app = make_router(state);
    let uri = format!(
        "/api/v1/file/list?path={}",
        urlencoding(&dir.path().display().to_string())
    );
    let resp = app.oneshot(get(&uri)).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let list = body_json(resp).await;
    assert_eq!(list.as_array().map(|a| a.len()), Some(2));
}

fn urlencoding(input: &str) -> String {
    let mut out = String::new();
    for byte in input.as_bytes() {
        match *byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'.' | b'_' | b'~' | b'-' => {
                out.push(*byte as char)
            }
            b'/' => out.push('/'),
            other => out.push_str(&format!("%{:02X}", other)),
        }
    }
    out
}
