//! OSC 52 clipboard write helper (COWBOY.md M6 / referenced in Phase B Copy task).
//!
//! Writes a payload to the host terminal's clipboard via the OSC 52 escape
//! sequence. Works in iTerm2, kitty, Alacritty, WezTerm, foot, ghostty, modern
//! tmux + screen, and most macOS/Linux terminals. Does NOT work in:
//!   - the basic macOS Terminal.app (Apple disabled OSC 52 long ago)
//!   - tmux without `set -g allow-passthrough on` and `set -g set-clipboard on`
//!   - GNU screen without `defbce` + OSC passthrough config
//!
//! Payload size: most terminals cap OSC52/base64 payloads at ~75-100KB. The
//! `write` fn here keeps the encoded payload under 80KB with a trailing
//! `\n…[+N chars truncated]` marker so the pasted output is honest about what
//! got cut.
//!
//! Wire-up: chat_runtime imports this from `/copy` action and `Ctrl+Shift+C`
//! key. Both call `osc52::copy_to_clipboard(transcript_text)`.

use std::io::{self, Write};

use base64::Engine as _;

/// Max payload size before truncation (in bytes of base64 output, conservative).
/// Most terminals cap around 100KB base64; we stay well under.
pub const MAX_PAYLOAD_BYTES: usize = 80 * 1024;

/// Write `payload` to the host terminal's clipboard via OSC 52. Returns the
/// number of bytes (of the original `payload`) actually copied — the rest
/// was truncated.
///
/// The `c;` selector is the standard clipboard target. Use `p;` for primary
/// (X11) if needed — we default to clipboard since that's what the user
/// typically expects from Ctrl+Shift+C.
pub fn copy_to_clipboard(payload: &str) -> io::Result<usize> {
    write_to(payload, ClipboardTarget::Clipboard, &mut io::stdout())
}

#[derive(Clone, Copy, Debug)]
pub enum ClipboardTarget {
    Clipboard,
    Primary,
}

impl ClipboardTarget {
    fn selector(self) -> &'static str {
        match self {
            Self::Clipboard => "c",
            Self::Primary => "p",
        }
    }
}

/// Like `copy_to_clipboard` but lets the caller pick the target + the writer.
/// Returns the number of original-payload bytes written.
pub fn write_to<W: Write>(
    payload: &str,
    target: ClipboardTarget,
    out: &mut W,
) -> io::Result<usize> {
    let (truncated, copied_len) = truncate_for_payload(payload);
    let encoded = base64::engine::general_purpose::STANDARD.encode(truncated.as_bytes());
    write!(out, "\x1b]52;{};{}\x07", target.selector(), encoded)?;
    out.flush()?;
    Ok(copied_len)
}

/// Truncate so the base64 OSC52 payload stays within `MAX_PAYLOAD_BYTES`,
/// appending a `\n…[+N chars truncated]` marker when truncation happens.
fn truncate_for_payload(payload: &str) -> (String, usize) {
    let original = payload.len();
    if encoded_len(original) <= MAX_PAYLOAD_BYTES {
        return (payload.to_string(), original);
    }

    let max_raw_payload_bytes = max_raw_bytes_for_encoded_cap(MAX_PAYLOAD_BYTES);
    let mut end = max_raw_payload_bytes.min(payload.len());

    loop {
        // Walk character boundary so we don't slice a UTF-8 codepoint.
        while !payload.is_char_boundary(end) && end > 0 {
            end -= 1;
        }

        let truncated_chars = payload[end..].chars().count();
        let marker = truncation_marker(truncated_chars);
        let max_head_bytes = max_raw_payload_bytes.saturating_sub(marker.len());
        if end <= max_head_bytes {
            let head = &payload[..end];
            let mut out = String::with_capacity(end + marker.len());
            out.push_str(head);
            out.push_str(&marker);
            return (out, end);
        }

        end = max_head_bytes.min(end.saturating_sub(1));
    }
}

fn truncation_marker(truncated_count: usize) -> String {
    format!("\n…[+{truncated_count} chars truncated]")
}

fn encoded_len(raw_bytes: usize) -> usize {
    raw_bytes.div_ceil(3) * 4
}

