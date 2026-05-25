//! Canonical forbidden-content patterns.
//!
//! These markers MUST NOT appear in produced ZYAL artifacts. Both the producer
//! (jankurai-runner publishes them in `ArtifactContract.forbidden_content`) and
//! the auditor (zyalc scans every artifact byte for any match) consume this
//! single list. Adding a pattern here covers both surfaces.
//!
//! The list mixes two categories:
//! 1. **Artifact-shape markers** that indicate a forbidden artifact structure
//!    (e.g. raw chain-of-thought, fixture-target leakage).
//! 2. **Credential-leakage markers** that indicate a real provider key or
//!    GitHub token escaped into an artifact.

/// All forbidden patterns. Substring match on raw artifact bytes (case-sensitive).
pub const FORBIDDEN_PATTERNS: &[&str] = &[
    // Artifact-shape markers (producer / packet contract):
    "raw_chain_of_thought",
    "fixture_target_values_in_model_visible_artifacts",
    "process_env_credentials",
    ".env.jnoccio_credentials",
    "jnoccio-local",
    // Credential-leakage markers (auditor scan):
    "OPENAI_API_KEY=",
    "ANTHROPIC_API_KEY=",
    "GEMINI_API_KEY=",
    "OPENROUTER_API_KEY=",
    "MISTRAL_API_KEY=",
    "GROQ_API_KEY=",
    "FIREWORKS_API_KEY=",
    "SAMBANOVA_API_KEY=",
    "CEREBRAS_API_KEY=",
    "sk-",
    "sk-or-",
    "gsk_",
    "ghp_",
];

/// Return the first forbidden pattern present in `text`, or `None`.
pub fn contains_any_forbidden(text: &str) -> Option<&'static str> {
    FORBIDDEN_PATTERNS
        .iter()
        .copied()
        .find(|pattern| text.contains(pattern))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pattern_count_is_eighteen() {
        // If this changes intentionally, update the producer/auditor tests too.
        assert_eq!(FORBIDDEN_PATTERNS.len(), 18);
    }

    #[test]
    fn detects_artifact_shape_marker() {
        assert_eq!(
            contains_any_forbidden("the artifact contains raw_chain_of_thought here"),
            Some("raw_chain_of_thought")
        );
    }

    #[test]
    fn detects_credential_leakage() {
        assert_eq!(
            contains_any_forbidden("OPENAI_API_KEY=sk-abc123"),
            Some("OPENAI_API_KEY=")
        );
        assert_eq!(
            contains_any_forbidden("token: ghp_redactedabc"),
            Some("ghp_")
        );
    }

    #[test]
    fn clean_text_returns_none() {
        assert_eq!(
            contains_any_forbidden("a perfectly fine artifact summary"),
            None
        );
    }
}
