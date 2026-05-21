// Inline-viewport transcript cards (COWBOY.md T1).
//
// Each card kind has a single render function that returns `Vec<Line<'static>>`
// ready to push into the terminal's native scrollback via
// `Terminal::insert_before`. No boxed borders — just a single `│` left margin
// and the Claude/Codex visual grammar (action bullets, dim metadata, dim tree
// prefixes, collapsed-line markers).
//
// Card kinds:
//   - `user` — user prompt
//   - `assistant` — streamed assistant text
//   - `reasoning` — extended thinking, italic, very dim
//   - `tool_call` — `● Bash(cmd)` header + indented output + collapsed marker
//   - `diff` — green `+` / red `-` lines with bg shading
//   - `system_notice` — info/warning/error chrome
//   - `collapsed_block` — `… +N lines (ctrl+o to expand)`
//
// These functions are pure: state in → lines out. They never touch the
// terminal or buffer directly. The caller decides where to render.

use std::str::FromStr;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};
use syntect::easy::HighlightLines;
use syntect::highlighting::{
    Color as SynColor, FontStyle, ScopeSelectors, StyleModifier, Theme, ThemeItem, ThemeSettings,
};
use syntect::parsing::SyntaxSet;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::anim::{
    elapsed_label, oscillate_codex_default_with_motion, pulse_glyph_with_motion, rotating_verb,
    THINKING_VERBS,
};
use crate::glyph_set;
use crate::theme::{
    codex_blue_path, codex_cyan_tab, codex_diff_add_bg, codex_diff_del_bg, codex_fg, codex_fg_dim,
    codex_fg_strong, codex_fg_very_dim, codex_green_ok, codex_orange_agent, codex_salmon_fail,
    codex_yellow,
};

/// Default rotation period for the thinking verbs (tip3.txt §18.2).
const THINKING_VERB_PERIOD: Duration = Duration::from_secs(4);

/// Returns `true` when `verb` carries no specific phase name, so the live
/// renderer should substitute a rotating thinking verb. Anything outside this
/// list (Bash, Read, Edit, Search, …) is treated as an explicit phase and
/// passes through untouched.
fn verb_is_generic(verb: &str) -> bool {
    matches!(
        verb.trim(),
        "" | "Working" | "working" | "running" | "Running" | "tool" | "Tool"
    )
}

/// Status of an action represented by a tool-call card.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionStatus {
    Pending,
    Running,
    Success,
    Failure,
    Cancelled,
}

impl ActionStatus {
    fn bullet(self) -> Span<'static> {
        self.bullet_at(Duration::ZERO, true)
    }

    fn bullet_at(self, elapsed: Duration, motion_enabled: bool) -> Span<'static> {
        // T-GLYPH-WAVE2: route the static bullets through the active GlyphMode
        // so `JEKKO_ASCII=1` swaps `●`/`○` for `*`/`o`. The Running variant
        // still goes through `pulse_glyph`, which owns its own animation set
        // (anim.rs is excluded from this migration wave).
        let g = glyph_set::current();
        match self {
            ActionStatus::Success => {
                Span::styled(g.bullet_success, Style::default().fg(codex_green_ok()))
            }
            ActionStatus::Failure => {
                Span::styled(g.bullet_success, Style::default().fg(codex_salmon_fail()))
            }
            ActionStatus::Cancelled => {
                Span::styled(g.bullet_success, Style::default().fg(codex_fg_very_dim()))
            }
            ActionStatus::Running => Span::styled(
                pulse_glyph_with_motion(elapsed, motion_enabled),
                Style::default().fg(codex_orange_agent()),
            ),
            ActionStatus::Pending => {
                Span::styled(g.bullet_pending, Style::default().fg(codex_fg_dim()))
            }
        }
    }
}

/// System notice severity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoticeKind {
    Info,
    Warn,
    Error,
}

impl NoticeKind {
    fn color(self) -> Color {
        match self {
            NoticeKind::Info => codex_cyan_tab(),
            NoticeKind::Warn => codex_yellow(),
            NoticeKind::Error => codex_salmon_fail(),
        }
    }
    fn label(self) -> &'static str {
        match self {
            NoticeKind::Info => "info",
            NoticeKind::Warn => "warn",
            NoticeKind::Error => "error",
        }
    }
}

// ── Cards ────────────────────────────────────────────────────────────────────

/// Render a user prompt card. Dim `│` margin, no label flair — the prompt
/// itself carries the message.
pub fn render_user(text: &str) -> Vec<Line<'static>> {
    let mut out = Vec::new();
    // T-GLYPH-WAVE2: the `│` margin defers to GlyphMode (`|` in ASCII).
    let margin = format!("{} ", glyph_set::current().card_margin);
    let prefix = Span::styled(margin, Style::default().fg(codex_fg_dim()));
    for body in text.lines() {
        out.push(Line::from(vec![
            prefix.clone(),
            Span::styled(body.to_string(), Style::default().fg(codex_fg_strong())),
        ]));
    }
    if out.is_empty() {
        out.push(Line::from(vec![prefix]));
    }
    out
}

