/// Inputs to [`compute_layout`]. All booleans/desired heights are derived once
/// per frame so the planner stays pure (same inputs -> same plan).
#[derive(Clone, Copy, Debug)]
struct LayoutInputs {
    /// True when a turn is in flight or background work is ticking — promotes
    /// the working strip to one row when the breakpoint allows it.
    working_strip_active: bool,
    /// Popup height already clamped to the composer's slash/mention state.
    /// Always 0 in mini/emergency modes (no headroom for the popup).
    desired_popup_height: u16,
    /// Number of rows the agent rail wants to occupy in the full layout. The
    /// planner caps this per breakpoint.
    desired_panel_rows: u16,
    /// Actual content rows the transcript wants. Splash phase passes the
    /// wordmark height (6); real content passes `transcript.row_count(width)`.
    /// When content fits within the available space, the layout shrinks
    /// transcript to this height and leaves blank terminal rows BELOW the
    /// chrome (composer follows content top, growing-bottom Codex/Claude
    /// pattern). When content overflows, uses the prior fill-and-
    /// anchor-chrome-at-bottom shape.
    content_rows: u16,
}

/// Pixel-precise per-component rectangles for the current frame. `None`
/// means the component is hidden at this breakpoint.
#[derive(Clone, Copy, Debug)]
struct LayoutPlan {
    transcript: Rect,
    working_strip: Option<Rect>,
    permission_banner: Option<Rect>,
    popup: Option<Rect>,
    composer: Rect,
    agent_rail: Option<Rect>,
    footer: Option<Rect>,
    /// Width-derived: ask the agent rail to drop token counts.
    ///
    /// Currently consumed only by tests pending a follow-up
    /// (`agents::panel::render` needs a compact-mode option). See T-??? in
    /// the bundle report.
    #[allow(dead_code)]
    compact_agent_rail: bool,
    /// Width-derived: ask the footer / banner / strip to render a single
    /// packed line instead of multi-segment metadata.
    ///
    /// Pending follow-up: thread through `layout::status_pack::pack` in the
    /// chrome renderers. Currently exposed for unit tests only.
    #[allow(dead_code)]
    pack_status_single_line: bool,
}

/// Vertical breakpoint label. Pure data so tests can pin the tier picked for
/// a given height.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HeightTier {
    Full,
    Compact,
    Mini,
    Emergency,
}

impl HeightTier {
    fn from_height(height: u16) -> Self {
        match height {
            h if h >= 28 => HeightTier::Full,
            h if h >= 18 => HeightTier::Compact,
            h if h >= 10 => HeightTier::Mini,
            _ => HeightTier::Emergency,
        }
    }

    /// Target composer height in cells for this tier (before clamping to the
    /// remaining vertical space).
    fn composer_height(self) -> u16 {
        match self {
            HeightTier::Full | HeightTier::Compact => 3,
            HeightTier::Mini | HeightTier::Emergency => 1,
        }
    }
}

/// Width breakpoint label.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WidthTier {
    Full,
    Abbreviate,
    HideTokens,
    SingleLine,
}

impl WidthTier {
    fn from_width(width: u16) -> Self {
        match width {
            w if w >= 100 => WidthTier::Full,
            w if w >= 70 => WidthTier::Abbreviate,
            w if w >= 50 => WidthTier::HideTokens,
            _ => WidthTier::SingleLine,
        }
    }
}

/// Pure planner. Same inputs always produce the same plan, so it's trivial
/// to unit-test each breakpoint.
fn compute_layout(area: Rect, inputs: LayoutInputs) -> LayoutPlan {
    let h_tier = HeightTier::from_height(area.height);
    let w_tier = WidthTier::from_width(area.width);

    // Per-component desired heights (pre-clamp).
    let composer_h = h_tier.composer_height().min(area.height.max(1));
    let banner_visible = false;
    let strip_visible =
        matches!(h_tier, HeightTier::Compact | HeightTier::Full) && inputs.working_strip_active;
    let footer_visible = matches!(
        h_tier,
        HeightTier::Mini | HeightTier::Compact | HeightTier::Full
    );
    let rail_cap = match h_tier {
        HeightTier::Full => inputs.desired_panel_rows,
        HeightTier::Compact => inputs.desired_panel_rows.min(2),
        HeightTier::Mini | HeightTier::Emergency => 0,
    };
    let popup_desired = match h_tier {
        HeightTier::Full | HeightTier::Compact => inputs.desired_popup_height,
        HeightTier::Mini | HeightTier::Emergency => 0,
    };

    let banner_h: u16 = if banner_visible { 1 } else { 0 };
    let strip_h: u16 = if strip_visible { 1 } else { 0 };
    let footer_h: u16 = if footer_visible { 1 } else { 0 };

    // Reserve fixed rows; transcript and rail share whatever's left.
    let fixed = composer_h
        .saturating_add(banner_h)
        .saturating_add(strip_h)
        .saturating_add(footer_h);
    let flex_total = area.height.saturating_sub(fixed);

    // Popup eats into the flex budget but never below 0.
    let popup_h = popup_desired.min(flex_total);
    let after_popup = flex_total.saturating_sub(popup_h);

    let rail_h = rail_cap.min(after_popup);
    let available_for_transcript = after_popup.saturating_sub(rail_h);

    // Growing-bottom layout (Codex / Claude CLI shape): when content fits in
    // the available transcript area, shrink the transcript slot to its
    // actual row count so chrome anchors immediately below content, and
    // park the remainder as a blank bottom slot. When content overflows,
    // transcript fills the available area + chrome ends up at terminal
    // bottom (compatibility shape).
    let transcript_h = inputs.content_rows.min(available_for_transcript);
    let bottom_pad_h = available_for_transcript.saturating_sub(transcript_h);

    // Now compose a Layout::vertical with the resolved heights.
    let chunks = Layout::vertical([
        Constraint::Length(transcript_h),
        Constraint::Length(strip_h),
        Constraint::Length(banner_h),
        Constraint::Length(popup_h),
        Constraint::Length(composer_h),
        Constraint::Length(rail_h),
        Constraint::Length(footer_h),
        Constraint::Length(bottom_pad_h),
    ])
    .split(area);

    let opt = |rect: Rect| if rect.height == 0 { None } else { Some(rect) };

    LayoutPlan {
        transcript: chunks[0],
        working_strip: opt(chunks[1]),
        permission_banner: opt(chunks[2]),
        popup: opt(chunks[3]),
        composer: chunks[4],
        agent_rail: opt(chunks[5]),
        footer: opt(chunks[6]),
        compact_agent_rail: !matches!(w_tier, WidthTier::Full),
        pack_status_single_line: matches!(w_tier, WidthTier::SingleLine),
    }
}
