/// T-INLINE-CLUSTER #6: full transcript export. Writes every transcript block
/// (plus the in-flight assistant buffer, if any) to a timestamped file in cwd.
fn export_transcript_full(
    transcript: &Transcript,
    in_flight: Option<&InFlight>,
) -> (NoticeKind, String) {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let cwd = current_dir_or_dot();
    let out_path = cwd.join(format!("jekko-export-{secs}.txt"));
    let body_text = transcript.serialize();
    let mut body = String::new();
    body.push_str(&format!("jekko export · unix {secs}\n"));
    body.push_str(&format!("cwd: {}\n", cwd.display()));
    body.push_str(&format!("blocks: {}\n", transcript.block_count()));
    body.push_str("---\n");
    body.push_str(&body_text);
    if !body_text.ends_with('\n') {
        body.push('\n');
    }
    if let Some(state) = in_flight {
        if !state.buffer.is_empty() {
            body.push_str("\n[in-flight assistant buffer]\n");
            body.push_str(&state.buffer);
            body.push('\n');
        }
    }
    let line_count = body_text.lines().count();
    match std::fs::write(&out_path, body) {
        Ok(_) => (
            NoticeKind::Info,
            format!("exported {} line(s) to {}", line_count, out_path.display()),
        ),
        Err(err) => (NoticeKind::Error, format!("export failed: {err}")),
    }
}

/// T-INLINE-CLUSTER #4/#5: flatten the transcript (and the in-flight buffer
/// when present) into a plain-text payload suitable for OSC52 clipboard write.
/// Returns `(text, bytes-in-text)`.
fn transcript_as_text(transcript: &Transcript, in_flight: Option<&InFlight>) -> (String, usize) {
    let mut out = transcript.serialize();
    if let Some(state) = in_flight {
        if !state.buffer.is_empty() {
            if !out.is_empty() && !out.ends_with('\n') {
                out.push('\n');
            }
            out.push_str(&state.buffer);
            if !state.buffer.ends_with('\n') {
                out.push('\n');
            }
        }
    }
    let bytes = out.len();
    (out, bytes)
}

/// T-INLINE-CLUSTER #3: turn a mouse-drag selection into a plain-text payload
/// by walking the visible transcript rows within the rect. The cell-coord
/// `start` / `end` pair is normalised to top-left / bottom-right before
/// slicing each row.
///
/// We deliberately work off the *currently rendered* viewport (offset =
/// `scroll.offset_from_bottom`) — anything outside the viewport hasn't been
/// painted yet, so the user couldn't have selected it.
fn selection_text(
    transcript: &Transcript,
    rect: Rect,
    scroll_offset_from_bottom: usize,
    start: Option<(u16, u16)>,
    end: Option<(u16, u16)>,
) -> String {
    let (Some((sx, sy)), Some((ex, ey))) = (start, end) else {
        return String::new();
    };
    if rect.width == 0 || rect.height == 0 {
        return String::new();
    }
    let rows = transcript.visible_rows(rect.width, rect.height, scroll_offset_from_bottom);
    if rows.is_empty() {
        return String::new();
    }

    // Normalise (top-left, bottom-right) so the user can drag in any
    // direction and still get a sane payload.
    let (min_y, max_y) = (sy.min(ey), sy.max(ey));
    let (min_x, max_x) = (sx.min(ex), sx.max(ex));

    let mut out = String::new();
    // The renderer paints `rows` so the last row sits at `rect.bottom() - 1`;
    // the visible top row therefore lives at `rect.bottom() - rows.len()`.
    let start_y = rect
        .y
        .saturating_add(rect.height.saturating_sub(rows.len() as u16));
    for (i, line) in rows.iter().enumerate() {
        let row_y = start_y + i as u16;
        if row_y < min_y || row_y > max_y {
            continue;
        }
        let mut line_text = String::new();
        for span in &line.spans {
            line_text.push_str(span.content.as_ref());
        }
        let local_min_x = if row_y == min_y { min_x } else { rect.x };
        let local_max_x = if row_y == max_y {
            max_x
        } else {
            rect.x.saturating_add(rect.width.saturating_sub(1))
        };
        let lo = local_min_x.saturating_sub(rect.x) as usize;
        let hi = (local_max_x.saturating_sub(rect.x) as usize + 1).min(line_text.len());
        if lo < line_text.len() && lo < hi {
            // Walk char boundaries so we don't slice mid-grapheme.
            let mut start_idx = lo;
            while !line_text.is_char_boundary(start_idx) && start_idx < line_text.len() {
                start_idx += 1;
            }
            let mut end_idx = hi.min(line_text.len());
            while !line_text.is_char_boundary(end_idx) && end_idx > start_idx {
                end_idx -= 1;
            }
            out.push_str(&line_text[start_idx..end_idx]);
        }
        out.push('\n');
    }
    out
}

