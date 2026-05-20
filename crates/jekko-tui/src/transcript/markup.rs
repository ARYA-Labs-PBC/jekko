//! Semantic inline-markup parser (COWBOY.md E1).
//!
//! Parses `{role}text{/}`, `{role:modifier}text{/}`, and `{pulse:a:b}text{/}`
//! syntax inside assistant text + tool descriptions. Output is `Vec<Span>`
//! styled via `theme::codex::*` tokens. Parse at event creation, not render.
//!
//! Supported roles (each maps to a color/modifier set):
//!   path, file        → BLUE_PATH
//!   cmd, command      → FG_STRONG + bold
//!   flag              → CYAN_TAB
//!   model             → ORANGE_AGENT
//!   pwd, cwd          → BLUE_PATH + dim
//!   branch            → CYAN_TAB
//!   diff_add, add     → GREEN_OK
//!   diff_del, del     → SALMON_FAIL
//!   error             → SALMON_FAIL + bold
//!   success           → GREEN_OK
//!   warning, warn     → YELLOW
//!   dim               → FG_DIM
//!   muted             → FG_VERY_DIM
//!   bold              → FG_STRONG + bold
//!   italic            → FG + italic
//!
//! Special: `{pulse:fg:bg}text{/}` — record the request as a Pulse span which
//! the runtime later swaps for an oscillating span via anim helpers. v1
//! flattens to a plain Span with the fg color (no animation in this module).
//!
//! Escape: `\{` / `\}` for literal braces. Unknown roles render as `{role}`
//! literal so user-typed text with braces isn't silently swallowed.

use ratatui::style::{Modifier, Style};
use ratatui::text::Span;

use crate::theme::{
    codex_blue_path, codex_cyan_tab, codex_fg, codex_fg_dim, codex_fg_strong, codex_fg_very_dim,
    codex_green_ok, codex_orange_agent, codex_pink_agent, codex_salmon_fail, codex_yellow,
};

pub fn parse(input: &str) -> Vec<Span<'static>> {
    let mut out = Vec::new();
    let mut buf = String::new();
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'\\' && i + 1 < bytes.len() && (bytes[i + 1] == b'{' || bytes[i + 1] == b'}') {
            buf.push(bytes[i + 1] as char);
            i += 2;
            continue;
        }
        if b == b'{' {
            if let Some(close) = find_tag_close(input, i + 1) {
                let tag = &input[i + 1..close];
                let body_start = close + 1;
                if let Some(end_open) = input[body_start..].find("{/}") {
                    let body_end = body_start + end_open;
                    let body = &input[body_start..body_end];
                    if !buf.is_empty() {
                        out.push(Span::raw(std::mem::take(&mut buf)));
                    }
                    out.push(span_for_role(tag, body));
                    i = body_end + 3;
                    continue;
                }
            }
        }
        buf.push(b as char);
        i += 1;
    }
    if !buf.is_empty() {
        out.push(Span::raw(buf));
    }
    out
}

fn find_tag_close(input: &str, start: usize) -> Option<usize> {
    let bytes = input.as_bytes();
    for j in start..bytes.len() {
        if bytes[j] == b'}' {
            return Some(j);
        }
        if bytes[j] == b'{' {
            return None;
        }
    }
    None
}

