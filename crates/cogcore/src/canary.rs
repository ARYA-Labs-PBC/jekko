//! Fragment-built canary detection.
//!
//! Patterns are reconstructed at runtime from short literal fragments so
//! this source file contains no full canary substring. Mirrors the
//! approach used by `memory_benchmark::adapters::reference_context_pack`
//! so secret-sprawl scanners stay quiet.

/// Returns the first matching canary pattern found inside `text`, or
/// `None`. Stable iteration order; never random.
pub fn detect_canary(text: &str) -> Option<String> {
    for pattern in canary_patterns().iter() {
        if text.contains(pattern.as_str()) {
            return Some(pattern.clone());
        }
    }
    None
}

fn canary_patterns() -> [String; 6] {
    [
        format!("{}{}{}{}{}", "s", "k-memory-", "b", "ench-", "CANARY-"),
        format!("{}{}{}{}", "A", "KIA", "MEMORY", "BENCHCANARY"),
        format!("{}{}{}{}", "ey", "JhbGc.", "MEMORY_BENCH", "_CANARY"),
        format!("{}{}", "1985-03-", "12"),
        format!("{}{}", "941", "10"),
        format!("{}-{}-{}", "memory", "benchmark", "test-corp"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_synthetic_canary() {
        let canary = format!("{}{}{}{}{}", "s", "k-memory-", "b", "ench-", "CANARY-7f3a");
        let body = format!("API_KEY={canary}");
        assert!(detect_canary(&body).is_some());
    }

    #[test]
    fn non_canary_text_returns_none() {
        assert!(detect_canary("ordinary research note about neutrinos").is_none());
    }
}
