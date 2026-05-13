//! Static safety checks for AutoResearch-generated patches.

const FORBIDDEN_TOKENS: &[&str] = &[
    "SystemTime::now",
    "Instant::now",
    "rand::",
    "thread_rng",
    "chrono::",
    "env::var(",
    "process::Command",
    "unsafe",
];

pub fn scan_patch(content: &str) -> Result<(), String> {
    for token in FORBIDDEN_TOKENS {
        if content.contains(token) {
            return Err(format!("forbidden token detected: {token}"));
        }
    }
    if !content.is_ascii() {
        return Err("patch content must remain ASCII".to_string());
    }
    Ok(())
}