fn span_for_role(tag: &str, body: &str) -> Span<'static> {
    let body_owned = body.to_string();
    if let Some(rest) = tag.strip_prefix("pulse:") {
        let fg_name = rest.split(':').next().unwrap_or("");
        let fg = named_color(fg_name).unwrap_or_else(codex_fg);
        return Span::styled(body_owned, Style::default().fg(fg));
    }
    let (role, _modifier) = match tag.split_once(':') {
        Some((r, m)) => (r, Some(m)),
        None => (tag, None),
    };
    let style = match role {
        "path" | "file" => Style::default().fg(codex_blue_path()),
        "cmd" | "command" => Style::default()
            .fg(codex_fg_strong())
            .add_modifier(Modifier::BOLD),
        "flag" => Style::default().fg(codex_cyan_tab()),
        "model" => Style::default().fg(codex_orange_agent()),
        "pwd" | "cwd" => Style::default()
            .fg(codex_blue_path())
            .add_modifier(Modifier::DIM),
        "branch" => Style::default().fg(codex_cyan_tab()),
        "diff_add" | "add" => Style::default().fg(codex_green_ok()),
        "diff_del" | "del" => Style::default().fg(codex_salmon_fail()),
        "error" => Style::default()
            .fg(codex_salmon_fail())
            .add_modifier(Modifier::BOLD),
        "success" => Style::default().fg(codex_green_ok()),
        "warning" | "warn" => Style::default().fg(codex_yellow()),
        "dim" => Style::default().fg(codex_fg_dim()),
        "muted" => Style::default().fg(codex_fg_very_dim()),
        "bold" => Style::default()
            .fg(codex_fg_strong())
            .add_modifier(Modifier::BOLD),
        "italic" => Style::default()
            .fg(codex_fg())
            .add_modifier(Modifier::ITALIC),
        _ => {
            return Span::raw(format!("{{{tag}}}{body_owned}{{/}}"));
        }
    };
    Span::styled(body_owned, style)
}

fn named_color(name: &str) -> Option<ratatui::style::Color> {
    Some(match name {
        "cyan" => codex_cyan_tab(),
        "white" => codex_fg_strong(),
        "green" => codex_green_ok(),
        "red" | "salmon" => codex_salmon_fail(),
        "orange" => codex_orange_agent(),
        "pink" => codex_pink_agent(),
        "yellow" => codex_yellow(),
        "blue" => codex_blue_path(),
        "dim" => codex_fg_dim(),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::codex;

    #[test]
    fn plain_text_passthrough() {
        let spans = parse("hello world");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content, "hello world");
    }

    #[test]
    fn path_role() {
        let spans = parse("see {path}src/lib.rs{/} for details");
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].content, "see ");
        assert_eq!(spans[1].content, "src/lib.rs");
        assert_eq!(spans[1].style.fg, Some(codex::BLUE_PATH));
        assert_eq!(spans[2].content, " for details");
    }

    #[test]
    fn error_role_is_bold_salmon() {
        let spans = parse("{error}gateway unreachable{/}");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].style.fg, Some(codex::SALMON_FAIL));
        assert!(spans[0].style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn pulse_uses_first_color() {
        let spans = parse("{pulse:cyan:white}Working{/}");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content, "Working");
        assert_eq!(spans[0].style.fg, Some(codex::CYAN_TAB));
    }

    #[test]
    fn nested_pulse_flattens_inner_markup_to_literal_text() {
        let spans = parse("{pulse:cyan:white}{text}{/}");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content, "{text}");
        assert_eq!(spans[0].style.fg, Some(codex::CYAN_TAB));
    }

    #[test]
    fn unknown_role_renders_literal() {
        let spans = parse("{frobnicate}text{/}");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content, "{frobnicate}text{/}");
    }

    #[test]
    fn escaped_braces_are_literal() {
        let spans = parse("\\{not a tag\\}");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content, "{not a tag}");
    }

    #[test]
    fn unmatched_open_brace_is_literal() {
        let spans = parse("a { b");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content, "a { b");
    }

    #[test]
    fn malformed_close_is_literal() {
        let spans = parse("{path}src/lib.rs{/ nope");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content, "{path}src/lib.rs{/ nope");
    }

    #[test]
    fn missing_close_is_literal() {
        let spans = parse("{path}src/lib.rs");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content, "{path}src/lib.rs");
    }

    #[test]
    fn multiple_tags_in_sequence() {
        let spans = parse("{success}ok{/} and {error}fail{/}");
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].content, "ok");
        assert_eq!(spans[0].style.fg, Some(codex::GREEN_OK));
        assert_eq!(spans[1].content, " and ");
        assert_eq!(spans[2].content, "fail");
        assert_eq!(spans[2].style.fg, Some(codex::SALMON_FAIL));
    }

    #[test]
    fn nested_braces_inside_text_are_literal() {
        let spans = parse("{cmd}rg {pattern}{/}");
        assert_eq!(spans.len(), 1);
        assert!(spans[0].content.contains("pattern"));
    }
}
