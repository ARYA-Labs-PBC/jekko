//! CORS configuration + layer construction.
//!
//! Ported from `packages/jekko/src/server/cors.ts`. Defaults allow
//! `http://localhost:*` / `http://127.0.0.1:*` plus any subdomain of
//! `jekko.ai` over HTTPS. Additional origins can be allow-listed via
//! [`CorsConfig::origins`].

use axum::http::{HeaderName, HeaderValue, Method};
use http::header::{AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use tower_http::cors::{AllowOrigin, CorsLayer};

/// CORS allow-list. Empty by default — wildcard host rules below still
/// apply.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CorsConfig {
    /// Extra explicit origins (e.g. `https://app.example.dev`).
    pub origins: Vec<String>,
}

impl CorsConfig {
    /// Append `origin` to the allow-list.
    pub fn allow(mut self, origin: impl Into<String>) -> Self {
        self.origins.push(origin.into());
        self
    }

    /// True if the supplied `Origin` header is acceptable.
    pub fn is_allowed(&self, origin: &str) -> bool {
        if origin.is_empty() {
            return true;
        }
        if origin.starts_with("http://localhost:")
            || origin.starts_with("http://127.0.0.1:")
            || origin == "http://localhost"
            || origin == "http://127.0.0.1"
        {
            return true;
        }
        if is_jekko_origin(origin) {
            return true;
        }
        self.origins.iter().any(|allowed| allowed == origin)
    }

    /// Build a [`CorsLayer`] from this config.
    pub fn layer(&self) -> CorsLayer {
        let allow_list = self.origins.clone();
        CorsLayer::new()
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::PATCH,
                Method::DELETE,
                Method::OPTIONS,
            ])
            .allow_headers([
                CONTENT_TYPE,
                AUTHORIZATION,
                HeaderName::from_static("x-jekko-api-key"),
            ])
            .allow_origin(AllowOrigin::predicate(
                move |origin: &HeaderValue, _request_parts: &http::request::Parts| {
                    let Ok(origin_str) = origin.to_str() else {
                        return false;
                    };
                    if origin_str.starts_with("http://localhost:")
                        || origin_str.starts_with("http://127.0.0.1:")
                        || origin_str == "http://localhost"
                        || origin_str == "http://127.0.0.1"
                    {
                        return true;
                    }
                    if is_jekko_origin(origin_str) {
                        return true;
                    }
                    allow_list.iter().any(|allowed| allowed == origin_str)
                },
            ))
    }
}

fn is_jekko_origin(input: &str) -> bool {
    // Mirrors `^https:\/\/([a-z0-9-]+\.)*jekko\.ai$`.
    let Some(rest) = input.strip_prefix("https://") else {
        return false;
    };
    if rest.is_empty() {
        return false;
    }
    if rest == "jekko.ai" {
        return true;
    }
    if let Some(prefix) = rest.strip_suffix(".jekko.ai") {
        if prefix.is_empty() {
            return false;
        }
        return prefix.split('.').all(|seg| {
            !seg.is_empty()
                && seg
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        });
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn localhost_allowed() {
        let cfg = CorsConfig::default();
        assert!(cfg.is_allowed("http://localhost:3000"));
        assert!(cfg.is_allowed("http://127.0.0.1:8080"));
    }

    #[test]
    fn jekko_subdomain_allowed() {
        let cfg = CorsConfig::default();
        assert!(cfg.is_allowed("https://app.jekko.ai"));
        assert!(cfg.is_allowed("https://jekko.ai"));
        assert!(!cfg.is_allowed("https://evil.example.com"));
    }

    #[test]
    fn explicit_allow_list() {
        let cfg = CorsConfig::default().allow("https://app.example.dev");
        assert!(cfg.is_allowed("https://app.example.dev"));
        assert!(!cfg.is_allowed("https://other.example.dev"));
    }
}
