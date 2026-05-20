//! COWBOY.md X1 — golden snapshot tests for inline-viewport transcript cards
//! and the boot loader.
//!
//! Each test renders one card kind into a `TestBackend` buffer and snapshots
//! the cell symbols (style stripped) so diffs stay stable across truecolor
//! tweaks.

use insta::assert_snapshot;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};
use ratatui::Terminal;

use std::time::Duration;

use jekko_tui::components::boot_inline::{render_inline_boot_block, BootContext};
use jekko_tui::components::footer_status::{render_footer_status, FooterInfo};
use jekko_tui::components::permission_banner::{
    render_permission_banner, HINT_AGENT_PANEL_FOCUS, HINT_CHAT_FOCUS,
};
use jekko_tui::components::working_strip::render_working_strip;
use jekko_tui::inline_runtime::render_composer_row;
use jekko_tui::prompt::{PROMPT_GLYPH, PROMPT_PREFIX_WIDTH};
use jekko_tui::theme::codex;
use jekko_tui::transcript::inline_cards::{
    render_assistant, render_diff, render_permission_chip, render_question_chip, render_reasoning,
    render_session_header, render_system_notice, render_tool_call, render_user, ActionStatus,
    DiffLine, DiffLineKind, NoticeKind, ToolCall,
};

fn render_lines(width: u16, height: u16, lines: &[Line<'static>]) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            let buf = frame.buffer_mut();
            for (i, line) in lines.iter().enumerate() {
                if (i as u16) >= height {
                    break;
                }
                let area = Rect::new(0, i as u16, width, 1);
                Paragraph::new(line.clone()).render(area, buf);
            }
        })
        .unwrap();
    buffer_to_symbols(terminal.backend().buffer())
}

