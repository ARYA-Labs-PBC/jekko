//! Authentication middleware.
//!
//! Ported behaviourally from `packages/jekko/src/server/auth.ts`. We look
//! for either:
//!
//! - `X-Jekko-API-Key: <token>` header (preferred), or
//! - `Authorization: Bearer <token>`, or
//! - `Authorization: Basic <base64>` (basic username:password form).
//!
//! When `JEKKO_API_KEY` is unset and no [`AuthConfig::password`] is supplied,
//! requests pass through (mirrors the TS `required` predicate which returns
//! `false` for empty configurations).

use std::sync::Arc;

use axum::body::Body;
use axum::extract::State;
use axum::http::{header, HeaderMap, Request};
use axum::middleware::Next;
use axum::response::Response;
use serde::{Deserialize, Serialize};

use crate::error::ServerError;
use crate::state::AppState;

/// Header name used by the Jekko TS server.
pub const API_KEY_HEADER: &str = "X-Jekko-API-Key";

/// Auth configuration. All fields are `None` by default which disables auth.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Token (also accepted via `Authorization: Bearer`).
    pub api_key: Option<String>,
    /// Optional basic-auth username (defaults to `"jekko"`).
    pub username: Option<String>,
    /// Optional basic-auth password.
    pub password: Option<String>,
}

impl AuthConfig {
    /// Construct from process environment. Reads `JEKKO_API_KEY`,
    /// `JEKKO_SERVER_USERNAME`, and `JEKKO_SERVER_PASSWORD`.
    pub fn from_env() -> Self {
        Self {
            api_key: std::env::var("JEKKO_API_KEY")
                .ok()
                .filter(|s| !s.is_empty()),
            username: std::env::var("JEKKO_SERVER_USERNAME")
                .ok()
                .filter(|s| !s.is_empty()),
            password: std::env::var("JEKKO_SERVER_PASSWORD")
                .ok()
                .filter(|s| !s.is_empty()),
        }
    }

    /// True if any of the credentials are configured.
    pub fn required(&self) -> bool {
        self.api_key.is_some() || self.password.is_some()
    }

    /// Verify request headers against this config. Returns
    /// `Err(Unauthorized)` if auth is required and not satisfied.
    pub fn verify(&self, headers: &HeaderMap) -> Result<(), ServerError> {
        if !self.required() {
            return Ok(());
        }

        // X-Jekko-API-Key header (preferred).
        if let Some(expected) = self.api_key.as_deref() {
            if let Some(provided) = headers.get(API_KEY_HEADER).and_then(|v| v.to_str().ok()) {
                if provided == expected {
                    return Ok(());
                }
            }
            // Also accept the same token via `Authorization: Bearer …`.
            if let Some(auth) = headers
                .get(header::AUTHORIZATION)
                .and_then(|v| v.to_str().ok())
            {
                if let Some(token) = auth.strip_prefix("Bearer ") {
                    if token == expected {
                        return Ok(());
                    }
                }
            }
        }

        // Basic auth branch.
        if let Some(expected_pwd) = self.password.as_deref() {
            let expected_user = self.username.as_deref().unwrap_or("jekko");
            if let Some(auth) = headers
                .get(header::AUTHORIZATION)
                .and_then(|v| v.to_str().ok())
            {
                if let Some(encoded) = auth.strip_prefix("Basic ") {
                    let decoded = base64_decode_pair(encoded);
                    if let Some((user, pwd)) = decoded {
                        if user == expected_user && pwd == expected_pwd {
                            return Ok(());
                        }
                    }
                }
            }
        }

        Err(ServerError::Unauthorized)
    }
}

/// Axum middleware function: enforces [`AuthConfig::verify`] before delegating
/// to the handler.
pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, ServerError> {
    state.auth.verify(request.headers())?;
    Ok(next.run(request).await)
}

fn base64_decode_pair(input: &str) -> Option<(String, String)> {
    let decoded = base64_decode(input)?;
    let pair = String::from_utf8(decoded).ok()?;
    let (u, p) = pair.split_once(':')?;
    Some((u.to_string(), p.to_string()))
}

/// Minimal base64 decoder. Sufficient for `Basic`-auth pairs.
fn base64_decode(input: &str) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut buf: u32 = 0;
    let mut bits: u8 = 0;
    for byte in input.trim().bytes() {
        let v: u8 = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => break,
            _ => return None,
        };
        buf = (buf << 6) | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn no_auth_required_passes() {
        let cfg = AuthConfig::default();
        let h = HeaderMap::new();
        assert!(cfg.verify(&h).is_ok());
    }

    #[test]
    fn api_key_match() {
        let cfg = AuthConfig {
            api_key: Some("topsecret".into()),
            ..AuthConfig::default()
        };
        let mut h = HeaderMap::new();
        h.insert(API_KEY_HEADER, HeaderValue::from_static("topsecret"));
        assert!(cfg.verify(&h).is_ok());
    }

    #[test]
    fn api_key_mismatch() {
        let cfg = AuthConfig {
            api_key: Some("topsecret".into()),
            ..AuthConfig::default()
        };
        let mut h = HeaderMap::new();
        h.insert(API_KEY_HEADER, HeaderValue::from_static("nope"));
        assert!(matches!(cfg.verify(&h), Err(ServerError::Unauthorized)));
    }

    #[test]
    fn bearer_accepted() {
        let cfg = AuthConfig {
            api_key: Some("topsecret".into()),
            ..AuthConfig::default()
        };
        let mut h = HeaderMap::new();
        h.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer topsecret"),
        );
        assert!(cfg.verify(&h).is_ok());
    }

    #[test]
    fn basic_auth_match() {
        let cfg = AuthConfig {
            password: Some("hunter2".into()),
            username: Some("jekko".into()),
            ..AuthConfig::default()
        };
        // "jekko:hunter2" → "amVra286aHVudGVyMg==" without padding works too
        let mut h = HeaderMap::new();
        h.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Basic amVra286aHVudGVyMg=="),
        );
        assert!(cfg.verify(&h).is_ok());
    }
}
