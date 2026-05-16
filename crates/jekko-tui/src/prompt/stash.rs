//! Per-route draft stash.
//!
//! Ports the in-memory portion of
//! `packages/jekko/src/cli/cmd/tui/component/prompt/stash.tsx`. Drafts are
//! keyed by an opaque `RouteKey` (the host crate decides whether that's the
//! route enum stringified, a session id, or both).

use std::collections::HashMap;

/// Opaque per-route identifier. Defined as a newtype so callers can pass
/// strongly-typed keys instead of bare strings.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RouteKey(pub String);

impl RouteKey {
    /// Build a key from any string-like input.
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Borrow the underlying string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for RouteKey {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for RouteKey {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// In-memory draft stash. Keyed by [`RouteKey`].
#[derive(Clone, Debug, Default)]
pub struct PromptStash {
    drafts: HashMap<RouteKey, String>,
}

impl PromptStash {
    /// Construct an empty stash.
    pub fn new() -> Self {
        Self::default()
    }

    /// Save the live buffer for `route`. An empty `text` removes the entry.
    pub fn save(&mut self, route: impl Into<RouteKey>, text: impl Into<String>) {
        let key = route.into();
        let text = text.into();
        if text.is_empty() {
            self.drafts.remove(&key);
        } else {
            self.drafts.insert(key, text);
        }
    }

    /// Restore (and remove) the draft for `route`.
    pub fn restore(&mut self, route: impl Into<RouteKey>) -> Option<String> {
        let key = route.into();
        self.drafts.remove(&key)
    }

    /// Peek at the current draft without removing it.
    pub fn peek(&self, route: impl Into<RouteKey>) -> Option<&str> {
        let key = route.into();
        self.drafts.get(&key).map(String::as_str)
    }

    /// Drop every stored draft.
    pub fn clear(&mut self) {
        self.drafts.clear();
    }

    /// Number of stored drafts.
    pub fn len(&self) -> usize {
        self.drafts.len()
    }

    /// Whether the stash holds no drafts.
    pub fn is_empty(&self) -> bool {
        self.drafts.is_empty()
    }
}
