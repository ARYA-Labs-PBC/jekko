struct InFlight {
    buffer: String,
    last_delta: Instant,
    started_at: Instant,
    cancel: Escalator,
    /// T-SEMANTIC-TRANSCRIPT-A: multi-tool correlation. Insertion order is
    /// preserved by `IndexMap` so the render code can show either the most
    /// recently started chip (the historical contract) or stack them.
    active_tools: IndexMap<String, ActiveToolChip>,
}

impl InFlight {
    fn new() -> Self {
        let token = CancellationToken::new();
        Self {
            buffer: String::new(),
            last_delta: Instant::now(),
            started_at: Instant::now(),
            cancel: Escalator::new(token),
            active_tools: IndexMap::new(),
        }
    }

    /// The most-recently-started tool, used for the single-slot spinner chip
    /// in the agent rail / live tool card. Preserves the pre-refactor visual
    /// contract while completed earlier tools still emit their own card.
    fn latest_tool(&self) -> Option<&ActiveToolChip> {
        self.active_tools.values().next_back()
    }

    fn cancel_token(&self) -> CancellationToken {
        self.cancel.token()
    }

    fn cancel_on_interrupt(&mut self) -> CancelLevel {
        self.cancel.on_esc()
    }

    fn cancel_on_stop(&mut self) -> CancelLevel {
        self.cancel.on_stop()
    }

    fn tick_cancel(&mut self) -> CancelLevel {
        self.cancel.tick()
    }

    fn spinner_glyph(&self, motion_enabled: bool) -> &'static str {
        spinner_glyph_for(self.started_at.elapsed(), motion_enabled)
    }

    /// Apply a streaming tool event. Returns the now-terminal chip (success
    /// or failure) when a `Complete` / `Fail` event arrives, so the caller can
    /// emit a per-tool transcript card + record the persistent OutputBuffer
    /// without having to peek inside `active_tools`.
    ///
    /// `StdoutChunk` / `StderrChunk` for an `id` that was never started (or
    /// has already completed) are silently dropped — the chip is the source
    /// of truth, so dangling chunks have nowhere to land.
    fn apply_tool_event(&mut self, event: ToolEvent) -> Option<ActiveToolChip> {
        match event {
            ToolEvent::Start { id, name, input } => {
                self.active_tools
                    .insert(id.clone(), ActiveToolChip::new(id, name, input));
                None
            }
            ToolEvent::StdoutChunk { id, chunk } => {
                if let Some(tool) = self.active_tools.get_mut(&id) {
                    tool.status = ToolChipStatus::Running;
                    tool.last_chunk = Some(chunk.clone());
                    tool.output.push_str(&chunk);
                    tool.stdout.push_str(&chunk);
                }
                None
            }
            ToolEvent::StderrChunk { id, chunk } => {
                if let Some(tool) = self.active_tools.get_mut(&id) {
                    tool.status = ToolChipStatus::Running;
                    tool.last_chunk = Some(chunk.clone());
                    tool.output.push_str(&chunk);
                    tool.stderr.push_str(&chunk);
                }
                None
            }
            ToolEvent::Complete { id } => self.active_tools.shift_remove(&id).map(|mut tool| {
                tool.status = ToolChipStatus::Success;
                tool
            }),
            ToolEvent::Fail { id, error } => self.active_tools.shift_remove(&id).map(|mut tool| {
                tool.status = ToolChipStatus::Failure;
                tool.last_chunk = Some(error);
                tool
            }),
        }
    }
}

#[derive(Clone, Debug)]
struct ActiveToolChip {
    _id: String,
    name: String,
    input: Option<String>,
    status: ToolChipStatus,
    last_chunk: Option<String>,
    /// Interleaved stdout+stderr stream — used by the live tool card so the
    /// user sees output in arrival order. The pager sidecar reads
    /// `stdout` / `stderr` instead for stream-aware browsing.
    output: String,
    /// Stdout-only stream, retained verbatim for the persistent OutputBuffer
    /// recorded on tool finalization (T-SEMANTIC-TRANSCRIPT-A).
    stdout: String,
    /// Stderr-only stream (see `stdout` above).
    stderr: String,
    started_at: Instant,
}

impl ActiveToolChip {
    fn new(id: String, name: String, input: Option<String>) -> Self {
        Self {
            _id: id,
            name,
            input,
            status: ToolChipStatus::Running,
            last_chunk: None,
            output: String::new(),
            stdout: String::new(),
            stderr: String::new(),
            started_at: Instant::now(),
        }
    }

    /// Build a persistent OutputBuffer from the captured stdout + stderr,
    /// keyed by tool id. Each captured line is pushed individually so the
    /// collapse layer can render head/tail slices. Borrows `self` so the
    /// chip can also feed `render_completed_tool_card` in the same scope.
    fn build_output_buffer(&self) -> OutputBuffer {
        let mut buf = OutputBuffer::new(self._id.clone());
        for line in self.stdout.lines() {
            // `push_line` only fails when the raw-log directory isn't writable.
            // The in-memory buffer is independent of that path, so dropping
            // the error keeps the pager useful in sandboxed test environments.
            let _ = buf.push_line(line.to_string());
        }
        for line in self.stderr.lines() {
            let _ = buf.push_line(line.to_string());
        }
        // Fallback: if both stream-tagged buffers are empty but the
        // interleaved `output` field has content (older chunks routed only
        // through the compatibility path), seed from `output` so the pager isn't
        // empty for completed tools.
        if buf.line_count() == 0 && !self.output.is_empty() {
            for line in self.output.lines() {
                let _ = buf.push_line(line.to_string());
            }
        }
        buf
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ToolChipStatus {
    Running,
    Success,
    Failure,
}

fn spinner_glyph_for(elapsed: Duration, motion_enabled: bool) -> &'static str {
    const FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    if !motion_enabled {
        return FRAMES[0];
    }
    let idx = (elapsed.as_millis() / 100) as usize % FRAMES.len();
    FRAMES[idx]
}