/// Render an assistant card. Orange `│` margin. Text rendered at normal weight.
/// Callers stream into this and re-render the in-flight card on each delta;
/// once the turn ends, the final lines flow into scrollback via `insert_before`.
pub fn render_assistant(text: &str) -> Vec<Line<'static>> {
    let mut out = Vec::new();
    // T-GLYPH-WAVE2: card margin glyph via GlyphMode.
    let margin = format!("{} ", glyph_set::current().card_margin);
    let prefix = Span::styled(margin, Style::default().fg(codex_orange_agent()));
    if text.is_empty() {
        out.push(Line::from(vec![prefix]));
        return out;
    }
    for body in text.lines() {
        out.push(Line::from(vec![
            prefix.clone(),
            Span::styled(body.to_string(), Style::default().fg(codex_fg())),
        ]));
    }
    out
}

/// Render a reasoning/thinking card. Very-dim margin, italic body.
pub fn render_reasoning(text: &str) -> Vec<Line<'static>> {
    let mut out = Vec::new();
    // T-GLYPH-WAVE2: card margin glyph via GlyphMode.
    let margin = format!("{} ", glyph_set::current().card_margin);
    let prefix = Span::styled(margin, Style::default().fg(codex_fg_very_dim()));
    for body in text.lines() {
        out.push(Line::from(vec![
            prefix.clone(),
            Span::styled(
                body.to_string(),
                Style::default()
                    .fg(codex_fg_very_dim())
                    .add_modifier(Modifier::ITALIC),
            ),
        ]));
    }
    if out.is_empty() {
        out.push(Line::from(vec![prefix]));
    }
    out
}

/// Tool-call card input. `verb` is the action label (Bash, Read, Edit, Search,
/// Web, etc.); `args` is what comes inside the parens. Output lines are the
/// tool's collected stdout/stderr, already line-split and trimmed.
pub struct ToolCall<'a> {
    pub verb: &'a str,
    pub args: &'a str,
    pub status: ActionStatus,
    pub output: &'a [String],
    /// Show only the last `max_output_lines` lines; the rest collapse into a
    /// `… +N lines (ctrl+o to expand)` marker. Set to `usize::MAX` to never
    /// collapse.
    pub max_output_lines: usize,
}

/// Render a tool-call card:
///
/// ```text
/// ● Bash(git status --short)
///   └ M  crates/jekko-tui/src/lifecycle.rs
///     M  crates/jekko-tui/src/theme.rs
///     … +12 lines (ctrl+o to expand)
/// ```
pub fn render_tool_call(call: &ToolCall<'_>) -> Vec<Line<'static>> {
    render_tool_call_with(
        call,
        call.status.bullet(),
        Style::default()
            .fg(codex_fg_strong())
            .add_modifier(Modifier::BOLD),
        None,
    )
}

/// Render a live running tool-call card with time-derived motion.
///
/// `started_at` and `now` are passed in by the runtime so the renderer stays
/// pure and testable. Running cards use a pulsing bullet, oscillating verb
/// color, and a 1Hz elapsed timer in the header.
pub fn render_tool_call_live(
    call: &ToolCall<'_>,
    started_at: Instant,
    now: Instant,
    motion_enabled: bool,
) -> Vec<Line<'static>> {
    let elapsed = now.saturating_duration_since(started_at);
    let verb_color = oscillate_codex_default_with_motion(elapsed, motion_enabled);
    // T1-V7: rotate a thinking verb when no specific phase name is provided,
    // otherwise pass the explicit phase through unchanged.
    let rotated;
    let effective_call = if verb_is_generic(call.verb) {
        rotated = ToolCall {
            verb: rotating_verb(elapsed, THINKING_VERBS, THINKING_VERB_PERIOD),
            args: call.args,
            status: call.status,
            output: call.output,
            max_output_lines: call.max_output_lines,
        };
        &rotated
    } else {
        call
    };
    render_tool_call_with(
        effective_call,
        effective_call.status.bullet_at(elapsed, motion_enabled),
        Style::default().fg(verb_color).add_modifier(Modifier::BOLD),
        Some(elapsed_label(elapsed)),
    )
}

