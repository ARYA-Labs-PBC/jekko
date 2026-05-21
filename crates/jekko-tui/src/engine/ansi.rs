//! ANSI byte stream → Ratatui `Span` parser (COWBOY.md F3).
//!
//! Wraps `vt100::Parser` to convert raw bytes (from a PTY runner or any
//! source that includes ANSI SGR escapes) into styled spans. Sanitizes
//! dangerous OSC sequences before they reach the parser — child processes
//! should NOT be able to write to the host clipboard (OSC 52), change the
//! terminal title (OSC 0/2), or toggle the alternate screen / bracketed
//! paste.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;

const SCREEN_COLS: u16 = 200;
const SCREEN_ROWS: u16 = 200;

pub fn parse_bytes(bytes: &[u8]) -> Vec<Span<'static>> {
    let sanitized = sanitize_osc(bytes);
    let mut parser = vt100::Parser::new(SCREEN_ROWS, SCREEN_COLS, 0);
    parser.process(&sanitized);
    let screen = parser.screen();
    let mut out = Vec::new();
    let mut cur_text = String::new();
    let mut cur_style: Option<Style> = None;
    for row in 0..SCREEN_ROWS {
        for col in 0..SCREEN_COLS {
            let Some(cell) = screen.cell(row, col) else {
                continue;
            };
            let contents = cell.contents();
            if contents.is_empty() {
                continue;
            }
            let style = cell_style(cell);
            if cur_style.map(|s| s != style).unwrap_or(true) {
                if !cur_text.is_empty() {
                    #[allow(clippy::manual_unwrap_or_default)]
                    let span_style = match cur_style {
                        Some(style) => style,
                        None => Style::default(),
                    };
                    out.push(Span::styled(std::mem::take(&mut cur_text), span_style));
                }
                cur_style = Some(style);
            }
            cur_text.push_str(contents);
        }
        if !cur_text.is_empty() {
            cur_text.push('\n');
        }
    }
    if !cur_text.is_empty() {
        #[allow(clippy::manual_unwrap_or_default)]
        let span_style = match cur_style {
            Some(style) => style,
            None => Style::default(),
        };
        out.push(Span::styled(cur_text, span_style));
    }
    out
}

fn cell_style(cell: &vt100::Cell) -> Style {
    let mut style = Style::default()
        .fg(map_color(cell.fgcolor()))
        .bg(map_color(cell.bgcolor()));
    if cell.bold() {
        style = style.add_modifier(Modifier::BOLD);
    }
    if cell.italic() {
        style = style.add_modifier(Modifier::ITALIC);
    }
    if cell.underline() {
        style = style.add_modifier(Modifier::UNDERLINED);
    }
    if cell.inverse() {
        style = style.add_modifier(Modifier::REVERSED);
    }
    style
}

fn map_color(c: vt100::Color) -> Color {
    match c {
        vt100::Color::Default => Color::Reset,
        vt100::Color::Idx(i) => Color::Indexed(i),
        vt100::Color::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}

/// Strip dangerous OSC sequences from a byte stream before parsing.
/// Removes OSC 52 (clipboard), OSC 0/1/2 (title), and the private-mode
/// toggles for alt-screen (?1049h/l, ?47h/l) and bracketed-paste (?2004h/l).
pub fn sanitize_osc(input: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len());
    let mut i = 0;
    while i < input.len() {
        if input[i] == 0x1b && i + 1 < input.len() {
            // OSC = ESC ]
            if input[i + 1] == b']' {
                if let Some(end) = find_osc_end(input, i + 2) {
                    let body = &input[i + 2..end];
                    let drop = body
                        .split(|&b| b == b';')
                        .next()
                        .map(|n| matches!(n, b"0" | b"1" | b"2" | b"52"))
                        .unwrap_or(false);
                    if drop {
                        // Skip ESC ] ... terminator (BEL or ST = ESC \).
                        i = next_after_terminator(input, end);
                        continue;
                    }
                }
            }
            // CSI = ESC [
            if input[i + 1] == b'[' {
                if let Some((end, body)) = find_csi(input, i + 2) {
                    let drop_csi = matches!(
                        body.as_slice(),
                        b"?1049h" | b"?1049l" | b"?47h" | b"?47l" | b"?2004h" | b"?2004l"
                    );
                    if drop_csi {
                        i = end + 1;
                        continue;
                    }
                }
            }
        }
        out.push(input[i]);
        i += 1;
    }
    out
}

fn find_osc_end(input: &[u8], start: usize) -> Option<usize> {
    let mut j = start;
    while j < input.len() {
        if input[j] == 0x07 {
            return Some(j);
        }
        if input[j] == 0x1b && j + 1 < input.len() && input[j + 1] == b'\\' {
            return Some(j);
        }
        j += 1;
    }
    None
}

fn next_after_terminator(input: &[u8], end: usize) -> usize {
    if end < input.len() && input[end] == 0x07 {
        end + 1
    } else if end + 1 < input.len() && input[end] == 0x1b && input[end + 1] == b'\\' {
        end + 2
    } else {
        end
    }
}

fn find_csi(input: &[u8], start: usize) -> Option<(usize, Vec<u8>)> {
    let mut j = start;
    let mut body = Vec::new();
    while j < input.len() {
        let b = input[j];
        body.push(b);
        if b.is_ascii_alphabetic() {
            return Some((j, body));
        }
        j += 1;
        if j - start > 32 {
            return None;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_round_trip() {
        let spans = parse_bytes(b"hello world");
        let joined: String = spans.iter().map(|s| s.content.as_ref()).collect::<String>();
        assert!(joined.contains("hello world"));
    }

    #[test]
    fn ansi_color_applies_style() {
        let spans = parse_bytes(b"\x1b[31mred\x1b[0m");
        let joined: String = spans.iter().map(|s| s.content.as_ref()).collect::<String>();
        assert!(joined.contains("red"));
    }

    #[test]
    fn sanitize_drops_osc_52_clipboard() {
        let with = b"before\x1b]52;c;ZGFuZ2Vy\x07after";
        let sanitized = sanitize_osc(with);
        assert!(!sanitized.windows(3).any(|w| w == b"52;"));
        let s = String::from_utf8_lossy(&sanitized);
        assert!(s.contains("before"));
        assert!(s.contains("after"));
    }

    #[test]
    fn sanitize_drops_title_changes() {
        let with = b"\x1b]0;evil title\x07ok";
        let sanitized = sanitize_osc(with);
        let s = String::from_utf8_lossy(&sanitized);
        assert!(!s.contains("evil"));
        assert!(s.contains("ok"));
    }

    #[test]
    fn sanitize_drops_alt_screen_toggle() {
        let with = b"\x1b[?1049hcontent\x1b[?1049l";
        let sanitized = sanitize_osc(with);
        let s = String::from_utf8_lossy(&sanitized);
        assert!(!s.contains("?1049h"));
        assert!(!s.contains("?1049l"));
        assert!(s.contains("content"));
    }
}
