/// Map a single [`Action`] from the chat-bridge worker to an optional
/// `ChatEvent`. Reasoning, ticks, and routing-only actions are dropped; the
/// activity start/finish bookends are surfaced as informational notices so the
/// user can see retry messages without having to inspect logs.
fn translate_action(action: Action) -> Option<ChatEvent> {
    match action {
        Action::Runtime(RuntimeEvent::AssistantTextDelta { text }) => {
            Some(ChatEvent::AssistantDelta(text))
        }
        Action::Runtime(RuntimeEvent::AssistantCompleted) => Some(ChatEvent::TurnComplete),
        Action::Runtime(RuntimeEvent::AssistantFailed { error }) => {
            Some(ChatEvent::TurnFailed(error))
        }
        Action::Runtime(RuntimeEvent::ReasoningEnded { reasoning_id, text }) => {
            Some(ChatEvent::Reasoning { reasoning_id, text })
        }
        Action::Runtime(RuntimeEvent::Tool(event)) => Some(ChatEvent::Tool(event)),
        Action::Runtime(other @ RuntimeEvent::SessionStarted { .. })
        | Action::Runtime(other @ RuntimeEvent::SessionEnded { .. })
        | Action::Runtime(other @ RuntimeEvent::DaemonStatus { .. })
        | Action::Runtime(other @ RuntimeEvent::PermissionAsked { .. })
        | Action::Runtime(other @ RuntimeEvent::PermissionReplied { .. })
        | Action::Runtime(other @ RuntimeEvent::QuestionAsked { .. })
        | Action::Runtime(other @ RuntimeEvent::QuestionReplied { .. }) => {
            Some(ChatEvent::Runtime(other))
        }
        Action::ActivityUpdated {
            status: Some(status),
            ..
        } => Some(ChatEvent::Notice(NoticeKind::Warn, status)),
        Action::ActivityFinished {
            success: false,
            status: Some(status),
            ..
        } => Some(ChatEvent::Notice(NoticeKind::Error, status)),
        _ => None,
    }
}

/// Stateful counterpart used by the per-turn translator thread. Mirrors
/// [`translate_action`] for non-tool actions but accumulates tool stdout per
/// `tool_id` so that on `ToolEvent::Complete` we can attempt to parse the
/// buffer as a unified diff and emit one [`ChatEvent::Diff`] per parsed file
/// before forwarding the `Complete`.
///
/// Returning `Vec<ChatEvent>` lets a single inbound action fan out to multiple
/// outbound events (e.g. N diff cards + 1 tool complete).
fn translate_action_stateful(
    action: Action,
    tool_stdout: &mut HashMap<String, String>,
) -> Vec<ChatEvent> {
    match action {
        Action::Runtime(RuntimeEvent::Tool(event)) => translate_tool_event(event, tool_stdout),
        other => translate_action(other).into_iter().collect(),
    }
}

/// Intercept tool lifecycle events to buffer stdout and synthesize
/// [`ChatEvent::Diff`] cards on completion. Always forwards the original
/// `ToolEvent` so existing chip / tool-card rendering keeps working.
fn translate_tool_event(
    event: ToolEvent,
    tool_stdout: &mut HashMap<String, String>,
) -> Vec<ChatEvent> {
    match &event {
        ToolEvent::Start { id, .. } => {
            tool_stdout.insert(id.clone(), String::new());
            vec![ChatEvent::Tool(event)]
        }
        ToolEvent::StdoutChunk { id, chunk } => {
            match tool_stdout.entry(id.clone()) {
                std::collections::hash_map::Entry::Occupied(mut entry) => {
                    entry.get_mut().push_str(chunk);
                }
                std::collections::hash_map::Entry::Vacant(entry) => {
                    entry.insert(chunk.to_string());
                }
            }
            vec![ChatEvent::Tool(event)]
        }
        ToolEvent::StderrChunk { .. } => vec![ChatEvent::Tool(event)],
        ToolEvent::Complete { id } => {
            #[allow(clippy::manual_unwrap_or_default)]
            let buf = match tool_stdout.remove(id) {
                Some(value) => value,
                None => String::new(),
            };
            let mut out: Vec<ChatEvent> = diff_events_from_stdout(&buf);
            out.push(ChatEvent::Tool(event));
            out
        }
        ToolEvent::Fail { id, .. } => {
            tool_stdout.remove(id);
            vec![ChatEvent::Tool(event)]
        }
    }
}

/// Parse `stdout` as a unified diff and turn each parsed file into a
/// [`ChatEvent::Diff`] payload. Returns an empty vector when `stdout` is empty
/// or does not contain at least one `--- … / +++ …` header pair.
fn diff_events_from_stdout(stdout: &str) -> Vec<ChatEvent> {
    if !looks_like_unified_diff(stdout) {
        return Vec::new();
    }
    parse_unified_diff(stdout)
        .into_iter()
        .filter(|file| !file.hunks.is_empty())
        .map(diff_file_to_event)
        .collect()
}

/// Quick guard so we don't waste cycles running the parser on every tool
/// stdout buffer. Looks for the classic unified-diff header — any `--- …`
/// line that's followed by `+++ …` indicates a unified diff.
fn looks_like_unified_diff(stdout: &str) -> bool {
    let mut saw_minus = false;
    for line in stdout.lines() {
        let trimmed = line.trim_start();
        if saw_minus && trimmed.starts_with("+++ ") {
            return true;
        }
        saw_minus = trimmed.starts_with("--- ");
    }
    false
}

/// Convert a parser-emitted [`DiffFile`] into the owned [`ChatEvent::Diff`]
/// payload the inline runtime expects. Hunks are flattened (one body line per
/// row) and per-line numbering is reconstructed from the hunk header so the
/// runtime's gutter renderer has dense, correctly-aligned data.
fn diff_file_to_event(file: DiffFile) -> ChatEvent {
    let mut hunks: Vec<DiffBlockLine> = Vec::new();
    for hunk in &file.hunks {
        let mut previous_lineno = hunk.old_start as usize;
        let mut new_lineno = hunk.new_start as usize;
        for line in &hunk.lines {
            let (kind, previous, new) = match line.kind {
                ParserDiffLineKind::Add => {
                    let n = new_lineno;
                    new_lineno += 1;
                    (DiffLineKind::Added, None, Some(n))
                }
                ParserDiffLineKind::Del => {
                    let n = previous_lineno;
                    previous_lineno += 1;
                    (DiffLineKind::Removed, Some(n), None)
                }
                ParserDiffLineKind::Ctx => {
                    let o = previous_lineno;
                    let n = new_lineno;
                    previous_lineno += 1;
                    new_lineno += 1;
                    (DiffLineKind::Context, Some(o), Some(n))
                }
            };
            hunks.push(DiffBlockLine {
                kind,
                old_lineno: previous,
                new_lineno: new,
                text: line.text.clone(),
            });
        }
    }
    ChatEvent::Diff {
        path: file.filename,
        hunks,
    }
}
