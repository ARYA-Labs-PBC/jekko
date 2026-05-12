//! Permission glob matcher.
//!
//! Whitelist semantics: a command must match at least one `allowed` glob AND
//! must NOT match any `denied` glob. Denial wins on intersection. The check is
//! intentionally strict — if no allow rule matches, the command is denied
//! rather than allowed-by-default.

use globset::{Glob, GlobSet, GlobSetBuilder};

#[derive(Debug)]
pub struct Matcher {
    allowed: GlobSet,
    denied: GlobSet,
    denied_patterns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Allow,
    DeniedByPattern { pattern: String },
    NoAllowMatched,
}

impl Matcher {
    pub fn new(
        allowed: &[String],
        denied: &[String],
    ) -> Result<Self, globset::Error> {
        let allowed_set = build_set(allowed)?;
        let denied_set = build_set(denied)?;
        Ok(Self {
            allowed: allowed_set,
            denied: denied_set,
            denied_patterns: denied.to_vec(),
        })
    }

    /// Evaluate an argv (joined with single spaces, no quoting) against the
    /// allow/deny lists. The caller is responsible for constructing a stable
    /// rendering — see `render_argv`.
    pub fn evaluate(&self, argv: &[String]) -> Decision {
        let rendered = render_argv(argv);
        if let Some(idx) = self.denied.matches(&rendered).first().copied() {
            // `idx` came from `denied.matches()`, which is built from the same
            // pattern vector, so the bounds check below is purely defensive.
            let pattern = match self.denied_patterns.get(idx) {
                Some(p) => p.clone(),
                None => String::new(),
            };
            return Decision::DeniedByPattern { pattern };
        }
        if self.allowed.is_match(&rendered) {
            Decision::Allow
        } else {
            Decision::NoAllowMatched
        }
    }
}

/// Stable argv → string rendering used for glob matching. Joins with single
/// spaces; no quoting (globs are applied to the cleartext form).
pub fn render_argv(argv: &[String]) -> String {
    argv.join(" ")
}

fn build_set(patterns: &[String]) -> Result<GlobSet, globset::Error> {
    let mut builder = GlobSetBuilder::new();
    for p in patterns {
        builder.add(Glob::new(p)?);
    }
    builder.build()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matcher(allow: &[&str], deny: &[&str]) -> Matcher {
        let allow: Vec<String> = allow.iter().map(|s| s.to_string()).collect();
        let deny: Vec<String> = deny.iter().map(|s| s.to_string()).collect();
        Matcher::new(&allow, &deny).expect("build")
    }

    fn argv(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn allow_match_passes() {
        let m = matcher(&["git status*", "just *"], &[]);
        assert_eq!(m.evaluate(&argv(&["git", "status"])), Decision::Allow);
        assert_eq!(m.evaluate(&argv(&["just", "fast"])), Decision::Allow);
    }

    #[test]
    fn deny_wins_over_allow() {
        let m = matcher(&["git *"], &["git push*"]);
        let d = m.evaluate(&argv(&["git", "push", "origin", "main"]));
        assert!(matches!(d, Decision::DeniedByPattern { .. }));
    }

    #[test]
    fn missing_allow_is_denied() {
        let m = matcher(&["just *"], &[]);
        assert_eq!(
            m.evaluate(&argv(&["rm", "-rf", "/"])),
            Decision::NoAllowMatched
        );
    }

    #[test]
    fn cargo_install_blocked_by_intersection() {
        // `cargo *` allows `cargo install`; an explicit deny for `cargo install*`
        // must still block it.
        let m = matcher(&["cargo *"], &["cargo install*"]);
        let d = m.evaluate(&argv(&["cargo", "install", "tokei"]));
        assert!(
            matches!(d, Decision::DeniedByPattern { .. }),
            "deny must win on intersection, got {d:?}"
        );
    }
}