fn buffer_to_symbols(buf: &Buffer) -> String {
    let area = buf.area();
    let mut out = String::new();
    for y in 0..area.height {
        for x in 0..area.width {
            let cell = &buf[(area.x + x, area.y + y)];
            out.push_str(cell.symbol());
        }
        if y + 1 < area.height {
            out.push('\n');
        }
    }
    // Trim trailing spaces per line for readable snapshots.
    out.lines()
        .map(|l| l.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
}

fn boot_ctx() -> BootContext {
    BootContext {
        version: "0.1.0".into(),
        cwd_display: "~/code/jekko".into(),
        branch: Some("main".into()),
    }
}

#[test]
fn boot_block_80_cols() {
    let ctx = boot_ctx();
    let lines = render_inline_boot_block(&ctx, 80);
    let out = render_lines(80, 4, &lines);
    assert_snapshot!("boot_block_80_cols", out);
}

#[test]
fn boot_block_120_cols() {
    let ctx = boot_ctx();
    let lines = render_inline_boot_block(&ctx, 120);
    let out = render_lines(120, 4, &lines);
    assert_snapshot!("boot_block_120_cols", out);
}

#[test]
fn user_card_short() {
    let lines = render_user("hello world");
    let out = render_lines(80, 1, &lines);
    assert_snapshot!("user_card_short", out);
}

#[test]
fn user_card_multiline() {
    let lines = render_user("hello\nworld\nthird line");
    let out = render_lines(80, 3, &lines);
    assert_snapshot!("user_card_multiline", out);
}

#[test]
fn assistant_card_basic() {
    let lines = render_assistant("here is the answer.");
    let out = render_lines(80, 1, &lines);
    assert_snapshot!("assistant_card_basic", out);
}

#[test]
fn reasoning_card_italic() {
    let lines = render_reasoning("thinking about edge cases…");
    let out = render_lines(80, 1, &lines);
    assert_snapshot!("reasoning_card_italic", out);
}

#[test]
fn session_started_notice() {
    let lines = render_system_notice(NoticeKind::Info, "session started: session_1 · hello");
    let out = render_lines(80, 1, &lines);
    assert_snapshot!("session_started_notice", out);
}

#[test]
fn daemon_status_notice() {
    let lines = render_system_notice(NoticeKind::Warn, "daemon offline: session_1 · retrying");
    let out = render_lines(80, 1, &lines);
    assert_snapshot!("daemon_status_notice", out);
}

#[test]
fn permission_chip_once_always_reject() {
    let lines = render_permission_chip(
        "perm_1",
        "session_1",
        "bash",
        &["ls".into(), "pwd".into()],
        &["ls".into()],
    );
    let out = render_lines(80, 1, &lines);
    assert_snapshot!("permission_chip_once_always_reject", out);
}

#[test]
fn question_chip_with_choices() {
    let lines = render_question_chip(
        "question_1",
        "session_1",
        "Continue?",
        &["yes".into(), "no".into(), "later".into()],
    );
    let out = render_lines(80, 1, &lines);
    assert_snapshot!("question_chip_with_choices", out);
}

#[test]
fn tool_call_bash_success() {
    let output = vec!["M  src/a.rs".to_string(), "M  src/b.rs".to_string()];
    let call = ToolCall {
        verb: "Bash",
        args: "git status --short",
        status: ActionStatus::Success,
        output: &output,
        max_output_lines: 5,
    };
    let lines = render_tool_call(&call);
    let out = render_lines(80, 3, &lines);
    assert_snapshot!("tool_call_bash_success", out);
}

#[test]
fn tool_call_bash_failure_with_collapse() {
    let output: Vec<String> = (0..12).map(|i| format!("line {i}")).collect();
    let call = ToolCall {
        verb: "Bash",
        args: "git status --short",
        status: ActionStatus::Failure,
        output: &output,
        max_output_lines: 3,
    };
    let lines = render_tool_call(&call);
    // header + 3 visible + 1 collapsed marker = 5
    let out = render_lines(80, 5, &lines);
    assert_snapshot!("tool_call_bash_failure_with_collapse", out);
}

#[test]
fn diff_added_context_removed() {
    let hunks = vec![
        DiffLine {
            kind: DiffLineKind::Context,
            old_lineno: Some(482),
            new_lineno: Some(482),
            text: "fn main() {",
        },
        DiffLine {
            kind: DiffLineKind::Added,
            old_lineno: None,
            new_lineno: Some(483),
            text: "    println!(\"hello\");",
        },
        DiffLine {
            kind: DiffLineKind::Removed,
            old_lineno: Some(484),
            new_lineno: None,
            text: "    todo!();",
        },
    ];
    let lines = render_diff("src/main.rs", &hunks);
    let out = render_lines(80, 4, &lines);
    assert_snapshot!("diff_added_context_removed", out);
}

#[test]
fn system_notice_error() {
    let lines = render_system_notice(NoticeKind::Error, "gateway unreachable\nretry in 5s");
    let out = render_lines(80, 2, &lines);
    assert_snapshot!("system_notice_error", out);
}

#[test]
fn session_header() {
    let lines = render_session_header("0.1.0", "~/code/jekko", Some("main"));
    let out = render_lines(80, 1, &lines);
    assert_snapshot!("session_header", out);
}

#[test]
fn assistant_card_uses_orange_margin() {
    let lines = render_assistant("ok");
    let expected = if std::env::var_os("NO_COLOR").is_some() {
        ratatui::style::Color::Reset
    } else {
        codex::ORANGE_AGENT
    };
    assert_eq!(lines[0].spans[0].style.fg, Some(expected));
}

// ── X2: slash popup snapshots ────────────────────────────────────────────────
//
// The runtime's `render_slash_popup` and `SlashState` are private. Per COWBOY
// guidance (approach a), we mirror the visual grammar here so the snapshot
// locks in the layout — symbols only, no style introspection.

#[derive(Clone, Copy)]
struct SlashCmdFx {
    id: &'static str,
    description: &'static str,
}

const SLASH_FX: &[SlashCmdFx] = &[
    SlashCmdFx {
        id: "help",
        description: "show keyboard shortcuts and available commands",
    },
    SlashCmdFx {
        id: "new",
        description: "start a new session (pushes a divider into scrollback)",
    },
    SlashCmdFx {
        id: "clear",
        description: "push a divider into scrollback",
    },
    SlashCmdFx {
        id: "echo",
        description: "toggle the echo backend (v1 placeholder)",
    },
    SlashCmdFx {
        id: "quit",
        description: "exit the inline chat surface",
    },
];

fn slash_filtered(query: &str) -> Vec<usize> {
    let q = query.to_lowercase();
    SLASH_FX
        .iter()
        .enumerate()
        .filter(|(_, cmd)| q.is_empty() || cmd.id.starts_with(&q))
        .map(|(i, _)| i)
        .collect()
}

fn render_slash_popup_fx(width: u16, height: u16, query: &str, cursor: usize) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            let buf = frame.buffer_mut();
            let popup_h = height.saturating_sub(4);

            // Popup title row.
            let title = Line::from(vec![
                Span::raw(" / commands  "),
                Span::raw(format!(
                    "({}/{})",
                    slash_filtered(query).len(),
                    SLASH_FX.len()
                )),
            ]);
            if popup_h > 0 {
                Paragraph::new(title).render(Rect::new(0, 0, width, 1), buf);
            }

            // Popup body rows.
            let filtered = slash_filtered(query);
            let body_rows = popup_h.saturating_sub(1) as usize;
            if popup_h > 0 {
                if filtered.is_empty() {
                    if body_rows > 0 {
                        let empty = Line::from(Span::raw("   (no matches)"));
                        Paragraph::new(empty).render(Rect::new(0, 1, width, 1), buf);
                    }
                } else if body_rows > 0 {
                    let cur = cursor.min(filtered.len() - 1);
                    let start = cur.saturating_sub(body_rows - 1);
                    let end = (start + body_rows).min(filtered.len());
                    for (offset, idx_pos) in (start..end).enumerate() {
                        let cmd = &SLASH_FX[filtered[idx_pos]];
                        let selected = idx_pos == cur;
                        let marker = if selected { " › " } else { "   " };
                        let row = Line::from(vec![
                            Span::raw(marker),
                            Span::raw(format!("/{:<10}", cmd.id)),
                            Span::raw(" "),
                            Span::raw(cmd.description),
                        ]);
                        let y = 1 + offset as u16;
                        Paragraph::new(row).render(Rect::new(0, y, width, 1), buf);
                    }
                }
            }

            // Composer chrome rules + row + shortcuts strip.
            let rule = "─".repeat(width as usize);
            Paragraph::new(Line::from(Span::raw(rule.clone())))
                .render(Rect::new(0, popup_h, width, 1), buf);

            let composer = Line::from(vec![
                Span::raw("› "),
                Span::raw(format!("/{}", query)),
                Span::raw(" "),
            ]);
            Paragraph::new(composer).render(Rect::new(0, popup_h + 1, width, 1), buf);

            Paragraph::new(Line::from(Span::raw(rule)))
                .render(Rect::new(0, popup_h + 2, width, 1), buf);

            let strip = Line::from(vec![
                Span::raw(" ↑/↓ "),
                Span::raw("select  "),
                Span::raw("⏎ "),
                Span::raw("run  "),
                Span::raw("⇥ "),
                Span::raw("complete  "),
                Span::raw("Esc "),
                Span::raw("close"),
            ]);
            Paragraph::new(strip).render(Rect::new(0, popup_h + 3, width, 1), buf);
        })
        .unwrap();
    buffer_to_symbols(terminal.backend().buffer())
}