fn render_tool_call_with(
    call: &ToolCall<'_>,
    bullet: Span<'static>,
    verb_style: Style,
    elapsed: Option<String>,
) -> Vec<Line<'static>> {
    let mut out = Vec::new();
    let mut header_spans = vec![
        bullet,
        Span::raw(" "),
        Span::styled(call.verb.to_string(), verb_style),
        Span::styled("(", Style::default().fg(codex_fg_dim())),
        Span::styled(call.args.to_string(), Style::default().fg(codex_fg())),
        Span::styled(")", Style::default().fg(codex_fg_dim())),
    ];
    if let Some(elapsed) = elapsed {
        // T-GLYPH-WAVE3: separator dot defers to GlyphMode (`-` in ASCII) so
        // the inline-card header degrades cleanly without Unicode middle-dot.
        header_spans.push(Span::styled(
            format!(" {} ", glyph_set::current().separator_dot),
            Style::default().fg(codex_fg_very_dim()),
        ));
        header_spans.push(Span::styled(elapsed, Style::default().fg(codex_fg_dim())));
    }
    out.push(Line::from(header_spans));

    if call.output.is_empty() {
        return out;
    }

    let total = call.output.len();
    let visible = total.min(call.max_output_lines.max(1));
    let hidden = total.saturating_sub(visible);
    let start = total.saturating_sub(visible);

    // T-GLYPH-WAVE2: tree branch glyph defers to GlyphMode (`\` in ASCII).
    let branch_glyph = glyph_set::current().tree_branch;
    let first_glyph = format!("  {branch_glyph} ");
    for (idx, line) in call.output[start..].iter().enumerate() {
        let is_first = idx == 0;
        let glyph = if is_first {
            first_glyph.clone()
        } else {
            "    ".to_string()
        };
        out.push(Line::from(vec![
            Span::styled(glyph, Style::default().fg(codex_fg_dim())),
            Span::styled(line.clone(), Style::default().fg(codex_fg())),
        ]));
    }

    if hidden > 0 {
        out.push(collapsed_marker(hidden, "ctrl+o to expand"));
    }

    out
}

/// Render a `… +N lines (hint)` collapsed marker, aligned under the tree
/// indent used by `render_tool_call`.
pub fn collapsed_marker(hidden: usize, hint: &str) -> Line<'static> {
    // T-GLYPH-WAVE2: ellipsis defers to GlyphMode (`...` in ASCII).
    Line::from(vec![
        Span::styled("    ", Style::default().fg(codex_fg_dim())),
        Span::styled(
            glyph_set::current().ellipsis,
            Style::default().fg(codex_fg_dim()),
        ),
        Span::styled(
            format!(" +{hidden} lines ({hint})"),
            Style::default().fg(codex_fg_very_dim()),
        ),
    ])
}

/// Diff line kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiffLineKind {
    Context,
    Added,
    Removed,
}

/// A single diff hunk line.
pub struct DiffLine<'a> {
    pub kind: DiffLineKind,
    pub old_lineno: Option<usize>,
    pub new_lineno: Option<usize>,
    pub text: &'a str,
}

/// Minimum width per gutter number column. Matches Codex / `git --column` so
/// short hunks don't visibly jitter as numbers grow.
const DIFF_GUTTER_MIN: usize = 3;

/// Compute the right-padded width for the old and new line-number columns.
/// Each width is the max stringified lineno seen in the hunk, floored at
/// [`DIFF_GUTTER_MIN`].
fn diff_gutter_widths(hunks: &[DiffLine<'_>]) -> (usize, usize) {
    let old_w = hunks
        .iter()
        .filter_map(|h| h.old_lineno)
        .map(|n| n.to_string().len())
        .max()
        .unwrap_or(0)
        .max(DIFF_GUTTER_MIN);
    let new_w = hunks
        .iter()
        .filter_map(|h| h.new_lineno)
        .map(|n| n.to_string().len())
        .max()
        .unwrap_or(0)
        .max(DIFF_GUTTER_MIN);
    (old_w, new_w)
}

/// Right-pad a lineno to `w` columns, or emit `w` blanks if `None`.
fn fmt_lineno(n: Option<usize>, w: usize) -> String {
    match n {
        Some(v) => format!("{v:>w$}", w = w),
        None => " ".repeat(w),
    }
}

/// Sigil + sigil fg color for a `DiffLineKind`.
fn diff_sigil(kind: DiffLineKind) -> (&'static str, Color) {
    match kind {
        DiffLineKind::Context => (" ", codex_fg_dim()),
        DiffLineKind::Added => ("+", codex_green_ok()),
        DiffLineKind::Removed => ("-", codex_salmon_fail()),
    }
}

/// Background color for a diff row, if any.
fn diff_row_bg(kind: DiffLineKind) -> Option<Color> {
    match kind {
        DiffLineKind::Added => Some(codex_diff_add_bg()),
        DiffLineKind::Removed => Some(codex_diff_del_bg()),
        DiffLineKind::Context => None,
    }
}

/// Build the gutter span sequence for a single diff row.
///
/// Column layout, left-to-right:
///   `[indent "  "][old (old_w)][" "][new (new_w)][" "][sigil][" "]`
///
/// Numbers are dim; sigil colored per kind; everything carries `row_bg` so the
/// shaded rows look uniform.
fn diff_row_gutter(
    hunk: &DiffLine<'_>,
    old_w: usize,
    new_w: usize,
    row_bg: Option<Color>,
) -> Vec<Span<'static>> {
    let (sigil, sigil_fg) = diff_sigil(hunk.kind);
    let old_str = match hunk.kind {
        DiffLineKind::Added => " ".repeat(old_w),
        _ => fmt_lineno(hunk.old_lineno, old_w),
    };
    let new_str = match hunk.kind {
        DiffLineKind::Removed => " ".repeat(new_w),
        _ => fmt_lineno(hunk.new_lineno, new_w),
    };
    let with_bg = |s: Style| match row_bg {
        Some(bg) => s.bg(bg),
        None => s,
    };
    let lineno_style = with_bg(Style::default().fg(codex_fg_dim()));
    let sigil_style = with_bg(Style::default().fg(sigil_fg));
    vec![
        Span::styled("  ", lineno_style),
        Span::styled(old_str, lineno_style),
        Span::styled(" ", lineno_style),
        Span::styled(new_str, lineno_style),
        Span::styled(" ", lineno_style),
        Span::styled(sigil.to_string(), sigil_style),
        Span::styled(" ", sigil_style),
    ]
}

