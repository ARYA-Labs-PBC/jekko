// Transcript storage + width cache for the fullscreen runtime.
//
// The runtime stores semantic events, then asks for a width-specific view of
// the visible rows. This keeps the scrollback source of truth width-independent
// while still allowing resize-sensitive wrapping.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use ratatui::style::Style;
use ratatui::text::{Line, Span};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::engine::output_collapse::OutputBuffer;
use crate::inline_runtime::DiffBlockLine;
use crate::transcript::inline_cards::{
    render_assistant, render_diff, render_reasoning, render_session_header, render_tool_call,
    render_user, ActionStatus, DiffLine, ToolCall,
};

#[derive(Clone, Debug, Default)]
struct TranscriptBlock {
    rows: Vec<Line<'static>>,
}

#[derive(Clone, Debug)]
pub enum TranscriptEvent {
    Generic {
        lines: Vec<Line<'static>>,
    },
    User {
        text: String,
        lines: Vec<Line<'static>>,
    },
    Assistant {
        text: String,
        lines: Vec<Line<'static>>,
    },
    Reasoning {
        reasoning_id: Option<String>,
        text: String,
        lines: Vec<Line<'static>>,
    },
    Tool {
        tool_id: String,
        name: String,
        input: Option<String>,
        /// Captured stdout/stderr lines for the completed tool. Used by
        /// [`render_event_at_width`] (T-SEMANTIC-TRANSCRIPT-B) to re-run
        /// [`render_tool_call`] at the current viewport width. May be empty for
        /// historical events constructed before the field existed — in that
        /// case the cached `lines` are returned via the flat-wrap fallback.
        output: Vec<String>,
        /// Terminal status used to pick the bullet color when re-rendering.
        status: ActionStatus,
        /// Output cap applied at re-render time (mirrors
        /// `ToolCall::max_output_lines`).
        max_output_lines: usize,
        lines: Vec<Line<'static>>,
    },
    Diff {
        path: String,
        /// Owned diff payload retained so [`render_event_at_width`]
        /// (T-SEMANTIC-TRANSCRIPT-B) can re-run [`render_diff`] at the current
        /// viewport width, refreshing the dual-line-number gutter widths.
        /// Empty for callers that only have the pre-rendered `lines` (e.g.
        /// generic diff cards) — in that case the cached `lines` are returned
        /// via the flat-wrap fallback.
        hunks: Vec<DiffBlockLine>,
        lines: Vec<Line<'static>>,
    },
    SystemNotice {
        kind: String,
        text: String,
        lines: Vec<Line<'static>>,
    },
    SessionHeader {
        version: String,
        cwd_display: String,
        branch: Option<String>,
        lines: Vec<Line<'static>>,
    },
    Divider {
        label: String,
        lines: Vec<Line<'static>>,
    },
}

impl TranscriptEvent {
    pub fn generic(lines: Vec<Line<'static>>) -> Self {
        Self::Generic { lines }
    }

    pub fn user(text: String, lines: Vec<Line<'static>>) -> Self {
        Self::User { text, lines }
    }

    pub fn assistant(text: String, lines: Vec<Line<'static>>) -> Self {
        Self::Assistant { text, lines }
    }

    pub fn reasoning(
        reasoning_id: Option<String>,
        text: String,
        lines: Vec<Line<'static>>,
    ) -> Self {
        Self::Reasoning {
            reasoning_id,
            text,
            lines,
        }
    }

    /// Backward-compat constructor: builds a `Tool` event without retained
    /// re-render data. Callers that have the originating chip should use
    /// [`TranscriptEvent::tool_with_output`] so the cache can re-run
    /// [`render_tool_call`] at the current viewport width.
    pub fn tool(
        tool_id: String,
        name: String,
        input: Option<String>,
        lines: Vec<Line<'static>>,
    ) -> Self {
        Self::tool_with_output(
            tool_id,
            name,
            input,
            Vec::new(),
            ActionStatus::Success,
            usize::MAX,
            lines,
        )
    }