#[test]
fn slash_popup_empty_query() {
    let out = render_slash_popup_fx(80, 8, "", 0);
    assert_snapshot!("slash_popup_empty_query", out);
}

#[test]
fn slash_popup_filtered_to_help() {
    let out = render_slash_popup_fx(80, 8, "he", 0);
    assert_snapshot!("slash_popup_filtered_to_help", out);
}

#[test]
fn slash_popup_no_matches() {
    let out = render_slash_popup_fx(80, 8, "zzz", 0);
    assert_snapshot!("slash_popup_no_matches", out);
}

#[test]
fn slash_popup_cursor_at_third() {
    let out = render_slash_popup_fx(80, 8, "", 2);
    assert_snapshot!("slash_popup_cursor_at_third", out);
}

// ── X3: mention popup snapshots ──────────────────────────────────────────────

const MENTION_FX: &[&str] = &[
    "src/main.rs",
    "src/lib.rs",
    "src/inline_runtime.rs",
    "README.md",
    "Cargo.toml",
    "crates/jekko-tui/src/inline_runtime.rs",
];

fn mention_filtered(query: &str) -> Vec<&'static str> {
    let q = query.to_lowercase();
    if q.is_empty() {
        return MENTION_FX.to_vec();
    }
    MENTION_FX
        .iter()
        .copied()
        .filter(|p| p.to_lowercase().contains(&q))
        .collect()
}

