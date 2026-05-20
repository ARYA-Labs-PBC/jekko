//! Footer status row (COWBOY.md T1-V4, per `tips/fucktui/tip6.txt` §1045-1049).
//!
//! Bottom row beneath the multi-agent rail:
//!
//! ```text
//! {model} {effort} · {cwd} · {branch} [{profile}]
//! ```
//!
//! Per-segment colour:
//! - `model`   → [`codex::YELLOW`]
//! - `effort`  → [`codex::FG_DIM`]
//! - `cwd`     → [`codex::GREEN_OK`]
//! - `branch`  → [`codex::CYAN_TAB`]
//! - `profile` → bracketed in [`codex::FG_DIM`]
//!
//! Layout notes:
//! - The model and the effort share a space (`{model} {effort}`) so they read
//!   as a single block; treated as a single anchor segment in the packer.
//! - The branch segment is dropped entirely when `info.branch` is `None`
//!   (no dangling ` · `).
//! - The profile segment is dropped entirely when `info.profile` is `None`.
//! - Truncation priority for narrow widths: model=0 (anchor) → cwd=1 →
//!   effort=2 → branch=3 → profile=4 (drops first).
//!
//! The cwd should already be `~`-relative when passed in — this module does
//! not do the rewrite (the caller has the `$HOME` it cares about). See
//! [`crate::components::boot_inline::BootContext`] for the helper that does.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Paragraph, Widget};

use crate::glyph_set;
use crate::layout::status_pack::{pack, PackOptions, Segment};
use crate::theme::codex;

/// Inputs for the footer row.
#[derive(Clone, Debug)]
pub struct FooterInfo {
    /// Model label, e.g. `"claude-opus-4-7"`.
    pub model: String,
    /// Effort tier, e.g. `"high"` / `"medium"`. Empty string → segment hidden.
    pub effort: String,
    /// Working directory, `~`-relative when applicable.
    pub cwd: String,
    /// Current git branch, or `None` for `(no git)`.
    pub branch: Option<String>,
    /// Configured profile name, or `None` for unsourced.
    pub profile: Option<String>,
    /// Jnoccio boot status snapshot, when available.
    pub jnoccio: Option<String>,
}

