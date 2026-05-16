//! Smoke tests for `WebSearchTool`.
//!
//! The real backend (Brave Search) requires a live API key. Both tests in
//! this file are `#[ignore]` by default. The first test exercises the
//! "missing key" code-path with no network access; the second exercises
//! the live API and is opt-in via `cargo test -- --ignored` plus
//! `BRAVE_SEARCH_API_KEY` set.

use jekko_runtime::tool::{websearch::BRAVE_API_KEY_ENV, Tool, ToolContext, WebSearchTool};

#[tokio::test]
#[ignore = "manual: depends on BRAVE_SEARCH_API_KEY"]
async fn live_search_returns_results() {
    let Ok(_) = std::env::var(BRAVE_API_KEY_ENV) else {
        eprintln!("skip: {BRAVE_API_KEY_ENV} not set");
        return;
    };
    let out = WebSearchTool
        .execute(
            serde_json::json!({ "query": "rust language", "count": 1 }),
            ToolContext::bare("."),
        )
        .await
        .expect("websearch should succeed");
    let results = out.metadata["results"].as_array().expect("results array");
    assert!(!results.is_empty(), "expected at least one result");
    assert!(out.title.contains("Web search"));
}

#[tokio::test]
async fn missing_api_key_errors_clearly() {
    // Snapshot + clear env to avoid relying on caller env.
    let prior = std::env::var(BRAVE_API_KEY_ENV).ok();
    // SAFETY: tokio test runs the body in the current task; we restore env
    // before returning. Other tests in this file don't share state.
    unsafe {
        std::env::remove_var(BRAVE_API_KEY_ENV);
    }
    let err = WebSearchTool
        .execute(
            serde_json::json!({ "query": "rust" }),
            ToolContext::bare("."),
        )
        .await
        .expect_err("expected error without API key");
    if let Some(v) = prior {
        unsafe {
            std::env::set_var(BRAVE_API_KEY_ENV, v);
        }
    }
    let msg = format!("{err}");
    assert!(
        msg.contains(BRAVE_API_KEY_ENV),
        "error should mention env var, got: {msg}"
    );
}