fn max_raw_bytes_for_encoded_cap(encoded_cap: usize) -> usize {
    (encoded_cap / 4) * 3
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encoded_payload<'a>(buf: &'a [u8], selector: &str) -> &'a str {
        let s = std::str::from_utf8(buf).unwrap();
        let prefix = format!("\x1b]52;{selector};");
        let start = s.find(&prefix).unwrap() + prefix.len();
        let end = s.rfind('\x07').unwrap();
        &s[start..end]
    }

    fn decoded_payload(buf: &[u8], selector: &str) -> String {
        let b64 = encoded_payload(buf, selector);
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(b64.as_bytes())
            .unwrap();
        String::from_utf8(decoded).unwrap()
    }

    #[test]
    fn small_payload_round_trips() {
        let mut buf: Vec<u8> = Vec::new();
        let n = write_to("hello clipboard", ClipboardTarget::Clipboard, &mut buf).unwrap();
        assert_eq!(n, "hello clipboard".len());
        let s = String::from_utf8_lossy(&buf);
        assert!(s.starts_with("\x1b]52;c;"));
        assert!(s.ends_with('\x07'));
    }

    #[test]
    fn primary_target_uses_p_selector() {
        let mut buf: Vec<u8> = Vec::new();
        write_to("x", ClipboardTarget::Primary, &mut buf).unwrap();
        let s = String::from_utf8_lossy(&buf);
        assert!(s.contains(";p;"));
    }

    #[test]
    fn base64_payload_decodes_to_input() {
        let mut buf: Vec<u8> = Vec::new();
        let input = "the quick brown fox\nover the lazy dog";
        write_to(input, ClipboardTarget::Clipboard, &mut buf).unwrap();
        assert_eq!(decoded_payload(&buf, "c"), input);
    }

    #[test]
    fn max_encoded_payload_without_marker_fits_exactly() {
        let input = "a".repeat(max_raw_bytes_for_encoded_cap(MAX_PAYLOAD_BYTES));
        let mut buf: Vec<u8> = Vec::new();
        let copied = write_to(&input, ClipboardTarget::Clipboard, &mut buf).unwrap();
        let b64 = encoded_payload(&buf, "c");

        assert_eq!(copied, input.len());
        assert_eq!(b64.len(), MAX_PAYLOAD_BYTES);
        assert_eq!(decoded_payload(&buf, "c"), input);
    }

    #[test]
    fn one_extra_raw_byte_is_truncated_to_encoded_cap() {
        let input = "a".repeat(max_raw_bytes_for_encoded_cap(MAX_PAYLOAD_BYTES) + 1);
        let mut buf: Vec<u8> = Vec::new();
        let copied = write_to(&input, ClipboardTarget::Clipboard, &mut buf).unwrap();
        let b64 = encoded_payload(&buf, "c");
        let text = decoded_payload(&buf, "c");

        assert!(copied < input.len());
        assert!(b64.len() <= MAX_PAYLOAD_BYTES);
        assert!(text.contains("chars truncated"));
        assert_eq!(copied, text.find("\n…").unwrap());
    }

    #[test]
    fn oversize_payload_truncated_with_marker() {
        let huge = "a".repeat(MAX_PAYLOAD_BYTES * 2);
        let mut buf: Vec<u8> = Vec::new();
        let copied = write_to(&huge, ClipboardTarget::Clipboard, &mut buf).unwrap();
        let b64 = encoded_payload(&buf, "c");
        let text = decoded_payload(&buf, "c");

        assert!(copied < huge.len());
        assert!(b64.len() <= MAX_PAYLOAD_BYTES);
        assert!(text.contains("chars truncated"));
        assert_eq!(copied, text.find("\n…").unwrap());
    }

    #[test]
    fn utf8_boundary_safe() {
        // String of multi-byte chars that crosses the truncation boundary.
        let s = "🦀".repeat(MAX_PAYLOAD_BYTES);
        let mut buf: Vec<u8> = Vec::new();
        let copied = write_to(&s, ClipboardTarget::Clipboard, &mut buf).unwrap();
        let b64 = encoded_payload(&buf, "c");
        let text = decoded_payload(&buf, "c");

        assert!(s.is_char_boundary(copied));
        assert!(b64.len() <= MAX_PAYLOAD_BYTES);
        assert!(text.contains("chars truncated"));
    }

    #[test]
    fn utf8_marker_counts_chars_not_bytes() {
        let raw_budget = max_raw_bytes_for_encoded_cap(MAX_PAYLOAD_BYTES);
        let s = format!("{}{}", "a".repeat(raw_budget), "🦀🦀🦀");
        let mut buf: Vec<u8> = Vec::new();
        write_to(&s, ClipboardTarget::Clipboard, &mut buf).unwrap();
        let text = decoded_payload(&buf, "c");

        assert!(text.contains("chars truncated]"));
        assert!(
            !text.contains("bytes truncated]"),
            "marker should keep the user-facing chars wording"
        );
        assert!(
            !text.contains("+12 chars truncated]"),
            "marker should not count the three crab emoji as twelve bytes"
        );
    }
}