/// Render a file-update / diff card.
///
/// ```text
/// ● Edit(crates/jekko-tui/src/theme.rs)
///   486 486   db,
///       487 + max_connections: self.max_connections,
///   487 488   })
/// ```
pub fn render_diff(path: &str, hunks: &[DiffLine<'_>]) -> Vec<Line<'static>> {
    let mut out = Vec::new();

    out.push(Line::from(vec![
        ActionStatus::Success.bullet(),
        Span::raw(" "),
        Span::styled(
            "Edit",
            Style::default()
                .fg(codex_fg_strong())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("(", Style::default().fg(codex_fg_dim())),
        Span::styled(path.to_string(), Style::default().fg(codex_blue_path())),
        Span::styled(")", Style::default().fg(codex_fg_dim())),
    ]));

    let (old_w, new_w) = diff_gutter_widths(hunks);

    for hunk in hunks {
        let row_bg = diff_row_bg(hunk.kind);
        let body_style = match row_bg {
            Some(bg) => Style::default().fg(codex_fg_strong()).bg(bg),
            None => Style::default().fg(codex_fg()),
        };
        let mut spans = diff_row_gutter(hunk, old_w, new_w, row_bg);
        spans.push(Span::styled(hunk.text.to_string(), body_style));
        out.push(Line::from(spans));
    }

    out
}

/// Render a diff card directly into a buffer, painting full-row backgrounds
/// for added/removed lines BEFORE laying styled spans on top. Required so the
/// green/red shading extends to the right edge instead of stopping at the end
/// of the line content (the ragged-patch bug flagged in the 2026-05-17 memo).
///
/// Body text is syntax-highlighted via syntect when the file extension is
/// recognized; unknown extensions and lines over 512 chars fall back to the
/// plain fg styling used by `render_diff`.
pub fn render_diff_into(buf: &mut Buffer, area: Rect, path: &str, hunks: &[DiffLine<'_>]) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let header = Line::from(vec![
        ActionStatus::Success.bullet(),
        Span::raw(" "),
        Span::styled(
            "Edit",
            Style::default()
                .fg(codex_fg_strong())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("(", Style::default().fg(codex_fg_dim())),
        Span::styled(path.to_string(), Style::default().fg(codex_blue_path())),
        Span::styled(")", Style::default().fg(codex_fg_dim())),
    ]);
    let header_area = Rect::new(area.x, area.y, area.width, 1);
    Paragraph::new(header).render(header_area, buf);

    let (old_w, new_w) = diff_gutter_widths(hunks);

    let lang_ext = path.rsplit('.').next().unwrap_or("");

    for (idx, hunk) in hunks.iter().enumerate() {
        let row = idx as u16 + 1;
        if row >= area.height {
            break;
        }
        let y = area.y + row;
        let row_area = Rect::new(area.x, y, area.width, 1);

        let row_bg = diff_row_bg(hunk.kind);
        if let Some(bg) = row_bg {
            buf.set_style(row_area, Style::default().bg(bg));
        }

        let mut spans = diff_row_gutter(hunk, old_w, new_w, row_bg);
        spans.extend(syntax_spans_for(lang_ext, hunk.text, row_bg));

        Paragraph::new(Line::from(spans)).render(row_area, buf);
    }
}

/// Highlight `text` for a single diff body line. Returns a single raw span
/// (preserving `row_bg`) when the extension isn't recognized, the line is too
/// long, or syntect fails. Otherwise produces one span per syntect range with
/// `bg` forced to `row_bg` so the diff row tint shows through.
pub fn syntax_spans_for(lang_ext: &str, text: &str, row_bg: Option<Color>) -> Vec<Span<'static>> {
    let plain_style = match row_bg {
        Some(bg) => Style::default().fg(codex_fg()).bg(bg),
        None => Style::default().fg(codex_fg()),
    };
    if text.len() > 512 || lang_ext.is_empty() {
        return vec![Span::styled(text.to_string(), plain_style)];
    }
    let assets = syntect_assets();
    let Some(syntax) = assets
        .syntax_set
        .find_syntax_by_extension(lang_ext)
        .or_else(|| assets.syntax_set.find_syntax_by_token(lang_ext))
    else {
        return vec![Span::styled(text.to_string(), plain_style)];
    };
    if syntax.name == "Plain Text" {
        return vec![Span::styled(text.to_string(), plain_style)];
    }
    let mut highlighter = HighlightLines::new(syntax, &assets.theme);
    let mut owned = text.to_string();
    if !owned.ends_with('\n') {
        owned.push('\n');
    }
    let Ok(ranges) = highlighter.highlight_line(&owned, &assets.syntax_set) else {
        return vec![Span::styled(text.to_string(), plain_style)];
    };
    let mut out: Vec<Span<'static>> = Vec::with_capacity(ranges.len());
    for (style, piece) in ranges {
        let trimmed = piece.strip_suffix('\n').unwrap_or(piece);
        if trimmed.is_empty() {
            continue;
        }
        let fg = Color::Rgb(style.foreground.r, style.foreground.g, style.foreground.b);
        let mut ratatui_style = Style::default().fg(fg);
        if let Some(bg) = row_bg {
            ratatui_style = ratatui_style.bg(bg);
        }
        if style.font_style.contains(FontStyle::BOLD) {
            ratatui_style = ratatui_style.add_modifier(Modifier::BOLD);
        }
        if style.font_style.contains(FontStyle::ITALIC) {
            ratatui_style = ratatui_style.add_modifier(Modifier::ITALIC);
        }
        if style.font_style.contains(FontStyle::UNDERLINE) {
            ratatui_style = ratatui_style.add_modifier(Modifier::UNDERLINED);
        }
        out.push(Span::styled(trimmed.to_string(), ratatui_style));
    }
    if out.is_empty() {
        out.push(Span::styled(text.to_string(), plain_style));
    }
    out
}

struct SyntectAssets {
    syntax_set: SyntaxSet,
    theme: Theme,
}

fn syntect_assets() -> &'static SyntectAssets {
    static ASSETS: OnceLock<SyntectAssets> = OnceLock::new();
    ASSETS.get_or_init(|| SyntectAssets {
        syntax_set: SyntaxSet::load_defaults_newlines(),
        theme: build_diff_theme(),
    })
}

/// Hand-built dark theme — `syntect` is compiled without `default-themes` so
/// `ThemeSet::load_defaults()` is unavailable. Mirrors the small palette used
/// by `transcript::syntax::build_theme` so diff highlighting matches fenced
/// code blocks.
fn build_diff_theme() -> Theme {
    let mut theme = Theme {
        name: Some("jekko-diff-dark".to_string()),
        author: Some("Jekko".to_string()),
        ..Default::default()
    };
    let settings = ThemeSettings {
        foreground: Some(syn_color(codex_fg())),
        background: None,
        ..Default::default()
    };
    theme.settings = settings;
    theme.scopes = vec![
        scope_item(
            "comment, punctuation.definition.comment",
            codex_fg_very_dim(),
            Some(FontStyle::ITALIC),
        ),
        scope_item(
            "keyword, storage, storage.modifier, storage.type",
            Color::Rgb(0xd4, 0x72, 0xb6),
            Some(FontStyle::BOLD),
        ),
        scope_item(
            "entity.name.function, support.function, meta.function-call",
            Color::Rgb(0x55, 0xd6, 0xff),
            None,
        ),
        scope_item(
            "string, constant.other.symbol",
            Color::Rgb(0x8a, 0xc8, 0x6a),
            None,
        ),
        scope_item("constant.numeric", Color::Rgb(0xf5, 0xa6, 0x23), None),
        scope_item(
            "entity.name.type, support.type, entity.name.class",
            Color::Rgb(0x55, 0xd6, 0xff),
            None,
        ),
        scope_item(
            "keyword.operator, punctuation, meta.brace, meta.delimiter",
            codex_fg_dim(),
            None,
        ),
        scope_item(
            "constant.language, variable.language",
            Color::Rgb(0x6a, 0xd6, 0xff),
            None,
        ),
    ];
    theme
}

fn scope_item(scope: &str, fg: Color, font_style: Option<FontStyle>) -> ThemeItem {
    ThemeItem {
        #[allow(clippy::manual_unwrap_or_default)]
        scope: match ScopeSelectors::from_str(scope) {
            Ok(selectors) => selectors,
            Err(_) => ScopeSelectors::default(),
        },
        style: StyleModifier {
            foreground: Some(syn_color(fg)),
            background: None,
            font_style: Some(match font_style {
                Some(style) => style,
                None => FontStyle::empty(),
            }),
        },
    }
}

fn syn_color(color: Color) -> SynColor {
    match color {
        Color::Rgb(r, g, b) => SynColor { r, g, b, a: 0xff },
        _ => SynColor::WHITE,
    }
}

/// Render a system notice — login state, errors, mode changes.
pub fn render_system_notice(kind: NoticeKind, text: &str) -> Vec<Line<'static>> {
    let color = kind.color();
    let label = kind.label();
    // T-GLYPH-WAVE2: card margin glyph via GlyphMode.
    let margin = format!("{} ", glyph_set::current().card_margin);
    let mut out = Vec::new();
    for (i, body) in text.lines().enumerate() {
        if i == 0 {
            out.push(Line::from(vec![
                Span::styled(margin.clone(), Style::default().fg(color)),
                Span::styled(
                    format!("{label} "),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(body.to_string(), Style::default().fg(codex_fg())),
            ]));
        } else {
            out.push(Line::from(vec![
                Span::styled(margin.clone(), Style::default().fg(color)),
                Span::styled(
                    " ".repeat(label.len() + 1),
                    Style::default().fg(codex_fg_dim()),
                ),
                Span::styled(body.to_string(), Style::default().fg(codex_fg())),
            ]));
        }
    }
    if out.is_empty() {
        out.push(Line::from(vec![Span::styled(
            margin,
            Style::default().fg(color),
        )]));
    }
    out
}

/// Render a compact inline chip with a label, body, and optional choice pills.
pub fn render_inline_chip(label: &str, body: &str, choices: &[&str]) -> Vec<Line<'static>> {
    let mut spans = vec![
        Span::styled(
            format!("{} ", glyph_set::current().card_margin),
            Style::default().fg(codex_fg_dim()),
        ),
        Span::styled(
            format!("{label} "),
            Style::default()
                .fg(codex_cyan_tab())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(body.to_string(), Style::default().fg(codex_fg_strong())),
    ];
    for choice in choices {
        spans.push(Span::styled(
            format!(" [{choice}]"),
            Style::default()
                .fg(codex_green_ok())
                .add_modifier(Modifier::BOLD),
        ));
    }
    vec![Line::from(spans)]
}

/// Render a permission ask chip with the canonical once/always/reject pills.
pub fn render_permission_chip(
    request_id: &str,
    session_id: &str,
    permission: &str,
    patterns: &[String],
    always: &[String],
) -> Vec<Line<'static>> {
    let mut body = format!("{permission} · {session_id} · {request_id}");
    if !patterns.is_empty() {
        body.push_str(&format!(" · {}", patterns.join(", ")));
    }
    if !always.is_empty() {
        body.push_str(&format!(" · always: {}", always.join(", ")));
    }
    render_inline_chip("permission", &body, &["once", "always", "reject"])
}

