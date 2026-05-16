//! Project identifier.
//!
//! Ported from `packages/jekko/src/project/schema.ts`.
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::session::IdParseError;

/// Project identifier (mirrors `ProjectID`).
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProjectId(String);

impl ProjectId {
    /// Identifier kind tag.
    pub const KIND: &'static str = "project";

    /// Sentinel identifier for the "global" project.
    pub const GLOBAL: &'static str = "global";

    /// Construct from any string-like value.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Build the canonical "global" project id.
    pub fn global() -> Self {
        Self(Self::GLOBAL.to_string())
    }

    /// Borrow the underlying string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume into the underlying [`String`].
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Display for ProjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for ProjectId {
    type Err = IdParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(IdParseError {
                kind: Self::KIND,
                value: s.to_string(),
            });
        }
        Ok(Self(s.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn global_project_id() {
        assert_eq!(ProjectId::global().to_string(), "global");
    }

    #[test]
    fn parse_round_trip() {
        let id: ProjectId = "myproj".parse().unwrap();
        assert_eq!(id.as_str(), "myproj");
    }
}