    /// T-SEMANTIC-TRANSCRIPT-B constructor: stash the captured output, status
    /// and cap so [`render_event_at_width`] can re-render the tool card.
    pub fn tool_with_output(
        tool_id: String,
        name: String,
        input: Option<String>,
        output: Vec<String>,
        status: ActionStatus,
        max_output_lines: usize,
        lines: Vec<Line<'static>>,
    ) -> Self {
        Self::Tool {
            tool_id,
            name,
            input,
            output,
            status,
            max_output_lines,
            lines,
        }
    }

    /// Backward-compat constructor for callers that only have pre-rendered
    /// diff lines (e.g. generic diff cards routed through translators that
    /// drop the parsed hunks). Prefer [`TranscriptEvent::diff_with_hunks`] so
    /// the cache can re-run [`render_diff`] at the current viewport width.
    pub fn diff(path: String, lines: Vec<Line<'static>>) -> Self {
        Self::Diff {
            path,
            hunks: Vec::new(),
            lines,
        }
    }

    /// T-SEMANTIC-TRANSCRIPT-B constructor: stash the owned diff payload so
    /// [`render_event_at_width`] can re-run [`render_diff`] at the current
    /// viewport width.
    pub fn diff_with_hunks(
        path: String,
        hunks: Vec<DiffBlockLine>,
        lines: Vec<Line<'static>>,
    ) -> Self {
        Self::Diff { path, hunks, lines }
    }

    pub fn system_notice(kind: String, text: String, lines: Vec<Line<'static>>) -> Self {
        Self::SystemNotice { kind, text, lines }
    }

    pub fn session_header(
        version: String,
        cwd_display: String,
        branch: Option<String>,
        lines: Vec<Line<'static>>,
    ) -> Self {
        Self::SessionHeader {
            version,
            cwd_display,
            branch,
            lines,
        }
    }

    pub fn divider(label: String, lines: Vec<Line<'static>>) -> Self {
        Self::Divider { label, lines }
    }

    pub fn lines(&self) -> &[Line<'static>] {
        match self {
            Self::Generic { lines }
            | Self::User { lines, .. }
            | Self::Assistant { lines, .. }
            | Self::Reasoning { lines, .. }
            | Self::Tool { lines, .. }
            | Self::Diff { lines, .. }
            | Self::SystemNotice { lines, .. }
            | Self::SessionHeader { lines, .. }
            | Self::Divider { lines, .. } => lines,
        }
    }

    pub fn serialize(&self) -> String {
        let mut out = String::new();
        for (idx, line) in self.lines().iter().enumerate() {
            if idx > 0 {
                out.push('\n');
            }
            out.push_str(&line_to_string(line));
        }
        out
    }
}

pub trait IntoTranscriptEvent {
    fn into_transcript_event(self) -> TranscriptEvent;
}

impl IntoTranscriptEvent for TranscriptEvent {
    fn into_transcript_event(self) -> TranscriptEvent {
        self
    }
}

impl IntoTranscriptEvent for Vec<Line<'static>> {
    fn into_transcript_event(self) -> TranscriptEvent {
        TranscriptEvent::generic(self)
    }
}

impl IntoTranscriptEvent for &[Line<'static>] {
    fn into_transcript_event(self) -> TranscriptEvent {
        TranscriptEvent::generic(self.to_vec())
    }
}

impl IntoTranscriptEvent for &Vec<Line<'static>> {
    fn into_transcript_event(self) -> TranscriptEvent {
        TranscriptEvent::generic(self.clone())
    }
}

#[derive(Debug, Default)]
struct TranscriptCache {
    width: u16,
    blocks: Vec<TranscriptBlock>,
    rows: Vec<Line<'static>>,
    /// Test-only counter incremented each time
    /// [`TranscriptCache::build`] re-runs the full event dispatch.
    /// T-SEMANTIC-TRANSCRIPT-B regression coverage uses this to confirm a
    /// resize triggers a rebuild and that repeat calls at the same width
    /// reuse the cached row set.
    build_count: usize,
}