/// Render a question chip when the backend provides choices.
pub fn render_question_chip(
    question_id: &str,
    session_id: &str,
    prompt: &str,
    choices: &[String],
) -> Vec<Line<'static>> {
    let body = format!("{prompt} · {session_id} · {question_id}");
    let choice_refs: Vec<&str> = choices.iter().map(|s| s.as_str()).collect();
    render_inline_chip("question", &body, &choice_refs)
}

/// Render a one-line scrollback header — used by the boot loader after the
/// splash collapses, and by session headers.
///
/// ```text
/// ⚡ JEKKO v0.1.0 · ~/code/jekko · branch: main
/// ```
pub fn render_session_header(
    version: &str,
    cwd_display: &str,
    branch: Option<&str>,
) -> Vec<Line<'static>> {
    // T-GLYPH-WAVE2: boot marker glyph defers to GlyphMode (`!` in ASCII).
    // T-GLYPH-WAVE3: separator dot also defers to GlyphMode (`-` in ASCII).
    let g = glyph_set::current();
    let boot = format!("{} ", g.boot_marker);
    let sep = format!(" {} ", g.separator_dot);
    let mut spans = vec![
        Span::styled(boot, Style::default().fg(codex_orange_agent())),
        Span::styled(
            "JEKKO",
            Style::default()
                .fg(codex_fg_strong())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!(" v{version}"), Style::default().fg(codex_fg_dim())),
        Span::styled(sep.clone(), Style::default().fg(codex_fg_very_dim())),
        Span::styled(
            cwd_display.to_string(),
            Style::default().fg(codex_blue_path()),
        ),
    ];
    if let Some(b) = branch {
        spans.push(Span::styled(sep, Style::default().fg(codex_fg_very_dim())));
        spans.push(Span::styled(
            format!("branch: {b}"),
            Style::default().fg(codex_cyan_tab()),
        ));
    }
    vec![Line::from(spans)]
}