/// T-INLINE-CLUSTER #9: a single completed (user, assistant) turn captured so
/// `/compact` can preserve recent context while summarising older history.
#[derive(Clone, Debug)]
struct ChatTurnRecord {
    user: String,
    assistant: String,
}

/// T-INLINE-CLUSTER #9: trim `chat_history` to the most-recent 5 turns,
/// summarise the rest through the backend, and replace the transcript with a
/// divider + summary marker + replayed last-5 turns.
///
/// Returns `Some((rx, in_flight, prompt))` when a summary turn was kicked off
/// (caller must install it as the new in-flight). Returns `None` when there
/// is nothing to compact (<= 5 turns recorded).
const COMPACT_KEEP_LAST_N: usize = 5;

fn compact_history<B: ChatBackend>(
    chat_history: &mut Vec<ChatTurnRecord>,
    transcript: &mut Transcript,
    scroll: &mut ScrollState,
    terminal: &mut Tty,
    mode: RuntimeMode,
    _in_flight: Option<&InFlight>,
    backend: &mut B,
) -> Option<(tokio::sync::mpsc::Receiver<ChatEvent>, InFlight, String)> {
    if chat_history.len() <= COMPACT_KEEP_LAST_N {
        return None;
    }
    let split = chat_history.len() - COMPACT_KEEP_LAST_N;
    let older: Vec<ChatTurnRecord> = chat_history.drain(0..split).collect();
    let older_count = older.len();
    let mut prompt = String::from("Summarize the following conversation in 3 paragraphs:\n\n");
    for (idx, turn) in older.iter().enumerate() {
        prompt.push_str(&format!(
            "Turn {}\nUser: {}\nAssistant: {}\n\n",
            idx + 1,
            turn.user.trim(),
            turn.assistant.trim()
        ));
    }

    // Replace the visible transcript with a divider + replayed last-5 turns.
    transcript.clear();
    // T-GLYPH-WAVE2: divider chrome glyph honors GlyphMode. We keep the
    // ` · ` middle-dot separator Unicode (no clean ASCII fallback per spec).
    let divider = vec![Line::from(Span::styled(
        format!(
            "{} Compacted {older_count} turns · waiting for summary ",
            glyph_set::current().divider
        ),
        Style::default().fg(theme::codex_rule()),
    ))];
    let _ = emit_lines(
        terminal,
        mode,
        transcript,
        scroll,
        TranscriptEvent::divider(format!("compacted {older_count} turns"), divider),
    );
    for turn in chat_history.iter() {
        let user_lines = render_user(&turn.user);
        let _ = emit_lines(
            terminal,
            mode,
            transcript,
            scroll,
            TranscriptEvent::user(turn.user.clone(), user_lines),
        );
        if !turn.assistant.is_empty() {
            let asst_lines = render_assistant(&turn.assistant);
            let _ = emit_lines(
                terminal,
                mode,
                transcript,
                scroll,
                TranscriptEvent::assistant(turn.assistant.clone(), asst_lines),
            );
        }
    }

    // Kick off the summarising turn through the backend so the assistant
    // reply lands inline like any other turn.
    let state = InFlight::new();
    let rx = backend.start_turn(prompt.clone(), state.cancel_token());
    let bridged = bridge_chat_channel(rx);
    Some((bridged, state, prompt))
}