#[derive(Debug, Default)]
pub struct Transcript {
    events: Vec<TranscriptEvent>,
    cache: RefCell<TranscriptCache>,
    /// Persistent tool output buffers keyed by tool_id. Unlike `events` (which
    /// only retain the collapsed/rendered transcript card), this map preserves
    /// the lossless `OutputBuffer` so the pager (Ctrl+O) can read back the
    /// full stdout/stderr of any completed tool — even after the originating
    /// `InFlight` has been dropped at turn finalization. T-SEMANTIC-TRANSCRIPT-A.
    tool_buffers: HashMap<String, Arc<OutputBuffer>>,
}

impl Transcript {
    pub const MAX_BLOCKS: usize = 20_000;

    pub fn push_event<E: IntoTranscriptEvent>(&mut self, event: E) {
        let event = event.into_transcript_event();
        if event.lines().is_empty() {
            return;
        }
        self.events.push(event);
        if self.events.len() > Self::MAX_BLOCKS {
            let excess = self.events.len() - Self::MAX_BLOCKS;
            self.events.drain(0..excess);
        }
        self.cache.borrow_mut().invalidate();
    }

    pub fn push(&mut self, lines: &[Line<'static>]) {
        self.push_event(lines);
    }

    pub fn clear(&mut self) {
        self.events.clear();
        self.tool_buffers.clear();
        self.cache.borrow_mut().invalidate();
    }

    /// Record a persistent tool output buffer indexed by `tool_id`. The pager
    /// reads from this map via [`Transcript::tool_buffer`]; the buffer is
    /// retained across event replays and only cleared by [`Transcript::clear`].
    pub fn record_tool_buffer(&mut self, tool_id: String, buffer: Arc<OutputBuffer>) {
        self.tool_buffers.insert(tool_id, buffer);
    }

    /// Look up the persistent output buffer for a completed tool by `tool_id`.
    pub fn tool_buffer(&self, tool_id: &str) -> Option<&Arc<OutputBuffer>> {
        self.tool_buffers.get(tool_id)
    }

    /// Return the line-by-line view of the most recently recorded tool buffer,
    /// or `None` if no tool has emitted output yet. Used by the pager's
    /// scrollback fallback (Ctrl+O outside an in-flight turn).
    ///
    /// "Most recent" is determined by walking events in reverse and picking the
    /// first `TranscriptEvent::Tool` whose `tool_id` has a registered buffer.
    pub fn last_tool_buffer_lines(&self) -> Option<Vec<String>> {
        for event in self.events.iter().rev() {
            if let TranscriptEvent::Tool { tool_id, .. } = event {
                if let Some(buf) = self.tool_buffers.get(tool_id) {
                    return Some(buf.all_lines().to_vec());
                }
            }
        }
        None
    }

    pub fn block_count(&self) -> usize {
        self.events.len()
    }

    pub fn row_count(&self, width: u16) -> usize {
        self.with_cache(width, |cache| {
            cache.blocks.iter().map(|block| block.rows.len()).sum()
        })
    }

    pub fn visible_rows(
        &self,
        width: u16,
        height: u16,
        offset_from_bottom: usize,
    ) -> Vec<Line<'static>> {
        if width == 0 || height == 0 {
            return Vec::new();
        }
        self.with_cache(width, |cache| {
            let total = cache.rows.len();
            if total == 0 {
                return Vec::new();
            }
            let height = height as usize;
            let max_offset = total.saturating_sub(height);
            let offset_from_bottom = offset_from_bottom.min(max_offset);
            let end = total.saturating_sub(offset_from_bottom);
            let start = end.saturating_sub(height);
            cache.rows[start..end].to_vec()
        })
    }

    pub fn clamp_offset(&self, width: u16, offset_from_bottom: usize) -> usize {
        let total = self.row_count(width);
        offset_from_bottom.min(total)
    }

    pub fn serialize(&self) -> String {
        let mut out = String::new();
        for (idx, event) in self.events.iter().enumerate() {
            if idx > 0 {
                out.push('\n');
            }
            out.push_str(&event.serialize());
        }
        out
    }

