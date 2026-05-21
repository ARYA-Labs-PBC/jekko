enum LoopEvent {
    Tick,
    Chat(Option<ChatEvent>),
    Input(Option<std::result::Result<Event, std::io::Error>>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RuntimeMode {
    Fullscreen,
    NoAltScreen,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FocusArea {
    Composer,
    Agents,
}

#[derive(Default)]
struct ScrollState {
    offset_from_bottom: usize,
}

impl ScrollState {
    fn sticky_bottom(&self) -> bool {
        self.offset_from_bottom == 0
    }

    /// T-INLINE-CLUSTER #3: scroll N lines up (used by mouse wheel ScrollUp).
    fn line_up_by(&mut self, n: u16) {
        self.offset_from_bottom = self.offset_from_bottom.saturating_add(n as usize);
    }

    /// T-INLINE-CLUSTER #3: scroll N lines down (used by mouse wheel ScrollDown).
    fn line_down_by(&mut self, n: u16) {
        self.offset_from_bottom = self.offset_from_bottom.saturating_sub(n as usize);
    }

    fn page_up(&mut self, terminal_height: u16) {
        let page = terminal_height.saturating_sub(5).max(1) as usize;
        self.offset_from_bottom = self.offset_from_bottom.saturating_add(page);
    }

    fn page_down(&mut self, terminal_height: u16) {
        let page = terminal_height.saturating_sub(5).max(1) as usize;
        self.offset_from_bottom = self.offset_from_bottom.saturating_sub(page);
    }

    fn scroll_to_top(&mut self, transcript: &Transcript, width: u16) {
        self.offset_from_bottom = transcript.row_count(width);
    }

    fn scroll_to_bottom(&mut self) {
        self.offset_from_bottom = 0;
    }

    fn clamp(&mut self, transcript: &Transcript, width: u16) {
        self.offset_from_bottom = self.offset_from_bottom.min(transcript.row_count(width));
    }
}

#[derive(Default)]
struct ComposerState {
    text: String,
    slash: SlashState,
    mention: MentionState,
    paste: PasteBuffer,
}

impl ComposerState {
    fn clear_all(&mut self) {
        self.text.clear();
        self.slash = SlashState::default();
        self.mention = MentionState::default();
        self.paste.clear();
    }

    fn insert_paste(&mut self, text: String, index: &FileIndex, catalog: &SlashCatalog) {
        if text.is_empty() {
            return;
        }
        if PasteBuffer::should_collapse(&text) {
            let record = self.paste.stash(text);
            self.text.push_str(&record.summary());
        } else {
            self.text.push_str(&text);
        }
        self.sync_popups(index, catalog);
    }

    fn take_expanded_text(&mut self) -> String {
        let visible = std::mem::take(&mut self.text);
        let expanded = self.paste.expand(&visible);
        self.clear_all();
        expanded
    }

    /// Recompute slash-popup activation from the current `text`. Called after
    /// every keystroke that could change `text`.
    fn sync_slash(&mut self, catalog: &SlashCatalog) {
        if let Some(rest) = self.text.strip_prefix('/') {
            // Only activate when the prefix is contiguous (no whitespace yet).
            if rest.chars().all(|c| !c.is_whitespace()) {
                if !self.slash.active {
                    self.slash.active = true;
                    self.slash.cursor = 0;
                }
                let next_query = rest.to_string();
                if self.slash.query != next_query {
                    self.slash.submenu = None;
                }
                self.slash.query = next_query;
                self.slash.refresh_filter(catalog);
                return;
            }
        }
        self.slash = SlashState::default();
    }

    /// Recompute mention-popup activation against `index`. Slash wins if both
    /// would trigger — the chrome renders only one popup at a time.
    fn sync_mention(&mut self, index: &FileIndex) {
        if self.slash.active {
            self.mention = MentionState::default();
            return;
        }
        let Some((offset, query)) = detect_mention_trigger(&self.text) else {
            self.mention = MentionState::default();
            return;
        };
        let activating = !self.mention.active;
        self.mention.active = true;
        self.mention.trigger_byte_offset = offset;
        self.mention.query = query;
        if activating {
            self.mention.cursor = 0;
        }
        let hits = index.search(&self.mention.query, MENTION_POPUP_LIMIT);
        self.mention.filtered = hits.into_iter().map(|p| p.to_path_buf()).collect();
        if self.mention.cursor >= self.mention.filtered.len() {
            self.mention.cursor = self.mention.filtered.len().saturating_sub(1);
        }
    }

    fn sync_popups(&mut self, index: &FileIndex, catalog: &SlashCatalog) {
        self.sync_slash(catalog);
        self.sync_mention(index);
    }

    /// Replace the active `@query` segment with `@<path>`.
    fn accept_mention(&mut self) {
        let Some(path) = self.mention.current_path().cloned() else {
            return;
        };
        let cut_at = self.mention.trigger_byte_offset;
        if cut_at > self.text.len() {
            return;
        }
        self.text.truncate(cut_at);
        self.text.push('@');
        self.text.push_str(&path.to_string_lossy());
        self.mention = MentionState::default();
    }
}

/// Startup-splash lifecycle (T1-V6b). Owns just enough state to (a) compute
/// the per-frame `elapsed` for the splash animation and (b) flip the
/// `dismissed` flag once the user submits their first prompt. Pure data —
/// drawing is delegated to [`crate::components::splash::render_splash`].
#[derive(Clone, Debug)]
struct SplashState {
    started_at: Option<Instant>,
    dismissed: bool,
    ctx: SplashContext,
}

impl SplashState {
    fn new(ctx: SplashContext) -> Self {
        Self {
            started_at: None,
            dismissed: false,
            ctx,
        }
    }

    /// Mark the splash visible from `now` if it hasn't been started yet.
    fn ensure_started(&mut self, now: Instant) {
        if self.started_at.is_none() {
            self.started_at = Some(now);
        }
    }

    /// True while the splash should occupy the transcript area.
    fn visible(&self) -> bool {
        !self.dismissed
    }

    /// Compute the animation `elapsed` for the current draw frame.
    fn elapsed_at(&self, now: Instant) -> Duration {
        match self.started_at {
            Some(start) => now.saturating_duration_since(start),
            None => Duration::ZERO,
        }
    }

    /// Called when the user submits their first prompt. Idempotent.
    fn on_first_submit(&mut self) {
        self.dismissed = true;
    }
}