/// Helper for callers that need to know the visible cell width of a rendered
/// span sequence (e.g. to right-align metadata). Grapheme-aware.
pub fn line_display_width(line: &Line<'_>) -> u16 {
    let mut w = 0u16;
    for span in &line.spans {
        for g in span.content.graphemes(true) {
            w = w.saturating_add(g.width() as u16);
        }
    }
    w
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::codex;
    use std::time::{Duration, Instant};

    #[test]
    fn user_card_renders_one_line_per_input_line() {
        let lines = render_user("hello\nworld");
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn empty_user_card_still_has_margin() {
        let lines = render_user("");
        assert_eq!(lines.len(), 1);
        // First span is the card margin (`│ ` in Unicode mode, `| ` in ASCII).
        // T-GLYPH-WAVE2: assert via the active GlyphMode so the test stays
        // correct under `JEKKO_ASCII=1` without depending on a side-channel
        // env mutation.
        let expected = format!("{} ", crate::glyph_set::current().card_margin);
        assert_eq!(lines[0].spans[0].content, expected);
    }

    #[test]
    fn tool_call_collapses_long_output() {
        let output: Vec<String> = (0..50).map(|i| format!("line {i}")).collect();
        let call = ToolCall {
            verb: "Bash",
            args: "ls",
            status: ActionStatus::Success,
            output: &output,
            max_output_lines: 5,
        };
        let lines = render_tool_call(&call);
        // header + 5 output + 1 collapsed marker = 7
        assert_eq!(lines.len(), 7);
        // Last line should mention "+45 lines"
        let last_text: String = lines
            .last()
            .unwrap()
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect::<Vec<_>>()
            .join("");
        assert!(last_text.contains("+45 lines"), "got: {last_text}");
    }

    #[test]
    fn tool_call_header_uses_action_bullet() {
        let call = ToolCall {
            verb: "Bash",
            args: "ls",
            status: ActionStatus::Failure,
            output: &[],
            max_output_lines: 10,
        };
        let lines = render_tool_call(&call);
        assert_eq!(lines.len(), 1);
        // T-GLYPH-WAVE2: success bullet glyph defers to GlyphMode (`●` in
        // Unicode mode, `*` in ASCII).
        assert_eq!(
            lines[0].spans[0].content,
            crate::glyph_set::current().bullet_success
        );
        assert_eq!(
            lines[0].spans[0].style.fg,
            Some(codex::SALMON_FAIL),
            "failure bullet must be salmon"
        );
    }

    #[test]
    fn tool_call_live_uses_elapsed_timer_and_pulse() {
        let output = vec!["line 1".to_string(), "line 2".to_string()];
        let call = ToolCall {
            verb: "Bash",
            args: "git status",
            status: ActionStatus::Running,
            output: &output,
            max_output_lines: 5,
        };
        let started = Instant::now();
        let lines = render_tool_call_live(&call, started, started + Duration::from_secs(65), true);
        let joined: String = lines
            .iter()
            .flat_map(|line| line.spans.iter().map(|s| s.content.as_ref()))
            .collect();
        assert!(joined.contains("Bash"));
        assert!(joined.contains("1m 5s"));
        assert!(
            joined.contains('●') || joined.contains('◉') || joined.contains('◌'),
            "running bullet should pulse"
        );
    }

    #[test]
    fn tool_call_live_uses_rotating_verb_when_no_explicit_phase() {
        // Empty / generic verbs are the trigger: render must substitute one of
        // the THINKING_VERBS rather than emit the placeholder text verbatim.
        let started = Instant::now();
        let expected_first = crate::anim::rotating_verb(
            Duration::from_secs(0),
            crate::anim::THINKING_VERBS,
            Duration::from_secs(4),
        );
        for placeholder in ["", "Working", "running", "tool"] {
            let call = ToolCall {
                verb: placeholder,
                args: "",
                status: ActionStatus::Running,
                output: &[],
                max_output_lines: 5,
            };
            let lines = render_tool_call_live(&call, started, started, true);
            let joined: String = lines
                .iter()
                .flat_map(|line| line.spans.iter().map(|s| s.content.as_ref()))
                .collect();
            assert!(
                joined.contains(expected_first),
                "generic verb {placeholder:?} should render {expected_first:?}, got {joined:?}"
            );
            assert!(
                !joined.contains(&format!("({placeholder})")) || placeholder.is_empty(),
                "placeholder verb {placeholder:?} should not leak into the header"
            );
        }

        // Sanity: at a later elapsed bucket the rotation must advance.
        let call = ToolCall {
            verb: "",
            args: "",
            status: ActionStatus::Running,
            output: &[],
            max_output_lines: 5,
        };
        let later = render_tool_call_live(&call, started, started + Duration::from_secs(4), true);
        let joined_later: String = later
            .iter()
            .flat_map(|line| line.spans.iter().map(|s| s.content.as_ref()))
            .collect();
        assert!(
            joined_later.contains(crate::anim::THINKING_VERBS[1]),
            "after one 4s period the second verb {:?} should appear, got {joined_later:?}",
            crate::anim::THINKING_VERBS[1]
        );
    }

    #[test]
    fn tool_call_live_respects_explicit_phase_over_rotation() {
        // Explicit tool names (Bash, Read, Edit, …) must pass through even
        // after enough elapsed time to advance the rotation bucket.
        let started = Instant::now();
        for explicit in ["Bash", "Read", "Edit", "Search", "Web"] {
            let call = ToolCall {
                verb: explicit,
                args: "noop",
                status: ActionStatus::Running,
                output: &[],
                max_output_lines: 5,
            };
            let lines =
                render_tool_call_live(&call, started, started + Duration::from_secs(20), true);
            let joined: String = lines
                .iter()
                .flat_map(|line| line.spans.iter().map(|s| s.content.as_ref()))
                .collect();
            assert!(
                joined.contains(explicit),
                "explicit phase {explicit:?} must survive rotation, got {joined:?}"
            );
            for thinking in crate::anim::THINKING_VERBS {
                assert!(
                    !joined.contains(thinking),
                    "explicit phase {explicit:?} should NOT trigger thinking verb {thinking:?}, got {joined:?}"
                );
            }
        }
    }

    #[test]
    fn diff_renders_added_with_green_sigil() {
        let path = "src/lib.rs";
        let hunks = vec![DiffLine {
            kind: DiffLineKind::Added,
            old_lineno: None,
            new_lineno: Some(42),
            text: "let x = 1;",
        }];
        let lines = render_diff(path, &hunks);
        // header + 1 hunk
        assert_eq!(lines.len(), 2);
        // Hunk line contains a "+" sigil colored green
        let sigil_span = lines[1]
            .spans
            .iter()
            .find(|s| s.content == "+")
            .expect("must have + sigil");
        assert_eq!(sigil_span.style.fg, Some(codex::GREEN_OK));
        assert_eq!(sigil_span.style.bg, Some(codex::DIFF_ADD_BG));
    }

    #[test]
    fn session_header_includes_version_and_cwd() {
        let lines = render_session_header("0.1.0", "~/code/jekko", Some("main"));
        assert_eq!(lines.len(), 1);
        let joined: String = lines[0]
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect::<Vec<_>>()
            .join("");
        assert!(joined.contains("JEKKO"));
        assert!(joined.contains("v0.1.0"));
        assert!(joined.contains("~/code/jekko"));
        assert!(joined.contains("branch: main"));
    }

    #[test]
    fn collapsed_marker_format() {
        let line = collapsed_marker(7, "ctrl+o to expand");
        let joined: String = line
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect::<Vec<_>>()
            .join("");
        assert!(joined.contains("+7 lines"));
        assert!(joined.contains("ctrl+o"));
    }

    fn render_diff_into_buf(width: u16, hunks: &[DiffLine<'_>], path: &str) -> Buffer {
        let height = (hunks.len() as u16) + 1;
        let area = Rect::new(0, 0, width, height);
        let mut buf = Buffer::empty(area);
        render_diff_into(&mut buf, area, path, hunks);
        buf
    }

    #[test]
    fn render_diff_into_paints_full_row_with_add_bg() {
        let hunks = vec![DiffLine {
            kind: DiffLineKind::Added,
            old_lineno: None,
            new_lineno: Some(42),
            text: "x",
        }];
        let buf = render_diff_into_buf(80, &hunks, "src/lib.rs");
        for x in 0..80 {
            let cell = &buf[(x, 1)];
            assert_eq!(
                cell.bg,
                codex::DIFF_ADD_BG,
                "cell at x={x} should have DIFF_ADD_BG, got {:?}",
                cell.bg
            );
        }
    }

    #[test]
    fn render_diff_into_paints_full_row_with_del_bg() {
        let hunks = vec![DiffLine {
            kind: DiffLineKind::Removed,
            old_lineno: Some(7),
            new_lineno: None,
            text: "x",
        }];
        let buf = render_diff_into_buf(80, &hunks, "src/lib.rs");
        for x in 0..80 {
            let cell = &buf[(x, 1)];
            assert_eq!(
                cell.bg,
                codex::DIFF_DEL_BG,
                "cell at x={x} should have DIFF_DEL_BG, got {:?}",
                cell.bg
            );
        }
    }

    #[test]
    fn render_diff_into_context_has_no_bg_fill() {
        let hunks = vec![DiffLine {
            kind: DiffLineKind::Context,
            old_lineno: Some(5),
            new_lineno: Some(5),
            text: "x",
        }];
        let buf = render_diff_into_buf(80, &hunks, "src/lib.rs");
        for x in 0..80 {
            let cell = &buf[(x, 1)];
            assert_eq!(
                cell.bg,
                Color::Reset,
                "context row cell at x={x} must not have a bg fill, got {:?}",
                cell.bg
            );
        }
    }

    #[test]
    fn syntect_highlights_rust_keyword() {
        let row_bg = Some(codex::DIFF_ADD_BG);
        let spans = syntax_spans_for("rs", "let x = 1;", row_bg);
        assert!(spans.len() > 1, "expected multiple spans, got {spans:?}");
        let let_span = spans
            .iter()
            .find(|s| s.content.trim() == "let")
            .expect("must have a span containing `let`");
        assert_ne!(
            let_span.style.fg,
            Some(codex::FG),
            "the `let` keyword should not use the default fg color"
        );
        assert!(let_span.style.fg.is_some(), "let must have a syntect fg");
        for span in &spans {
            assert_eq!(
                span.style.bg,
                Some(codex::DIFF_ADD_BG),
                "every span must carry the row bg, got {:?} for {:?}",
                span.style.bg,
                span.content
            );
        }
    }

    #[test]
    fn syntect_skips_unknown_extension() {
        let spans = syntax_spans_for("xyz", "anything goes here", None);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content, "anything goes here");
        assert_eq!(spans[0].style.fg, Some(codex::FG));
        assert_eq!(spans[0].style.bg, None);
    }

    #[test]
    fn syntect_skips_overlong_line() {
        let long = "a".repeat(600);
        let spans = syntax_spans_for("rs", &long, Some(codex::DIFF_ADD_BG));
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content.len(), 600);
        assert_eq!(spans[0].style.bg, Some(codex::DIFF_ADD_BG));
    }
}