/// Render the footer status row into `area`.
///
/// `pack_single_line` — T-COMPONENT-PLUMBING: when `true`, route the
/// segments through [`layout::status_pack::pack`] using a sharper drop chain
/// so even narrower widths collapse to a single packed line that still keeps
/// the model anchor visible. Default `false` preserves the historical
/// priorities so existing call sites do not change behaviour.
pub fn render_footer_status(
    buf: &mut Buffer,
    area: Rect,
    info: &FooterInfo,
    pack_single_line: bool,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let model_style = Style::default().fg(codex::YELLOW);
    let dim = Style::default().fg(codex::FG_DIM);
    let cwd_style = Style::default().fg(codex::GREEN_OK);
    let branch_style = Style::default().fg(codex::CYAN_TAB);

    // Anchor: `{model}` always, `{model} {effort}` when effort is present.
    // Effort sharing the model anchor span keeps the colour split visible in
    // a single Line — but the packer treats it as a single segment for drop
    // accounting. WHY this departure from "effort=2 drops second-to-last":
    // the spec writes them inseparable. If effort needs to drop ahead of
    // cwd in narrow widths, the packer keeps the model anchor and would
    // simply render the model alone (the model gets effort suffix only when
    // both fit together). The model+effort string here is computed up-front
    // so the priority tier `model=0` anchors the row even at 10 cols.
    //
    // We split out effort as a separate Segment with `priority=2` so that
    // when intermediate widths can't fit model+effort+cwd, the packer drops
    // effort (priority=2, drops before branch=3) ahead of cwd (priority=1).
    //
    // T-COMPONENT-PLUMBING: in `pack_single_line` mode we bias priorities
    // sharper (effort/branch/profile shed earlier) so even tiny widths still
    // produce a packed line with the model + cwd visible.
    let (p_effort, p_cwd, p_branch, p_profile) = if pack_single_line {
        (3u8, 1u8, 4u8, 5u8)
    } else {
        (2u8, 1u8, 3u8, 4u8)
    };
    let mut segments: Vec<Segment> = Vec::with_capacity(5);
    segments.push(Segment::new(info.model.clone(), model_style, 0));
    if !info.effort.is_empty() {
        segments.push(Segment::new(info.effort.clone(), dim, p_effort));
    }
    segments.push(Segment::new(info.cwd.clone(), cwd_style, p_cwd));
    if let Some(branch) = &info.branch {
        segments.push(Segment::new(branch.clone(), branch_style, p_branch));
    }
    if let Some(profile) = &info.profile {
        segments.push(Segment::new(format!("[{profile}]"), dim, p_profile));
    }
    if let Some(jnoccio) = &info.jnoccio {
        segments.push(Segment::new(jnoccio.clone(), dim, 1));
    }

    // T-GLYPH-WAVE3: the ` · ` separator now defers to GlyphMode via
    // `separator_dot` (`-` in ASCII), in addition to the ellipsis honoring
    // GlyphMode for packed-truncation. `PackOptions::separator` wants a
    // `&'static str`, but `glyph_set::current().separator_dot` is already
    // `&'static str` — so we cache the active glyph set up-front, build the
    // separator literal via concat, and hand the borrowed slice through.
    let g = glyph_set::current();
    let separator: &'static str = if g.separator_dot == "·" {
        " · "
    } else {
        " - "
    };
    let opts = PackOptions {
        separator,
        separator_style: Style::default().fg(codex::FG_VERY_DIM),
        ellipsis: g.ellipsis,
    };
    let spans = pack(&segments, area.width, &opts);
    if spans.is_empty() {
        return;
    }
    let row = Rect::new(area.x, area.y, area.width, 1);
    Paragraph::new(Line::from(spans)).render(row, buf);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use unicode_width::UnicodeWidthStr;

    fn sample() -> FooterInfo {
        FooterInfo {
            model: "claude-opus-4-7".into(),
            effort: "high".into(),
            cwd: "~/code/jekko".into(),
            branch: Some("main".into()),
            profile: Some("dev".into()),
            jnoccio: Some("jnoccio checking".into()),
        }
    }

    fn render(width: u16, info: &FooterInfo) -> String {
        render_with_pack(width, info, false)
    }

    fn render_with_pack(width: u16, info: &FooterInfo, pack_single_line: bool) -> String {
        let backend = TestBackend::new(width, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                render_footer_status(
                    frame.buffer_mut(),
                    Rect::new(0, 0, width, 1),
                    info,
                    pack_single_line,
                );
            })
            .unwrap();
        let buf = terminal.backend().buffer();
        let mut out = String::new();
        for x in 0..width {
            out.push_str(buf[(x, 0)].symbol());
        }
        out.trim_end().to_string()
    }

    #[test]
    fn happy_path_renders_all_segments() {
        let out = render(120, &sample());
        assert!(out.contains("claude-opus-4-7"), "model: {out:?}");
        assert!(out.contains("high"), "effort: {out:?}");
        assert!(out.contains("~/code/jekko"), "cwd: {out:?}");
        assert!(out.contains("main"), "branch: {out:?}");
        assert!(out.contains("[dev]"), "profile: {out:?}");
        assert!(out.contains("jnoccio checking"), "boot status: {out:?}");
    }

    #[test]
    fn missing_branch_drops_segment_no_dangling_separator() {
        let info = FooterInfo {
            branch: None,
            ..sample()
        };
        let out = render(120, &info);
        assert!(out.contains("claude-opus-4-7"));
        assert!(out.contains("~/code/jekko"));
        assert!(out.contains("[dev]"));
        assert!(out.contains("jnoccio checking"));
        // The bare word "main" must not appear, and we should not end with " ·".
        assert!(!out.contains("main"), "branch should be dropped: {out:?}");
        assert!(
            !out.trim_end().ends_with('·'),
            "no trailing separator: {out:?}"
        );
    }

    #[test]
    fn missing_profile_drops_segment() {
        let info = FooterInfo {
            profile: None,
            ..sample()
        };
        let out = render(120, &info);
        assert!(!out.contains("[dev]"), "profile dropped: {out:?}");
    }

    #[test]
    fn empty_effort_drops_segment() {
        let info = FooterInfo {
            effort: String::new(),
            ..sample()
        };
        let out = render(120, &info);
        assert!(!out.contains("high"));
        assert!(out.contains("claude-opus-4-7"));
    }

    #[test]
    fn narrow_width_drops_profile_first() {
        // Drop priority order (highest first) = profile (4) → branch (3) →
        // effort (2) → cwd (1) → model (0, anchor).
        let info = sample();
        let out = render(40, &info);
        assert!(out.contains("claude-opus-4-7"), "model preserved: {out:?}");
        assert!(!out.contains("[dev]"), "profile dropped first: {out:?}");
    }

    #[test]
    fn very_narrow_width_keeps_only_model_anchor() {
        let out = render(15, &sample());
        assert!(out.contains("claude-opus-4-7"), "model anchor: {out:?}");
        assert!(!out.contains("[dev]"));
        assert!(!out.contains("~/code/jekko"));
    }

    #[test]
    fn zero_area_paints_nothing() {
        let backend = TestBackend::new(80, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                render_footer_status(frame.buffer_mut(), Rect::new(0, 0, 0, 0), &sample(), false);
            })
            .unwrap();
        let buf = terminal.backend().buffer();
        for x in 0..80 {
            assert_eq!(buf[(x, 0)].symbol(), " ", "cell at x={x}");
        }
    }

    #[test]
    fn rendered_width_never_exceeds_terminal_width() {
        for w in [15u16, 20, 40, 60, 80, 120] {
            let out = render(w, &sample());
            assert!(
                UnicodeWidthStr::width(out.as_str()) <= w as usize,
                "width {w}: rendered {} cells, text={out:?}",
                UnicodeWidthStr::width(out.as_str())
            );
        }
    }

    #[test]
    fn footer_status_single_line_packs_under_narrow_width() {
        // T-COMPONENT-PLUMBING: with `pack_single_line == true`, the row at
        // 40 cols still fits on a single line and keeps the model anchor.
        // The sharper priority chain sheds profile (5) → branch (4) → effort
        // (3) before touching cwd (1) or model (0).
        let out = render_with_pack(40, &sample(), true);
        assert!(
            UnicodeWidthStr::width(out.as_str()) <= 40,
            "packed row must fit single line at 40 cols, got width {} text={out:?}",
            UnicodeWidthStr::width(out.as_str())
        );
        assert!(
            out.contains("claude-opus-4-7"),
            "packed mode: model anchor preserved at 40 cols, got {out:?}"
        );
        assert!(
            !out.contains("[dev]"),
            "packed mode: profile drops first at 40 cols, got {out:?}"
        );
    }
}
