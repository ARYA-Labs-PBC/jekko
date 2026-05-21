/// Single match hit: which line, and the byte range inside that line.
///
/// `byte_start..byte_end` is a UTF-8 byte slice into
/// [`PagerState::lines`]`[line]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchRef {
    pub line: usize,
    pub byte_start: usize,
    pub byte_end: usize,
}

/// Which sub-mode the pager is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PagerMode {
    /// Normal scroll/yank.
    Browse,
    /// User is typing into the `/` prompt; Enter commits, Esc cancels.
    Search,
    /// Search complete, current/other match styling is live and `n`/`N`
    /// cycle through hits.
    Highlight,
}

/// Pager state. Cheap to clone is not a goal -- owners hand a `&mut` to the
/// input and render entry points.
#[derive(Clone, Debug)]
pub struct PagerState {
    pub lines: Vec<String>,
    pub scroll: usize,
    pub search_query: String,
    pub matches: Vec<MatchRef>,
    pub current_match: Option<usize>,
    pub mode: PagerMode,
}

impl PagerState {
    /// Build a fresh pager around `lines`. Starts in `Browse` at the top.
    pub fn new(lines: Vec<String>) -> Self {
        Self {
            lines,
            scroll: 0,
            search_query: String::new(),
            matches: Vec::new(),
            current_match: None,
            mode: PagerMode::Browse,
        }
    }

    /// Total number of body lines available to scroll through.
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Move the viewport up by `n` lines, clamped at the top.
    pub fn scroll_up(&mut self, n: usize) {
        self.scroll = self.scroll.saturating_sub(n);
    }

    /// Move the viewport down by `n` lines, clamped so the last visible row
    /// stays inside the buffer.
    pub fn scroll_down(&mut self, n: usize) {
        let max = self.lines.len().saturating_sub(1);
        self.scroll = self.scroll.saturating_add(n).min(max);
    }

    /// PageUp: scroll by the size of one viewport.
    pub fn page_up(&mut self, viewport_height: usize) {
        let step = viewport_height.max(1);
        self.scroll_up(step);
    }

    /// PageDown: scroll by the size of one viewport.
    pub fn page_down(&mut self, viewport_height: usize) {
        let step = viewport_height.max(1);
        self.scroll_down(step);
    }

    /// Jump to the very top.
    pub fn home(&mut self) {
        self.scroll = 0;
    }

    /// Jump so the last line of the buffer sits at the bottom of the
    /// viewport, or at the top if the buffer is shorter than the viewport.
    pub fn end(&mut self, viewport_height: usize) {
        let total = self.lines.len();
        let h = viewport_height.max(1);
        self.scroll = total.saturating_sub(h);
    }

    /// Enter `Search` mode and reset the query buffer.
    pub fn start_search(&mut self) {
        self.mode = PagerMode::Search;
        self.search_query.clear();
    }

    /// Append a character to the in-progress search query.
    pub fn push_search_char(&mut self, c: char) {
        self.search_query.push(c);
    }

    /// Drop the last character of the in-progress search query.
    pub fn pop_search_char(&mut self) {
        self.search_query.pop();
    }

    /// Abort search input and return to `Browse`.
    pub fn cancel_search(&mut self) {
        self.mode = PagerMode::Browse;
        self.search_query.clear();
        self.matches.clear();
        self.current_match = None;
    }

    /// Commit the current query: scan lines, populate `matches`, transition
    /// to `Highlight`, and jump to the first match.
    pub fn commit_search(&mut self) {
        let query = self.search_query.clone();
        self.matches.clear();
        self.current_match = None;
        if !query.is_empty() {
            for (line_idx, line) in self.lines.iter().enumerate() {
                for (byte_start, hit) in line.match_indices(&query) {
                    self.matches.push(MatchRef {
                        line: line_idx,
                        byte_start,
                        byte_end: byte_start + hit.len(),
                    });
                }
            }
        }
        self.mode = PagerMode::Highlight;
        if !self.matches.is_empty() {
            self.current_match = Some(0);
            self.jump_to_current(usize::MAX);
        }
    }

    /// Step to the next match, wrapping at the end.
    pub fn next_match(&mut self, viewport_height: usize) {
        if self.matches.is_empty() {
            return;
        }
        let cur = self.current_match.unwrap_or(0);
        let next = (cur + 1) % self.matches.len();
        self.current_match = Some(next);
        self.jump_to_current(viewport_height);
    }

    /// Step to the previous match, wrapping at the start.
    pub fn prev_match(&mut self, viewport_height: usize) {
        if self.matches.is_empty() {
            return;
        }
        let cur = self.current_match.unwrap_or(0);
        let len = self.matches.len();
        let prev = (cur + len - 1) % len;
        self.current_match = Some(prev);
        self.jump_to_current(viewport_height);
    }

    /// Borrow the currently-selected match, if any.
    pub fn selected_match(&self) -> Option<&MatchRef> {
        self.current_match.and_then(|i| self.matches.get(i))
    }

    fn jump_to_current(&mut self, viewport_height: usize) {
        let Some(target_line) = self.selected_match().map(|m| m.line) else {
            return;
        };
        let h = viewport_height.max(1);
        let bias = h / 3;
        let new_scroll = target_line.saturating_sub(bias);
        let total = self.lines.len();
        let max_scroll = total.saturating_sub(1);
        self.scroll = new_scroll.min(max_scroll);
    }
}
