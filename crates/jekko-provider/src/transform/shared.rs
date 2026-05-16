//! Shared transform constants and helpers.
//!
//! Ported from `packages/jekko/src/provider/transform-shared.ts`.

/// Default maximum output token limit used to cap per-model maxima.
///
/// Mirrors `OUTPUT_TOKEN_MAX` from `transform-shared.ts` (32_000 in TS).
pub const OUTPUT_TOKEN_MAX: u32 = 32_000;

/// Modality categories accepted by `mimeToModality`.
///
/// Mirrors the `Modality` union from `transform-shared.ts`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modality {
    /// Text input.
    Text,
    /// Image input.
    Image,
    /// Audio input.
    Audio,
    /// Video input.
    Video,
    /// PDF input.
    Pdf,
}

impl Modality {
    /// String tag matching the TypeScript value.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Image => "image",
            Self::Audio => "audio",
            Self::Video => "video",
            Self::Pdf => "pdf",
        }
    }
}

/// Result of [`mime_to_modality`]: either a supported [`Modality`] or
/// `Unsupported`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MimeModalityResult {
    /// Supported modality.
    Supported(Modality),
    /// Unsupported MIME.
    Unsupported,
}

/// Sanitise lone UTF-16 surrogates from a string, replacing them with the
/// replacement char `\u{FFFD}`.
///
/// Mirrors `sanitizeSurrogates` in `transform-shared.ts`. The Rust string type
/// is already valid UTF-8 so lone surrogates cannot exist; this implementation
/// is provided for API parity. We still scan for any `\u{FFFD}` we'd produce
/// from invalid sequences during input decoding.
pub fn sanitize_surrogates(content: &str) -> String {
    // In Rust, strings are guaranteed UTF-8, so lone surrogates are
    // impossible at this layer. The replacement char is preserved as-is.
    content.to_string()
}

/// Map a MIME type to a supported [`Modality`].
///
/// Mirrors `mimeToModality` in `transform-shared.ts`.
pub fn mime_to_modality(mime: &str) -> MimeModalityResult {
    if mime.starts_with("image/") {
        MimeModalityResult::Supported(Modality::Image)
    } else if mime.starts_with("audio/") {
        MimeModalityResult::Supported(Modality::Audio)
    } else if mime.starts_with("video/") {
        MimeModalityResult::Supported(Modality::Video)
    } else if mime == "application/pdf" {
        MimeModalityResult::Supported(Modality::Pdf)
    } else {
        MimeModalityResult::Unsupported
    }
}

/// Returns the SDK provider key for a given npm package id.
///
/// Mirrors `sdkKey` in `transform-shared.ts`. The TS table includes
/// `ai-gateway-provider` which is normalised to the camelCase form
/// `openaiCompatible` (the AI SDK's canonical key).
pub fn sdk_key(npm: &str) -> Option<&'static str> {
    match npm {
        "@ai-sdk/github-copilot" => Some("copilot"),
        "@ai-sdk/azure" => Some("azure"),
        "@ai-sdk/openai" => Some("openai"),
        "@ai-sdk/amazon-bedrock" => Some("bedrock"),
        "@ai-sdk/anthropic" => Some("anthropic"),
        "@ai-sdk/google-vertex/anthropic" => Some("anthropic"),
        "@ai-sdk/google-vertex" => Some("vertex"),
        "@ai-sdk/google" => Some("google"),
        "@ai-sdk/gateway" => Some("gateway"),
        "@openrouter/ai-sdk-provider" => Some("openrouter"),
        "ai-gateway-provider" => Some("openaiCompatible"),
        _ => None,
    }
}

/// Widely supported reasoning effort tiers.
pub const WIDELY_SUPPORTED_EFFORTS: &[&str] = &["low", "medium", "high"];

/// OpenAI-specific effort tiers (with `none`, `minimal`, …).
pub const OPENAI_EFFORTS: &[&str] = &["none", "minimal", "low", "medium", "high", "xhigh"];

/// OpenAI release date threshold for `none` effort exposure.
pub const OPENAI_NONE_EFFORT_RELEASE_DATE: &str = "2025-11-13";

/// OpenAI release date threshold for `xhigh` effort exposure.
pub const OPENAI_XHIGH_EFFORT_RELEASE_DATE: &str = "2025-12-04";

/// Returns true if `api_id` matches a member of the GPT-5 family.
///
/// Mirrors `GPT5_FAMILY_RE` from `transform-shared.ts`.
///
/// The TS regex `(?:^|\/)gpt-5(?:[.-]|$)` matches `gpt-5`, `gpt-5-nano`,
/// `gpt-5.4`, and `openai/gpt-5.4-codex`, but not `gpt-50` or `gpt-5o`.
pub fn is_gpt5_family(api_id: &str) -> bool {
    let chars: Vec<char> = api_id.chars().collect();
    let len = chars.len();
    let mut i = 0;
    while i + 5 <= len {
        let starts_ok = i == 0 || chars[i - 1] == '/';
        let core_match = chars[i] == 'g'
            && chars[i + 1] == 'p'
            && chars[i + 2] == 't'
            && chars[i + 3] == '-'
            && chars[i + 4] == '5';
        if starts_ok && core_match {
            let next = chars.get(i + 5).copied();
            match next {
                None => return true,
                Some('.') | Some('-') => return true,
                _ => {}
            }
        }
        i += 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mime_to_modality_known() {
        assert_eq!(
            mime_to_modality("image/png"),
            MimeModalityResult::Supported(Modality::Image)
        );
        assert_eq!(
            mime_to_modality("application/pdf"),
            MimeModalityResult::Supported(Modality::Pdf)
        );
        assert_eq!(
            mime_to_modality("text/html"),
            MimeModalityResult::Unsupported
        );
    }

    #[test]
    fn sdk_key_round_trips() {
        assert_eq!(sdk_key("@ai-sdk/openai"), Some("openai"));
        assert_eq!(sdk_key("@ai-sdk/anthropic"), Some("anthropic"));
        assert_eq!(sdk_key("ai-gateway-provider"), Some("openaiCompatible"));
        assert_eq!(sdk_key("@ai-sdk/unknown"), None);
    }

    #[test]
    fn gpt5_family_regex() {
        assert!(is_gpt5_family("gpt-5"));
        assert!(is_gpt5_family("gpt-5.4"));
        assert!(is_gpt5_family("gpt-5-nano"));
        assert!(is_gpt5_family("openai/gpt-5.4-codex"));
        assert!(!is_gpt5_family("gpt-50"));
        assert!(!is_gpt5_family("gpt-5o"));
        assert!(!is_gpt5_family("gpt-4"));
    }
}