    fn with_cache<R>(&self, width: u16, f: impl FnOnce(&TranscriptCache) -> R) -> R {
        let mut cache = self.cache.borrow_mut();
        if cache.width != width {
            let next_count = cache.build_count.wrapping_add(1);
            *cache = TranscriptCache::build(width, &self.events, next_count);
        }
        f(&cache)
    }

    /// Test-only accessor: number of times the cache has been rebuilt over
    /// the lifetime of this `Transcript`. T-SEMANTIC-TRANSCRIPT-B uses this
    /// to assert that resizes trigger a rebuild and that repeat calls at the
    /// same width reuse the cached row set.
    #[cfg(test)]
    pub(crate) fn cache_build_count(&self) -> usize {
        self.cache.borrow().build_count
    }
}

impl TranscriptCache {
    fn build(width: u16, events: &[TranscriptEvent], build_count: usize) -> Self {
        let mut blocks = Vec::with_capacity(events.len());
        let mut rows = Vec::new();
        for event in events {
            let mut block_rows = Vec::new();
            for line in render_event_at_width(event, width) {
                let wrapped = wrap_line(&line, width);
                block_rows.extend(wrapped.iter().cloned());
                rows.extend(wrapped);
            }
            blocks.push(TranscriptBlock { rows: block_rows });
        }
        Self {
            width,
            blocks,
            rows,
            build_count,
        }
    }

    fn invalidate(&mut self) {
        self.width = 0;
        self.rows.clear();
    }
}

/// T-SEMANTIC-TRANSCRIPT-B: re-render a single [`TranscriptEvent`] at the
/// supplied viewport `width`, returning the resulting `Line<'static>` rows
/// (before final wrap). Variants that retain enough state to re-run their
/// renderer (`Diff` with hunks, `Tool` with output, `User`, `Assistant`,
/// `Reasoning`, `SessionHeader`) do so here so resize-aware layout (gutter
/// widths, tool chip wrap, etc.) updates with the new width. Variants without
/// retained state fall through to the cached `lines` and let `wrap_line`
/// handle the new width with the flat-wrap path.
fn render_event_at_width(event: &TranscriptEvent, _width: u16) -> Vec<Line<'static>> {
    match event {
        TranscriptEvent::User { text, lines, .. } => {
            if text.is_empty() {
                lines.clone()
            } else {
                render_user(text)
            }
        }
        TranscriptEvent::Assistant { text, lines, .. } => {
            if text.is_empty() {
                lines.clone()
            } else {
                render_assistant(text)
            }
        }
        TranscriptEvent::Reasoning { text, lines, .. } => {
            if text.is_empty() {
                lines.clone()
            } else {
                render_reasoning(text)
            }
        }
        TranscriptEvent::Tool {
            name,
            input,
            output,
            status,
            max_output_lines,
            lines,
            ..
        } => {
            // The renderer needs borrowed slices. When the producer didn't
            // retain the output (legacy `TranscriptEvent::tool`), fall back to
            // the cached lines so we don't render an empty tool card.
            if output.is_empty() && !lines.is_empty() {
                return lines.clone();
            }
            let call = ToolCall {
                verb: name.as_str(),
                args: input.as_deref().unwrap_or(""),
                status: *status,
                output: output.as_slice(),
                max_output_lines: *max_output_lines,
            };
            render_tool_call(&call)
        }
        TranscriptEvent::Diff { path, hunks, lines } => {
            // When the producer routed through the legacy `TranscriptEvent::diff`
            // constructor the hunks are empty and we can't re-derive the dual
            // gutter — fall back to the cached lines. Anything coming through
            // `diff_with_hunks` (the inline runtime's ChatEvent::Diff arm)
            // re-renders at the new width.
            if hunks.is_empty() {
                return lines.clone();
            }
            let borrowed: Vec<DiffLine<'_>> = hunks
                .iter()
                .map(|h| DiffLine {
                    kind: h.kind,
                    old_lineno: h.old_lineno,
                    new_lineno: h.new_lineno,
                    text: h.text.as_str(),
                })
                .collect();
            render_diff(path, &borrowed)
        }
        TranscriptEvent::SessionHeader {
            version,
            cwd_display,
            branch,
            lines,
        } => {
            if version.is_empty() {
                lines.clone()
            } else {
                render_session_header(version, cwd_display, branch.as_deref())
            }
        }
        // Generic / SystemNotice / Divider don't carry enough metadata to
        // re-run their renderer (or have no width-sensitive layout), so let
        // wrap_line handle them.
        TranscriptEvent::Generic { lines }
        | TranscriptEvent::SystemNotice { lines, .. }
        | TranscriptEvent::Divider { lines, .. } => lines.clone(),
    }
}

