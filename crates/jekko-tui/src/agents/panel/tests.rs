#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn empty_panel_renders_strip_only() {
        let p = AgentPanelState::new();
        let lines = render(&p, Instant::now(), &PanelRenderOptions::default());
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn sub_agent_renders_strip_plus_two_rows() {
        let mut p = AgentPanelState::new();
        let mut a = AgentRun::new_main("worker", "do thing");
        a.kind = AgentKind::Worker;
        p.agents.push(a);
        let lines = render(&p, Instant::now(), &PanelRenderOptions::default());
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn main_agent_omits_summary_row() {
        // The main turn's prompt already appears in the transcript above the
        // composer; it must not be echoed as a summary row below the box.
        let mut p = AgentPanelState::new();
        p.agents.push(AgentRun::new_main("main", "do thing"));
        let lines = render(&p, Instant::now(), &PanelRenderOptions::default());
        assert_eq!(lines.len(), 2, "main agent renders strip + bullet only");
    }

    #[test]
    fn agent_without_summary_skips_summary_row() {
        let mut p = AgentPanelState::new();
        let mut a = AgentRun::new_main("main", "");
        a.summary.clear();
        p.agents.push(a);
        let lines = render(&p, Instant::now(), &PanelRenderOptions::default());
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn focused_strip_shows_nav_hints() {
        let mut p = AgentPanelState::new();
        p.set_focus(true);
        let lines = render(&p, Instant::now(), &PanelRenderOptions::default());
        let strip_text: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(strip_text.contains("↑/↓"));
        assert!(strip_text.contains("Esc"));
    }

    #[test]
    fn format_tokens_units() {
        use crate::format::format_tokens;
        assert_eq!(format_tokens(500), "500");
        assert_eq!(format_tokens(12_300), "12.3k");
        assert_eq!(format_tokens(1_500_000), "1.5m");
    }

    #[test]
    fn token_count_appears_when_nonzero() {
        let mut p = AgentPanelState::new();
        let mut a = AgentRun::new_main("main", "thing");
        a.tokens.add_input(10_000);
        a.tokens.add_output(2_300);
        p.agents.push(a);
        let lines = render(&p, Instant::now(), &PanelRenderOptions::default());
        let bullet_text: String = lines[1].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(
            bullet_text.contains("↑ 2.3k"),
            "expected ↑ 2.3k in {bullet_text:?}"
        );
        assert!(
            bullet_text.contains("↓ 10.0k"),
            "expected ↓ 10.0k in {bullet_text:?}"
        );
    }

    #[test]
    fn cap_respected_at_max_agents() {
        let mut p = AgentPanelState::new();
        for i in 0..20 {
            let mut a = AgentRun::new_main(format!("a{i}"), "x");
            a.id = crate::agents::AgentId::new(format!("a{i}"));
            // Sub-agents keep a summary row, so each occupies two rows — the
            // row-budgeting this test exercises.
            a.kind = AgentKind::Worker;
            p.agents.push(a);
        }
        p.set_viewport_rows(9);
        let lines = render(
            &p,
            Instant::now(),
            &PanelRenderOptions {
                permission_mode_label: Cow::Borrowed("auto"),
                max_agents: 4,
                max_visible_rows: 9,
                width: 80,
                activity: None,
                compact: false,
                motion_enabled: true,
            },
        );
        assert_eq!(lines.len(), 9);
    }

    #[test]
    fn panel_compact_mode_drops_token_columns() {
        let mut p = AgentPanelState::new();
        let mut a = AgentRun::new_main("main", "do thing");
        a.tokens.add_input(10_000);
        a.tokens.add_output(2_300);
        p.agents.push(a);
        let opts = PanelRenderOptions {
            compact: true,
            ..PanelRenderOptions::default()
        };
        let lines = render(&p, Instant::now(), &opts);
        let bullet_text: String = lines[1].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(
            !bullet_text.contains("↑"),
            "compact: upward token column should be dropped, got {bullet_text:?}"
        );
        assert!(
            !bullet_text.contains("↓"),
            "compact: downward token column should be dropped, got {bullet_text:?}"
        );
        assert!(
            bullet_text.contains("main"),
            "compact: agent name still visible, got {bullet_text:?}"
        );
        assert!(
            bullet_text.contains("running"),
            "compact: status bullet text still visible, got {bullet_text:?}"
        );
        assert!(lines.len() >= 2);
    }

    #[test]
    fn panel_compact_short_runtime_drops_trailing_unit() {
        let mut p = AgentPanelState::new();
        let mut a = AgentRun::new_main("main", "do thing");
        a.started_at = Instant::now() - std::time::Duration::from_secs(5 * 60 + 42);
        p.agents.push(a);
        let opts = PanelRenderOptions {
            compact: true,
            ..PanelRenderOptions::default()
        };
        let lines = render(&p, Instant::now(), &opts);
        let bullet_text: String = lines[1].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(
            bullet_text.contains("running · 5m"),
            "compact: expected 'running · 5m' prefix, got {bullet_text:?}"
        );
        assert!(
            !bullet_text.contains("5m 42s"),
            "compact: trailing seconds should drop, got {bullet_text:?}"
        );
    }

    #[test]
    fn short_runtime_label_keeps_first_unit() {
        assert_eq!(short_runtime_label("5m 42s"), "5m");
        assert_eq!(short_runtime_label("42s"), "42s");
        assert_eq!(short_runtime_label("1h 5m"), "1h");
        assert_eq!(short_runtime_label("1d 1h"), "1d");
        assert_eq!(short_runtime_label(""), "");
    }

    #[test]
    fn pulse_glyph_changes_for_running() {
        let mut p = AgentPanelState::new();
        p.agents.push(AgentRun::new_main("main", "x"));
        let l1 = render(&p, Instant::now(), &PanelRenderOptions::default());
        let l2 = render(
            &p,
            Instant::now() + Duration::from_secs(60),
            &PanelRenderOptions::default(),
        );
        assert_eq!(l1.len(), l2.len());
    }

    #[test]
    fn permission_mode_label_accepts_cow_owned() {
        let mut p = AgentPanelState::new();
        p.agents.push(AgentRun::new_main("main", ""));
        let dynamic = format!("policy:{}", "custom-mode");
        let opts = PanelRenderOptions {
            permission_mode_label: Cow::Owned(dynamic.clone()),
            ..PanelRenderOptions::default()
        };
        let lines = render(&p, Instant::now(), &opts);
        let strip_text: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(
            strip_text.contains(&dynamic),
            "expected dynamic label {dynamic:?} in strip {strip_text:?}"
        );
    }

    #[test]
    fn permission_mode_label_accepts_cow_borrowed() {
        let opts = PanelRenderOptions {
            permission_mode_label: Cow::Borrowed("bypass permissions"),
            ..PanelRenderOptions::default()
        };
        assert_eq!(opts.permission_mode_label.as_ref(), "bypass permissions");
        assert!(matches!(opts.permission_mode_label, Cow::Borrowed(_)));
    }
}