fn split_dir_base_fx(path: &str) -> (String, String) {
    match path.rfind('/') {
        Some(i) => (path[..=i].to_string(), path[i + 1..].to_string()),
        None => (String::new(), path.to_string()),
    }
}

fn render_mention_popup_fx(width: u16, height: u16, query: &str, cursor: usize) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            let buf = frame.buffer_mut();
            let popup_h = height.saturating_sub(4);

            let filtered = mention_filtered(query);
            let title = Line::from(vec![
                Span::raw(" @ files  "),
                Span::raw(format!("({}/{})", filtered.len(), MENTION_FX.len())),
            ]);
            if popup_h > 0 {
                Paragraph::new(title).render(Rect::new(0, 0, width, 1), buf);
            }

            let body_rows = popup_h.saturating_sub(1) as usize;
            if popup_h > 0 {
                if filtered.is_empty() {
                    if body_rows > 0 {
                        let empty = Line::from(Span::raw("   (no matches)"));
                        Paragraph::new(empty).render(Rect::new(0, 1, width, 1), buf);
                    }
                } else if body_rows > 0 {
                    let cur = cursor.min(filtered.len() - 1);
                    let start = cur.saturating_sub(body_rows - 1);
                    let end = (start + body_rows).min(filtered.len());
                    for (offset, idx_pos) in (start..end).enumerate() {
                        let path = filtered[idx_pos];
                        let selected = idx_pos == cur;
                        let marker = if selected { " › " } else { "   " };
                        let (dir, base) = split_dir_base_fx(path);
                        let row =
                            Line::from(vec![Span::raw(marker), Span::raw(dir), Span::raw(base)]);
                        let y = 1 + offset as u16;
                        Paragraph::new(row).render(Rect::new(0, y, width, 1), buf);
                    }
                }
            }

            let rule = "─".repeat(width as usize);
            Paragraph::new(Line::from(Span::raw(rule.clone())))
                .render(Rect::new(0, popup_h, width, 1), buf);

            let composer = Line::from(vec![
                Span::raw("› "),
                Span::raw(format!("@{}", query)),
                Span::raw(" "),
            ]);
            Paragraph::new(composer).render(Rect::new(0, popup_h + 1, width, 1), buf);

            Paragraph::new(Line::from(Span::raw(rule)))
                .render(Rect::new(0, popup_h + 2, width, 1), buf);

            let strip = Line::from(vec![
                Span::raw(" ↑/↓ "),
                Span::raw("select  "),
                Span::raw("⏎ "),
                Span::raw("insert  "),
                Span::raw("⇥ "),
                Span::raw("complete  "),
                Span::raw("Esc "),
                Span::raw("close"),
            ]);
            Paragraph::new(strip).render(Rect::new(0, popup_h + 3, width, 1), buf);
        })
        .unwrap();
    buffer_to_symbols(terminal.backend().buffer())
}

#[test]
fn mention_popup_empty_query() {
    let out = render_mention_popup_fx(80, 8, "", 0);
    assert_snapshot!("mention_popup_empty_query", out);
}

