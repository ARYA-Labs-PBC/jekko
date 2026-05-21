//! Inline boot loader (COWBOY.md B1).
//!
//! Renders the one-shot "JEKKO is starting" block that gets pushed into the
//! user's terminal scrollback at startup, plus a steady-state `⚡ JEKKO …`
//! header line.
//!
//! The full-screen splash with the pixel wordmark lives in
//! `components/splash` and is reserved for the legacy alt-screen path. The
//! inline mode keeps the boot block short (4 lines) so the user's first
//! scrollback impression is brand + version + workspace, not a graphic.

use std::env;
use std::path::Path;
use std::process::Command;

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::glyph_set;
use crate::theme::{
    codex_blue_path, codex_cyan_tab, codex_fg_dim, codex_fg_strong, codex_orange_agent,
};
use crate::transcript::inline_cards::render_session_header;

/// What we know about the workspace at boot time. Cheap to compute — read
/// once in main and pass into the render functions.
#[derive(Clone, Debug)]
pub struct BootContext {
    pub version: String,
    /// User-friendly cwd display (e.g. `~/code/jekko`). Falls back to the
    /// full path if `$HOME` does not match.
    pub cwd_display: String,
    /// Git branch name, if the cwd is inside a repo and `git` is on PATH.
    pub branch: Option<String>,
}

impl BootContext {
    /// Build context from the environment. Never returns Err — missing data
    /// degrades gracefully.
    pub fn detect() -> Self {
        let version = env::var("JEKKO_VERSION_OVERRIDE")
            .unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string());
        let cwd_display = current_cwd_display();
        let branch = current_git_branch();
        Self {
            version,
            cwd_display,
            branch,
        }
    }
}

/// 5-line Claude-style boot block. Renders at the top of the alt-screen
/// surface on session start, then becomes part of the scrollback.
///
/// ```text
///  ✻ Welcome to JEKKO
///
///    /help for shortcuts, /jankurai to audit, /status for setup
///
///    cwd: ~/code/jekko · branch: main · v0.1.0
/// ```
pub fn render_inline_boot_block(ctx: &BootContext, _term_width: u16) -> Vec<Line<'static>> {
    // T-GLYPH-WAVE3: welcome marker defers to GlyphMode (`*` in ASCII).
    let welcome = Line::from(vec![
        Span::styled(
            format!(" {} ", glyph_set::current().welcome_marker),
            Style::default().fg(codex_orange_agent()),
        ),
        Span::styled(
            "Welcome to JEKKO",
            Style::default()
                .fg(codex_fg_strong())
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    let blank = Line::from(Span::raw(""));

    let hint = Line::from(vec![
        Span::raw("   "),
        Span::styled("/help", Style::default().fg(codex_cyan_tab())),
        Span::styled(" for shortcuts, ", Style::default().fg(codex_fg_dim())),
        Span::styled("/jankurai", Style::default().fg(codex_cyan_tab())),
        Span::styled(" to audit, ", Style::default().fg(codex_fg_dim())),
        Span::styled("/status", Style::default().fg(codex_cyan_tab())),
        Span::styled(" for setup", Style::default().fg(codex_fg_dim())),
    ]);

    let mut ctx_spans = vec![
        Span::raw("   "),
        Span::styled("cwd: ", Style::default().fg(codex_fg_dim())),
        Span::styled(
            ctx.cwd_display.clone(),
            Style::default().fg(codex_blue_path()),
        ),
    ];
    if let Some(branch) = &ctx.branch {
        ctx_spans.push(Span::styled(
            " · branch: ",
            Style::default().fg(codex_fg_dim()),
        ));
        ctx_spans.push(Span::styled(
            branch.clone(),
            Style::default().fg(codex_cyan_tab()),
        ));
    }
    ctx_spans.push(Span::styled(" · v", Style::default().fg(codex_fg_dim())));
    ctx_spans.push(Span::styled(
        ctx.version.clone(),
        Style::default().fg(codex_fg_dim()),
    ));

    vec![welcome, blank.clone(), hint, blank, Line::from(ctx_spans)]
}

/// Compact 1-line scrollback header that we push between sessions (e.g. after
/// `/new`) or as the persistent reminder of "what process is this".
pub fn render_inline_session_marker(ctx: &BootContext) -> Vec<Line<'static>> {
    render_session_header(&ctx.version, &ctx.cwd_display, ctx.branch.as_deref())
}

// ── helpers ─────────────────────────────────────────────────────────────────

fn current_cwd_display() -> String {
    let cwd = env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
    if let Some(home) = env::var_os("HOME") {
        let home_path = Path::new(&home);
        if let Ok(rel) = cwd.strip_prefix(home_path) {
            if rel.as_os_str().is_empty() {
                return "~".to_string();
            }
            return format!("~/{}", rel.display());
        }
    }
    cwd.display().to_string()
}

fn current_git_branch() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let s = String::from_utf8(output.stdout).ok()?;
    let trimmed = s.trim().to_string();
    if trimmed.is_empty() || trimmed == "HEAD" {
        None
    } else {
        Some(trimmed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boot_block_is_five_lines() {
        let ctx = BootContext {
            version: "0.1.0".into(),
            cwd_display: "~/code/jekko".into(),
            branch: Some("main".into()),
        };
        let lines = render_inline_boot_block(&ctx, 80);
        assert_eq!(lines.len(), 5);
    }

    #[test]
    fn boot_block_first_line_welcomes_jekko() {
        let ctx = BootContext {
            version: "0.1.0".into(),
            cwd_display: "~".into(),
            branch: None,
        };
        let lines = render_inline_boot_block(&ctx, 80);
        let first: String = lines[0]
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect::<Vec<_>>()
            .join("");
        assert!(first.contains("Welcome to JEKKO"));
        assert!(first.contains("✻"));
    }

    #[test]
    fn boot_block_cwd_line_contains_cwd_branch_version() {
        let ctx = BootContext {
            version: "9.9.9".into(),
            cwd_display: "~/x".into(),
            branch: Some("main".into()),
        };
        let lines = render_inline_boot_block(&ctx, 80);
        let cwd_line: String = lines[4]
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect::<Vec<_>>()
            .join("");
        assert!(cwd_line.contains("~/x"));
        assert!(cwd_line.contains("main"));
        assert!(cwd_line.contains("9.9.9"));
    }

    #[test]
    fn detect_does_not_panic() {
        let _ = BootContext::detect();
    }
}
