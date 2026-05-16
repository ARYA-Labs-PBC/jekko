//! Permission rules and identifiers.
//!
//! Ported from `packages/jekko/src/permission/schema.ts` and the runtime
//! shape produced by `packages/jekko/src/config/permission.ts` after normalisation.
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Coarse-grained permission decision.
///
/// Mirrors `Action` in `config/permission.ts`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PermissionAction {
    /// Prompt the user before allowing.
    Ask,
    /// Allow without prompting.
    Allow,
    /// Deny outright.
    Deny,
}

/// Per-target permission map (`{ "*": "allow", "read:/foo": "ask" }`).
///
/// Keys are arbitrary user-supplied target strings; order is preserved by
/// using a [`BTreeMap`] (deterministic) — the TS code relied on
/// `propertyOrder: "original"` for precedence, callers should re-implement
/// that logic at use-site by reading the keys in insertion order from the
/// configuration source.
pub type PermissionObject = BTreeMap<String, PermissionAction>;

/// Either a coarse action or a per-target map.
///
/// Mirrors `Rule` in `config/permission.ts`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PermissionRule {
    /// Coarse action applied to every target.
    Action(PermissionAction),
    /// Per-target action map.
    Object(PermissionObject),
}

impl PermissionRule {
    /// Resolve a permission rule for a given target. The catch-all `"*"` entry
    /// matches when no exact-match key is present.
    pub fn resolve(&self, target: &str) -> Option<PermissionAction> {
        match self {
            Self::Action(action) => Some(*action),
            Self::Object(map) => match map.get(target).copied() {
                Some(action) => Some(action),
                None => map.get("*").copied(),
            },
        }
    }
}

/// Permission configuration after normalisation.
///
/// Ported from the `Info` type in `config/permission.ts`. Known tool keys
/// receive named fields; unknown keys are preserved in [`PermissionConfig::extra`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionConfig {
    /// `read` permission.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read: Option<PermissionRule>,
    /// `edit` permission.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edit: Option<PermissionRule>,
    /// `glob` permission.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glob: Option<PermissionRule>,
    /// `grep` permission.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grep: Option<PermissionRule>,
    /// `list` permission.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list: Option<PermissionRule>,
    /// `bash` permission.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bash: Option<PermissionRule>,
    /// `task` permission.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task: Option<PermissionRule>,
    /// `external_directory` permission.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_directory: Option<PermissionRule>,
    /// `todowrite` permission (single action).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub todowrite: Option<PermissionAction>,
    /// `question` permission (single action).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub question: Option<PermissionAction>,
    /// `research` permission (single action).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub research: Option<PermissionAction>,
    /// `webfetch` permission (single action).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub webfetch: Option<PermissionAction>,
    /// `websearch` permission (single action).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub websearch: Option<PermissionAction>,
    /// `lsp` permission.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lsp: Option<PermissionRule>,
    /// `doom_loop` permission (single action).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doom_loop: Option<PermissionAction>,
    /// `skill` permission.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skill: Option<PermissionRule>,
    /// Catch-all for unknown permission keys (preserved verbatim).
    #[serde(flatten)]
    pub extra: BTreeMap<String, PermissionRule>,
}

/// Either a shorthand action ("allow") or a full [`PermissionConfig`] object.
///
/// Mirrors the `Action | InputObject` union in `config/permission.ts`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
#[allow(clippy::large_enum_variant)]
pub enum PermissionInput {
    /// Shorthand `"allow"`/`"deny"`/`"ask"` form.
    Action(PermissionAction),
    /// Full object form.
    Object(PermissionConfig),
}

impl PermissionInput {
    /// Normalise into a [`PermissionConfig`] (the shorthand form maps the
    /// action against `"*"` in [`PermissionConfig::extra`]).
    pub fn normalize(self) -> PermissionConfig {
        match self {
            Self::Object(config) => config,
            Self::Action(action) => {
                let mut config = PermissionConfig::default();
                config
                    .extra
                    .insert("*".to_string(), PermissionRule::Action(action));
                config
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rule_resolves_action() {
        let rule = PermissionRule::Action(PermissionAction::Allow);
        assert_eq!(rule.resolve("anything"), Some(PermissionAction::Allow));
    }

    #[test]
    fn rule_resolves_object_with_wildcard() {
        let mut map = PermissionObject::new();
        map.insert("read".to_string(), PermissionAction::Allow);
        map.insert("*".to_string(), PermissionAction::Ask);
        let rule = PermissionRule::Object(map);
        assert_eq!(rule.resolve("read"), Some(PermissionAction::Allow));
        assert_eq!(rule.resolve("write"), Some(PermissionAction::Ask));
    }

    #[test]
    fn input_action_normalises_to_wildcard() {
        let cfg = PermissionInput::Action(PermissionAction::Deny).normalize();
        assert_eq!(
            cfg.extra.get("*"),
            Some(&PermissionRule::Action(PermissionAction::Deny))
        );
    }

    #[test]
    fn input_object_round_trip() {
        let input = PermissionInput::Object(PermissionConfig {
            bash: Some(PermissionRule::Action(PermissionAction::Allow)),
            ..PermissionConfig::default()
        });
        let json = serde_json::to_string(&input).unwrap();
        let parsed: PermissionInput = serde_json::from_str(&json).unwrap();
        assert_eq!(input, parsed);
    }
}