#[test]
fn mention_popup_substring_filter() {
    let out = render_mention_popup_fx(80, 8, "inli", 0);
    assert_snapshot!("mention_popup_substring_filter", out);
}

#[test]
fn mention_popup_no_matches() {
    let out = render_mention_popup_fx(80, 8, "zzznope", 0);
    assert_snapshot!("mention_popup_no_matches", out);
}

#[test]
fn mention_popup_cursor_navigation() {
    let out = render_mention_popup_fx(80, 8, "", 1);
    assert_snapshot!("mention_popup_cursor_navigation", out);
}

// ── T1-V1b: live composer chevron snapshots ──────────────────────────────────
//
// These tests drive the exact `render_composer_row` helper the inline runtime
// uses, so a regression in the live path would re-bake these snapshots.
// Symbol-only snapshots (style stripped) match the rest of this file.

fn render_composer_row_buf(width: u16, height: u16, input: &str) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            let area = Rect::new(0, 0, width, height);
            render_composer_row(frame.buffer_mut(), area, input, false, None, None);
        })
        .unwrap();
    buffer_to_symbols(terminal.backend().buffer())
}

#[test]
fn live_composer_chevron_single_line_80x24() {
    // Single-row prompt area, 80 cols. Row 0 col 0 must be `›`, col 1 blank,
    // body starts at col 2. Trailing spaces are trimmed for readability.
    let out = render_composer_row_buf(80, 1, "what is the answer");
    assert_snapshot!("live_composer_chevron_single_line_80x24", out);
    // Belt-and-braces: prove the line starts with the prefix the const owns.
    let prefix = format!(
        "{}{}",
        PROMPT_GLYPH,
        " ".repeat(PROMPT_PREFIX_WIDTH as usize - 1)
    );
    assert!(
        out.starts_with(&prefix),
        "snapshot must lead with `{prefix}` (PROMPT_GLYPH + blank)"
    );
}

#[test]
fn live_composer_chevron_multi_line_80x24() {
    // Multi-row prompt area, mimicking what future multi-line composer growth
    // looks like: row 0 carries the `›`, continuation rows have a blank
    // gutter (verified in `inline_runtime` unit tests via direct cell reads —
    // insta strips trailing whitespace/empty rows on disk so the snapshot
    // collapses to just the visible body row). Body text comes from the LAST
    // line of `input` (current v1 collapses multi-line input to its last
    // line), so the snapshot here serves as a regression target on the
    // composer's leading `›` glyph + body offset when the area is tall.
    let out = render_composer_row_buf(80, 3, "first line\nsecond line\nthird line");
    assert_snapshot!("live_composer_chevron_multi_line_80x24", out);
    // Continuation rows must not carry a second chevron.
    let mut lines = out.lines();
    let _row0 = lines.next();
    for (i, line) in lines.enumerate() {
        assert!(
            !line.starts_with(PROMPT_GLYPH),
            "continuation row {} carried a stray chevron: {line:?}",
            i + 1
        );
    }
}

// ── T1-V2/V3/V4: bottom-chrome snapshots ─────────────────────────────────────
//
// These render the same primitive functions the inline runtime calls in
// production, stacked into the layout the runtime owns (`Layout::vertical`
// in `draw`). Each fixture covers one chrome configuration:
//
// - `bottom_chrome_full_80x24`: in-flight turn → working strip visible.
// - `bottom_chrome_idle_no_strip_80x24`: idle → working strip hidden.
// - `bottom_chrome_narrow_60x24` / `_40x24`: narrow widths drop status segs.
//
// The composer body shows just the `›` gutter (no input) so we focus on the
// chrome rows; agent rail is omitted (the rail layout is fully covered by
// `agents::panel` tests). The footer uses a stable model/cwd/branch fixture
// rather than reading env so the snapshot is reproducible across machines.

