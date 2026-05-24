// ── Mentions ─────────────────────────────────────────────────────────────────

#[derive(Default)]
struct MentionState {
    active: bool,
    trigger_byte_offset: usize,
    query: String,
    cursor: usize,
    filtered: Vec<PathBuf>,
}

impl MentionState {
    fn current_path(&self) -> Option<&PathBuf> {
        self.filtered.get(self.cursor)
    }
}

/// Find the byte offset of the active `@` trigger in `text`, if any.
///
/// The trigger is the last `@` that is either at the start of the text or
/// preceded by a non-alphanumeric character (so emails like `a@b.com` don't
/// fire). Everything after the `@` up to the cursor must be non-whitespace.
fn detect_mention_trigger(text: &str) -> Option<(usize, String)> {
    let at_pos = text.rfind('@')?;
    let prefix = &text[..at_pos];
    let preceded_by_alnum = prefix
        .chars()
        .next_back()
        .map(|c| c.is_alphanumeric() || c == '_')
        .unwrap_or(false);
    if preceded_by_alnum {
        return None;
    }
    let after = &text[at_pos + 1..];
    if after.chars().any(|c| c.is_whitespace()) {
        return None;
    }
    Some((at_pos, after.to_string()))
}