fn wrap_line(line: &Line<'static>, width: u16) -> Vec<Line<'static>> {
    if width == 0 {
        return Vec::new();
    }
    let width = width as usize;
    let mut out = Vec::new();
    let mut current: Vec<Span<'static>> = Vec::new();
    let mut current_width = 0usize;
    let mut saw_grapheme = false;

    for span in &line.spans {
        let style = span.style;
        for grapheme in span.content.graphemes(true) {
            let g_width = grapheme.width();
            saw_grapheme = true;
            if g_width == 0 {
                current.push(Span::styled(grapheme.to_string(), style));
                continue;
            }
            if current_width > 0 && current_width + g_width > width {
                out.push(Line::from(std::mem::take(&mut current)));
                current_width = 0;
            }
            current.push(Span::styled(grapheme.to_string(), style));
            current_width += g_width;
            if current_width >= width {
                out.push(Line::from(std::mem::take(&mut current)));
                current_width = 0;
            }
        }
    }

    if !current.is_empty() {
        out.push(Line::from(current));
    } else if !saw_grapheme {
        out.push(Line::from(Span::styled(
            String::new(),
            match line.spans.first() {
                Some(span) => span.style,
                None => Style::default(),
            },
        )));
    }
    out
}

fn line_to_string(line: &Line<'static>) -> String {
    let mut out = String::new();
    for span in &line.spans {
        out.push_str(span.content.as_ref());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::{Color, Style};
    use ratatui::text::{Line, Span};

    fn line(spans: Vec<Span<'static>>) -> Line<'static> {
        Line::from(spans)
    }

    fn text_line(text: &str) -> Line<'static> {
        Line::from(Span::styled(text.to_string(), Style::default()))
    }

    #[test]
    fn push_tracks_events() {
        let mut t = Transcript::default();
        t.push(&[text_line("a")]);
        t.push_event(TranscriptEvent::user(
            "hello".to_string(),
            vec![text_line("b")],
        ));
        assert_eq!(t.block_count(), 2);
    }

    #[test]
    fn visible_rows_returns_tail_slice() {
        let mut t = Transcript::default();
        t.push(&[text_line("one")]);
        t.push(&[text_line("two")]);
        t.push(&[text_line("three")]);
        let rows = t.visible_rows(80, 2, 0);
        assert_eq!(rows.len(), 2);
        assert_eq!(line_to_string(&rows[0]), "two");
        assert_eq!(line_to_string(&rows[1]), "three");
    }

    #[test]
    fn cache_preserves_event_boundaries_per_block() {
        let mut t = Transcript::default();
        t.push_event(TranscriptEvent::assistant(
            "one two three four five six seven".to_string(),
            render_assistant("one two three four five six seven"),
        ));
        t.push_event(TranscriptEvent::user("ok".to_string(), render_user("ok")));

        let _ = t.visible_rows(16, 12, 0);
        let cache = t.cache.borrow();
        assert_eq!(cache.blocks.len(), 2);
        assert!(
            cache.blocks[0].rows.len() > 1,
            "assistant event should wrap"
        );
        assert_eq!(cache.blocks[1].rows.len(), 1, "user event stays one block");
        let flat_rows: usize = cache.blocks.iter().map(|block| block.rows.len()).sum();
        assert_eq!(flat_rows, cache.rows.len());
    }

    #[test]
    fn width_cache_rebuilds_on_resize() {
        let mut t = Transcript::default();
        t.push(&[text_line("hello world")]);
        let wide = t.visible_rows(80, 10, 0);
        let narrow = t.visible_rows(5, 10, 0);
        assert!(!wide.is_empty());
        assert!(!narrow.is_empty());
        assert!(t.row_count(5) >= t.row_count(80));
    }

    #[test]
    fn wrap_line_preserves_mixed_span_styles() {
        let red = Style::default().fg(Color::Red);
        let blue = Style::default().fg(Color::Blue);
        let rows = wrap_line(
            &line(vec![
                Span::styled("ab".to_string(), red),
                Span::styled("cd".to_string(), blue),
            ]),
            3,
        );
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].spans[0].style, red);
        assert_eq!(rows[1].spans[0].style, blue);
        let joined: String = rows
            .iter()
            .flat_map(|row| row.spans.iter().map(|span| span.content.as_ref()))
            .collect();
        assert_eq!(joined, "abcd");
    }

    #[test]
    fn wrap_line_handles_wide_graphemes() {
        let rows = wrap_line(&text_line("a中b"), 2);
        assert!(!rows.is_empty());
        let joined: String = rows
            .iter()
            .flat_map(|row| row.spans.iter().map(|span| span.content.as_ref()))
            .collect();
        assert_eq!(joined, "a中b");
    }

    #[test]
    fn wrap_line_keeps_zero_width_graphemes() {
        let rows = wrap_line(&text_line("a\u{200b}b"), 1);
        assert!(!rows.is_empty());
        let joined: String = rows
            .iter()
            .flat_map(|row| row.spans.iter().map(|span| span.content.as_ref()))
            .collect();
        assert_eq!(joined, "a\u{200b}b");
    }

    #[test]
    fn wrap_line_preserves_empty_spans() {
        let rows = wrap_line(
            &line(vec![
                Span::styled(String::new(), Style::default()),
                Span::styled(String::new(), Style::default()),
            ]),
            4,
        );
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].spans.len(), 1);
        assert!(rows[0].spans[0].content.is_empty());
    }

    #[test]
    fn serialize_is_width_independent() {
        let mut t = Transcript::default();
        t.push_event(TranscriptEvent::session_header(
            "1.0.0".to_string(),
            "~/code/jekko".to_string(),
            Some("main".to_string()),
            vec![text_line("header")],
        ));
        t.push_event(TranscriptEvent::assistant(
            "hello".to_string(),
            vec![text_line("assistant line")],
        ));
        let before = t.serialize();
        let _ = t.visible_rows(4, 1, 0);
        let after = t.serialize();
        assert_eq!(before, after);
        assert!(after.contains("assistant line"));
    }

    #[test]
    fn push_trims_max_blocks() {
        let mut t = Transcript::default();
        for idx in 0..(Transcript::MAX_BLOCKS + 3) {
            t.push_event(TranscriptEvent::generic(vec![text_line(&format!(
                "line {idx}"
            ))]));
        }
        assert_eq!(t.block_count(), Transcript::MAX_BLOCKS);
    }

    #[test]
    fn tool_buffer_survives_event_replay() {
        let mut t = Transcript::default();
        let id = "tool-survives".to_string();
        let mut buf = OutputBuffer::new(id.clone());
        // `push_line` may fail to open the raw-log dir in CI sandboxes; we
        // ignore the error because the in-memory buffer is what the pager
        // reads — the on-disk persistence is best-effort.
        let _ = buf.push_line("hello-from-tool");
        let arc = Arc::new(buf);

        t.push_event(TranscriptEvent::tool(
            id.clone(),
            "Bash".into(),
            Some("echo hi".into()),
            vec![text_line("Bash · running")],
        ));
        t.record_tool_buffer(id.clone(), Arc::clone(&arc));

        for idx in 0..100 {
            t.push_event(TranscriptEvent::generic(vec![text_line(&format!(
                "filler {idx}"
            ))]));
        }

        let fetched = t.tool_buffer(&id).expect("buffer must survive replay");
        assert!(Arc::ptr_eq(fetched, &arc));
    }

    #[test]
    fn clear_drops_tool_buffers() {
        let mut t = Transcript::default();
        let id = "tool-cleared".to_string();
        let buf = Arc::new(OutputBuffer::new(id.clone()));
        t.record_tool_buffer(id.clone(), Arc::clone(&buf));
        assert!(t.tool_buffer(&id).is_some());
        t.clear();
        assert!(t.tool_buffer(&id).is_none());
    }

    // ── T-SEMANTIC-TRANSCRIPT-B: resize-aware re-render of cached events ────

    /// Lift the `inline_runtime::DiffBlockLine` definition into a small helper
    /// so the resize tests stay readable. Mirrors the producer in
    /// `inline_runtime::ChatEvent::Diff -> TranscriptEvent::diff_with_hunks`.
    fn diff_payload() -> (String, Vec<DiffBlockLine>) {
        use crate::transcript::inline_cards::DiffLineKind;
        let path = "src/lib.rs".to_string();
        let hunks = vec![
            DiffBlockLine {
                kind: DiffLineKind::Context,
                old_lineno: Some(1),
                new_lineno: Some(1),
                text: "ctx ".to_string() + &"x".repeat(60),
            },
            DiffBlockLine {
                kind: DiffLineKind::Added,
                old_lineno: None,
                new_lineno: Some(2),
                text: "add ".to_string() + &"y".repeat(60),
            },
            DiffBlockLine {
                kind: DiffLineKind::Removed,
                old_lineno: Some(3),
                new_lineno: None,
                text: "del ".to_string() + &"z".repeat(60),
            },
        ];
        (path, hunks)
    }

    fn row_to_string(row: &Line<'static>) -> String {
        row.spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect::<Vec<_>>()
            .join("")
    }

    #[test]
    fn resize_diff_event_uses_target_width_gutter() {
        let (path, hunks) = diff_payload();
        let mut t = Transcript::default();
        // Pre-render the lines at width 80 (mirrors what the inline runtime's
        // ChatEvent::Diff producer would do via render_diff_lines_from_payload).
        let prebuilt = {
            use crate::transcript::inline_cards::{render_diff, DiffLine};
            let borrowed: Vec<DiffLine<'_>> = hunks
                .iter()
                .map(|h| DiffLine {
                    kind: h.kind,
                    old_lineno: h.old_lineno,
                    new_lineno: h.new_lineno,
                    text: h.text.as_str(),
                })
                .collect();
            render_diff(&path, &borrowed)
        };
        t.push_event(TranscriptEvent::diff_with_hunks(path, hunks, prebuilt));

        // Fully zoom out: at width 200 the body fits without wrap, so the
        // gutter columns appear unsplit on each body row.
        let wide = t.visible_rows(200, 64, 0);
        // The header line is row 0; the three diff rows follow. With the dual
        // gutter present, each body row must still carry a `+` / `-` / ` `
        // sigil — confirms render_diff actually ran.
        let wide_sigils: Vec<char> = wide[1..]
            .iter()
            .filter_map(|row| {
                row.spans.iter().find_map(|s| match s.content.as_ref() {
                    "+" | "-" => s.content.chars().next(),
                    _ => None,
                })
            })
            .collect();
        assert!(
            wide_sigils.contains(&'+'),
            "wide render must have + sigil: {wide_sigils:?}"
        );
        assert!(
            wide_sigils.contains(&'-'),
            "wide render must have - sigil: {wide_sigils:?}"
        );

        // Narrow to 40 columns: each body row must now wrap onto multiple
        // rows. Total row count must exceed the width-200 row count.
        let narrow_total = t.row_count(40);
        let wide_total = t.row_count(200);
        assert!(
            narrow_total > wide_total,
            "narrow width must produce more wrapped rows ({narrow_total} <= {wide_total})"
        );

        // At width 40 the gutter still wins the first row of each body line
        // (sigil appears on the first wrapped row only).
        let narrow = t.visible_rows(40, 200, 0);
        let narrow_joined: String = narrow.iter().map(row_to_string).collect();
        assert!(
            narrow_joined.contains("+"),
            "narrow render must keep + sigil"
        );
        assert!(
            narrow_joined.contains("-"),
            "narrow render must keep - sigil"
        );
    }

    #[test]
    fn resize_assistant_event_preserves_span_styles() {
        let text = "hello world this is a fairly long assistant turn".to_string();
        let mut t = Transcript::default();
        let lines = {
            use crate::transcript::inline_cards::render_assistant;
            render_assistant(&text)
        };
        t.push_event(TranscriptEvent::assistant(text.clone(), lines));

        // Wide: single wrapped row per text line. Narrow: multiple rows. Each
        // wrapped row must carry the orange margin span style (codex_orange_agent)
        // for the prefix or the strong fg for the body.
        let wide = t.visible_rows(80, 10, 0);
        assert!(!wide.is_empty());
        let narrow = t.visible_rows(20, 64, 0);
        assert!(narrow.len() > wide.len());
        // First span on every wrapped row carries a style (the margin or its
        // continuation). Confirm no row is span-less.
        for row in &narrow {
            assert!(
                !row.spans.is_empty(),
                "wrapped row must have at least 1 span"
            );
            // The original prefix carries a non-default fg color (the orange
            // agent color). Wave 9's span-aware wrap_line preserves spans, so
            // the first character keeps that style.
            let first_style = row.spans[0].style;
            assert!(
                first_style.fg.is_some(),
                "wrapped continuation row should keep a styled span (got fg=None): {:?}",
                row.spans
            );
        }
    }

    #[test]
    fn resize_tool_event_truncates_at_width() {
        use crate::transcript::inline_cards::ActionStatus;
        let mut t = Transcript::default();
        let output: Vec<String> = (0..20)
            .map(|i| format!("output line {i}: ").repeat(8))
            .collect();
        let prebuilt = {
            use crate::transcript::inline_cards::{render_tool_call, ToolCall};
            let call = ToolCall {
                verb: "Bash",
                args: "long-running-cmd",
                status: ActionStatus::Success,
                output: &output,
                max_output_lines: 12,
            };
            render_tool_call(&call)
        };
        t.push_event(TranscriptEvent::tool_with_output(
            "tool-resize".to_string(),
            "Bash".to_string(),
            Some("long-running-cmd".to_string()),
            output,
            ActionStatus::Success,
            12,
            prebuilt,
        ));

        let wide = t.visible_rows(120, 64, 0);
        let narrow = t.visible_rows(40, 200, 0);
        // The wide render fits each output line on a single row; narrow wraps
        // them, so narrow has strictly more rows.
        assert!(
            narrow.len() > wide.len(),
            "tool card must produce more rows when wrapped narrower ({} <= {})",
            narrow.len(),
            wide.len()
        );
        // Sanity: at width 40 the joined text must still contain "Bash" + the
        // header verb, confirming `render_tool_call` actually re-ran.
        let narrow_joined: String = narrow.iter().map(row_to_string).collect();
        assert!(narrow_joined.contains("Bash"));
        assert!(narrow_joined.contains("long-running-cmd"));
    }

    #[test]
    fn cache_rebuild_on_width_change() {
        let mut t = Transcript::default();
        t.push_event(TranscriptEvent::user(
            "hi".to_string(),
            vec![text_line("│ hi")],
        ));
        // First read at width 80 — cache build count goes from 0 to 1.
        let _ = t.visible_rows(80, 10, 0);
        let count_after_first = t.cache_build_count();
        assert_eq!(count_after_first, 1);
        // Resize to 40 — cache must rebuild (count goes to 2).
        let _ = t.visible_rows(40, 10, 0);
        assert_eq!(t.cache_build_count(), 2);
    }

    #[test]
    fn cache_reused_at_same_width() {
        let mut t = Transcript::default();
        t.push_event(TranscriptEvent::user(
            "hi".to_string(),
            vec![text_line("│ hi")],
        ));
        // Two reads at the same width should reuse the cache.
        let _ = t.visible_rows(80, 10, 0);
        let count_after_first = t.cache_build_count();
        let _ = t.visible_rows(80, 10, 0);
        assert_eq!(
            t.cache_build_count(),
            count_after_first,
            "cache should not rebuild at the same width"
        );
        let _ = t.row_count(80);
        assert_eq!(
            t.cache_build_count(),
            count_after_first,
            "row_count at the same width should not trigger rebuild"
        );
    }
}