fn render_bottom_chrome_buf(
    width: u16,
    in_flight_elapsed: Option<Duration>,
    background_count: usize,
    focus_hint: &str,
    footer: &FooterInfo,
) -> String {
    // Layout:
    //   transcript filler (1 row blank)
    //   working_strip (0|1 row)
    //   permission_banner (1 row)
    //   composer (3 rows)
    //   footer (1 row)
    let working_h: u16 = if in_flight_elapsed.is_some() || background_count > 0 {
        1
    } else {
        0
    };
    let height: u16 = 1 + working_h + 1 + 3 + 1;
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            let buf = frame.buffer_mut();
            // Skip transcript row (intentionally blank — focus is on chrome).
            let mut y: u16 = 1;
            if working_h == 1 {
                // T-INLINE-WAVE3 (test unblock): sibling T-GLYPH-WAVE3 changed
                // `render_working_strip`'s last arg from `motion_enabled: bool`
                // to `cfg: Option<&UiConfig>`. `None` keeps the legacy
                // env/file fallback inside `anim::motion_enabled_with_cfg`,
                // which is fine for snapshot golden generation.
                render_working_strip(
                    buf,
                    Rect::new(0, y, width, 1),
                    in_flight_elapsed,
                    background_count,
                    false,
                    None,
                );
                y += 1;
            }
            render_permission_banner(
                buf,
                Rect::new(0, y, width, 1),
                "bypass permissions",
                2,
                focus_hint,
                false,
            );
            y += 1;
            // Composer 3-row shell rendered as just the prompt row at y+1 so
            // we don't depend on the full ComposerShell here. The snapshot
            // pins the chrome layout, not the composer specifics (which has
            // its own snapshot above).
            render_composer_row(buf, Rect::new(0, y + 1, width, 1), "", false, None, None);
            y += 3;
            render_footer_status(buf, Rect::new(0, y, width, 1), footer, false);
        })
        .unwrap();
    buffer_to_symbols(terminal.backend().buffer())
}

fn footer_fx() -> FooterInfo {
    FooterInfo {
        model: "claude-opus-4-7".into(),
        effort: "high".into(),
        cwd: "~/code/jekko".into(),
        branch: Some("main".into()),
        profile: None,
        jnoccio: None,
    }
}

#[test]
fn bottom_chrome_full_80x24() {
    // In-flight turn → working strip visible at row 1, permission banner row
    // 2, composer prompt row 4, footer row 5. Total height = 6 rows.
    let out = render_bottom_chrome_buf(
        80,
        Some(Duration::from_secs(65)),
        0,
        HINT_CHAT_FOCUS,
        &footer_fx(),
    );
    assert_snapshot!("bottom_chrome_full_80x24", out);
    assert!(
        out.contains("Working"),
        "expected Working strip in 80-col snapshot, got: {out}"
    );
    assert!(out.contains("bypass permissions"));
    assert!(out.contains("claude-opus-4-7"));
}

#[test]
fn bottom_chrome_idle_no_strip_80x24() {
    // Idle → working strip hidden, only banner + composer + footer.
    let out = render_bottom_chrome_buf(80, None, 0, HINT_CHAT_FOCUS, &footer_fx());
    assert_snapshot!("bottom_chrome_idle_no_strip_80x24", out);
    assert!(
        !out.contains("Working"),
        "working strip must be hidden when idle, got: {out}"
    );
    assert!(out.contains("bypass permissions"));
    assert!(out.contains("claude-opus-4-7"));
}

#[test]
fn bottom_chrome_narrow_60x24() {
    // Medium width: banner drops the hint, footer drops profile bracket.
    let out = render_bottom_chrome_buf(
        60,
        Some(Duration::from_secs(5)),
        0,
        HINT_CHAT_FOCUS,
        &footer_fx(),
    );
    assert_snapshot!("bottom_chrome_narrow_60x24", out);
}

#[test]
fn bottom_chrome_narrow_40x24() {
    // Tight width: aggressive truncation across all three chrome rows.
    let out = render_bottom_chrome_buf(
        40,
        Some(Duration::from_secs(5)),
        1,
        HINT_AGENT_PANEL_FOCUS,
        &footer_fx(),
    );
    assert_snapshot!("bottom_chrome_narrow_40x24", out);
}
