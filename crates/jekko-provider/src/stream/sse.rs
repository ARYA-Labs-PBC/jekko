use bytes::Bytes;
use serde::{Deserialize, Serialize};

/// Per-frame SSE result.
#[derive(Debug, Clone, PartialEq)]
pub struct SseFrame {
    /// `event:` field (may be empty).
    pub event: String,
    /// `data:` field (may span multiple `data:` lines).
    pub data: String,
    /// `id:` field, if present.
    pub id: Option<String>,
    /// `retry:` field, if present.
    pub retry: Option<u64>,
}

/// Provider capability hint surfaced by adapters.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderCapabilities {
    /// Adapter supports streaming SSE output.
    pub streaming: bool,
    /// Adapter supports cache_control markers.
    pub cache_control: bool,
    /// Adapter supports tool-call streaming.
    pub tool_streaming: bool,
}

/// Incremental SSE decoder: accepts byte chunks and yields [`SseFrame`] blocks
/// once a complete event terminator is observed.
#[derive(Debug, Default)]
pub struct SseDecoder {
    buf: String,
}

impl SseDecoder {
    /// Construct an empty decoder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed a chunk of bytes and return all complete frames produced.
    pub fn feed(&mut self, chunk: &Bytes) -> Vec<SseFrame> {
        if let Ok(s) = std::str::from_utf8(chunk) {
            self.buf.push_str(s);
        } else {
            self.buf.push_str(&String::from_utf8_lossy(chunk));
        }
        self.drain()
    }

    /// Flush any remaining buffered data as a final frame (used at EOF).
    pub fn flush(&mut self) -> Vec<SseFrame> {
        if self.buf.is_empty() {
            return Vec::new();
        }
        if !self.buf.ends_with("\n\n") {
            self.buf.push_str("\n\n");
        }
        self.drain()
    }

    fn drain(&mut self) -> Vec<SseFrame> {
        let mut out = Vec::new();
        while let Some((block, rest)) = split_event(&self.buf) {
            if let Some(frame) = parse_sse_block(block) {
                out.push(frame);
            }
            self.buf = rest.to_string();
        }
        out
    }
}

fn split_event(buf: &str) -> Option<(&str, &str)> {
    let bytes = buf.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i < len {
        if bytes[i] == b'\n' && bytes.get(i + 1).copied() == Some(b'\n') {
            return Some((&buf[..i], &buf[i + 2..]));
        }
        if bytes[i] == b'\r'
            && bytes.get(i + 1).copied() == Some(b'\n')
            && bytes.get(i + 2).copied() == Some(b'\r')
            && bytes.get(i + 3).copied() == Some(b'\n')
        {
            return Some((&buf[..i], &buf[i + 4..]));
        }
        i += 1;
    }
    None
}

/// Parse a single SSE block (text between two blank lines) into a [`SseFrame`].
///
/// Returns `None` for blocks that contain only comments or whitespace.
pub fn parse_sse_block(block: &str) -> Option<SseFrame> {
    let mut event = String::new();
    let mut data = String::new();
    let mut id: Option<String> = None;
    let mut retry: Option<u64> = None;
    let mut any = false;

    for raw_line in block.split('\n') {
        let line = raw_line.trim_end_matches('\r');
        if line.is_empty() || line.starts_with(':') {
            continue;
        }
        any = true;
        let (key, rest) = match line.split_once(':') {
            Some((k, r)) => (k, r),
            None => (line, ""),
        };
        let value = rest.strip_prefix(' ').unwrap_or(rest);
        match key {
            "event" => event = value.to_string(),
            "data" => {
                if !data.is_empty() {
                    data.push('\n');
                }
                data.push_str(value);
            }
            "id" => id = Some(value.to_string()),
            "retry" => retry = value.parse::<u64>().ok(),
            _ => {}
        }
    }

    if !any {
        return None;
    }
    Some(SseFrame {
        event,
        data,
        id,
        retry,
    })
}
