/// Health classification stored per `(provider, model)` per user.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyHealth {
    /// Working as expected.
    Ready,
    /// HTTP 429 — temporarily rate-limited.
    RateLimited,
    /// HTTP 401/403 — credential rejected.
    AuthFailed,
    /// 5xx — upstream failure.
    ServerError,
}

impl KeyHealth {
    fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::RateLimited => "rate_limited",
            Self::AuthFailed => "auth_failed",
            Self::ServerError => "server_error",
        }
    }

    fn from_str(s: &str) -> Self {
        match s {
            "rate_limited" => Self::RateLimited,
            "auth_failed" => Self::AuthFailed,
            "server_error" => Self::ServerError,
            _ => Self::Ready,
        }
    }

    fn weight(self) -> f64 {
        match self {
            Self::Ready => 1.0,
            Self::RateLimited => 0.25,
            Self::ServerError => 0.45,
            Self::AuthFailed => 0.0,
        }
    }
}

/// Outcome categories used by [`KeyBalancer::record_failure`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureKind {
    /// HTTP 429.
    RateLimited,
    /// HTTP 401/403.
    AuthFailed,
    /// HTTP 5xx.
    ServerError,
    /// Anything else (timeout, transport, json decode...).
    Other,
}

impl FailureKind {
    fn classify_http(status: u16) -> Self {
        match status {
            401 | 403 => Self::AuthFailed,
            429 => Self::RateLimited,
            500..=599 => Self::ServerError,
            _ => Self::Other,
        }
    }

    fn cooldown_seconds(self, failures: u64) -> i64 {
        let n = failures.min(10) as i64;
        match self {
            Self::RateLimited => 30 * (1 + n),
            Self::AuthFailed => 60 * 60 * 24,
            Self::ServerError => 15 * (1 + n / 2),
            Self::Other => 5 * (1 + n),
        }
    }

    fn health(self) -> KeyHealth {
        match self {
            Self::RateLimited => KeyHealth::RateLimited,
            Self::AuthFailed => KeyHealth::AuthFailed,
            Self::ServerError | Self::Other => KeyHealth::ServerError,
        }
    }
}

/// One row of the `key_usage` table.
#[derive(Debug, Clone)]
pub struct KeyUsage {
    /// Number of selection attempts so far.
    pub attempts: u64,
    /// Number of failures so far.
    pub failures: u64,
    /// UNIX timestamp of the last failure, if any.
    pub last_failure_at: Option<i64>,
    /// UNIX timestamp until which this key is sidelined.
    pub cooldown_until: Option<i64>,
    /// Current health classification.
    pub status: KeyHealth,
}

impl Default for KeyUsage {
    fn default() -> Self {
        Self {
            attempts: 0,
            failures: 0,
            last_failure_at: None,
            cooldown_until: None,
            status: KeyHealth::Ready,
        }
    }
}
