#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn echo_backend_produces_deltas_then_complete() {
        let mut backend = EchoBackend;
        let rx = backend.start_turn("hi".into(), CancellationToken::new());
        let mut got_delta = false;
        let mut got_complete = false;
        for _ in 0..20 {
            match rx.recv_timeout(Duration::from_secs(2)) {
                Ok(ChatEvent::AssistantDelta(_)) => got_delta = true,
                Ok(ChatEvent::TurnComplete) => {
                    got_complete = true;
                    break;
                }
                Ok(ChatEvent::TurnFailed(_)) => panic!("unexpected failure"),
                Ok(ChatEvent::Runtime(_)) => {}
                Ok(ChatEvent::Reasoning { .. }) => {}
                Ok(ChatEvent::Tool(_)) => {}
                Ok(ChatEvent::Diff { .. }) => {}
                Ok(ChatEvent::Notice(_, _)) => {}
                Err(_) => break,
            }
        }
        assert!(got_delta, "expected at least one delta");
        assert!(got_complete, "expected a TurnComplete");
    }

    #[test]
    fn in_flight_tracks_tool_events() {
        let mut state = InFlight::new();

        state.apply_tool_event(ToolEvent::Start {
            id: "tool-1".into(),
            name: "Bash".into(),
            input: Some("git status".into()),
        });
        assert!(matches!(
            state.latest_tool(),
            Some(tool) if tool.name == "Bash" && tool.status == ToolChipStatus::Running
        ));

        state.apply_tool_event(ToolEvent::StdoutChunk {
            id: "tool-1".into(),
            chunk: "dirty".into(),
        });
        assert!(matches!(
            state.latest_tool(),
            Some(tool) if tool.last_chunk.as_deref() == Some("dirty")
        ));

        let terminal = state.apply_tool_event(ToolEvent::Complete {
            id: "tool-1".into(),
        });
        assert!(matches!(
            terminal,
            Some(tool) if tool.status == ToolChipStatus::Success
        ));
        assert!(state.latest_tool().is_none());
    }

    #[test]
    fn multi_tool_turn_preserves_first_tool_output() {
        // T-SEMANTIC-TRANSCRIPT-A: two overlapping tools (A then B) must each
        // keep their own captured output so the first tool's stdout isn't
        // overwritten when the second tool starts.
        let mut state = InFlight::new();

        let _ = state.apply_tool_event(ToolEvent::Start {
            id: "tool-a".into(),
            name: "Bash".into(),
            input: Some("echo hello".into()),
        });
        let _ = state.apply_tool_event(ToolEvent::StdoutChunk {
            id: "tool-a".into(),
            chunk: "hello".into(),
        });

        // Tool B starts before A completes (multi-tool scenario).
        let _ = state.apply_tool_event(ToolEvent::Start {
            id: "tool-b".into(),
            name: "Bash".into(),
            input: Some("echo world".into()),
        });
        let _ = state.apply_tool_event(ToolEvent::StdoutChunk {
            id: "tool-b".into(),
            chunk: "world".into(),
        });

        // Both tools should still be live with their own output.
        assert_eq!(state.active_tools.len(), 2);
        assert_eq!(
            state.active_tools.get("tool-a").map(|t| t.output.as_str()),
            Some("hello"),
            "tool A's output must survive tool B's Start"
        );
        assert_eq!(
            state.active_tools.get("tool-b").map(|t| t.output.as_str()),
            Some("world")
        );

        let chip_a = state
            .apply_tool_event(ToolEvent::Complete {
                id: "tool-a".into(),
            })
            .expect("Complete(A) yields chip A");
        let chip_b = state
            .apply_tool_event(ToolEvent::Complete {
                id: "tool-b".into(),
            })
            .expect("Complete(B) yields chip B");

        assert_eq!(chip_a.output, "hello", "tool A's output preserved");
        assert_eq!(chip_b.output, "world", "tool B's output preserved");
        assert!(state.active_tools.is_empty());
    }

    #[test]
    fn tool_event_unknown_id_is_noop() {
        // T-SEMANTIC-TRANSCRIPT-A: a StdoutChunk for an id that was never
        // started (or already completed) must be a silent drop — it must not
        // panic, mutate other tools' buffers, or yield a terminal chip.
        let mut state = InFlight::new();

        // No active tools yet.
        let returned = state.apply_tool_event(ToolEvent::StdoutChunk {
            id: "ghost".into(),
            chunk: "no-op".into(),
        });
        assert!(returned.is_none());
        assert!(state.active_tools.is_empty());

        let returned = state.apply_tool_event(ToolEvent::StderrChunk {
            id: "ghost".into(),
            chunk: "no-op".into(),
        });
        assert!(returned.is_none());

        // Complete for an unknown id is also a no-op (no chip yielded).
        let returned = state.apply_tool_event(ToolEvent::Complete { id: "ghost".into() });
        assert!(returned.is_none());

        // Tool A alive — a stray chunk targeting the ghost id must not bleed
        // into A's output.
        let _ = state.apply_tool_event(ToolEvent::Start {
            id: "tool-a".into(),
            name: "Bash".into(),
            input: None,
        });
        let _ = state.apply_tool_event(ToolEvent::StdoutChunk {
            id: "ghost".into(),
            chunk: "stray".into(),
        });
        assert_eq!(
            state.active_tools.get("tool-a").map(|t| t.output.as_str()),
            Some("")
        );
    }

    #[test]
    fn active_tool_card_renders_elapsed_tool_header() {
        let tool = ActiveToolChip {
            _id: "tool-1".into(),
            name: "Bash".into(),
            input: Some("git status".into()),
            status: ToolChipStatus::Running,
            last_chunk: None,
            output: "dirty\n".into(),
            stdout: "dirty\n".into(),
            stderr: String::new(),
            started_at: Instant::now() - Duration::from_secs(65),
        };
        // T-INLINE-WAVE3 #1: `render_active_tool_card` now takes
        // `motion_enabled: bool` after sibling T-GLYPH-WAVE3 widening.
        // Pass `true` so the rotating verb / pulsing bullet behave like the
        // pre-widening run; this test asserts header content, not motion.
        let lines = render_active_tool_card(&tool, true);
        let text: String = lines
            .iter()
            .flat_map(|line| line.spans.iter().map(|s| s.content.as_ref()))
            .collect();
        assert!(text.contains("Bash"));
        assert!(text.contains("1m 5s"));
        assert!(text.contains("dirty"));
    }

    #[test]
    fn echo_backend_emits_tool_flow() {
        let mut backend = EchoBackend;
        let rx = backend.start_turn("ping".into(), CancellationToken::new());
        let mut saw_start = false;
        let mut saw_complete = false;
        for _ in 0..200 {
            match rx.recv_timeout(Duration::from_secs(2)) {
                Ok(ChatEvent::Tool(ToolEvent::Start { name, .. })) if name == "Bash" => {
                    saw_start = true;
                }
                Ok(ChatEvent::Tool(ToolEvent::Complete { .. })) => {
                    saw_complete = true;
                }
                Ok(ChatEvent::TurnComplete) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        assert!(saw_start, "expected a Bash tool Start");
        assert!(saw_complete, "expected a Tool Complete");
    }

    #[test]
    fn echo_backend_reports_cancelled_when_token_is_raised() {
        let mut backend = EchoBackend;
        let token = CancellationToken::new();
        let rx = backend.start_turn("slow".into(), token.clone());
        token.cancel_hard();
        let mut saw_cancelled = false;
        for _ in 0..20 {
            match rx.recv_timeout(Duration::from_secs(2)) {
                Ok(ChatEvent::TurnFailed(err)) if err == "cancelled" => {
                    saw_cancelled = true;
                    break;
                }
                Ok(ChatEvent::TurnComplete) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        assert!(saw_cancelled, "expected cancelled TurnFailed");
    }

    #[test]
    fn cancellation_notice_records_explicit_level() {
        let lines = render_system_notice(
            NoticeKind::Warn,
            &format!(
                "cancellation requested: {}",
                cancel_level_label(CancelLevel::Hard)
            ),
        );
        let text: String = lines
            .iter()
            .flat_map(|line| line.spans.iter().map(|span| span.content.as_ref()))
            .collect();
        assert!(text.contains("cancellation requested"));
        assert!(text.contains("hard stop"));
    }

    #[test]
    fn chunk_string_respects_utf8_boundaries() {
        let s = "héllo wörld";
        let chunks = chunk_string(s, 3);
        let joined: String = chunks.iter().copied().collect();
        assert_eq!(joined, s);
    }

    fn fixture_catalog() -> SlashCatalog {
        SlashCatalog::new()
    }

    #[test]
    fn slash_state_activates_on_leading_slash() {
        let cat = fixture_catalog();
        let mut c = ComposerState::default();
        c.text.push('/');
        c.sync_slash(&cat);
        assert!(c.slash.active);
        assert!(!c.slash.filtered.is_empty());
    }

    #[test]
    fn slash_state_filters_by_prefix() {
        let cat = fixture_catalog();
        let mut c = ComposerState::default();
        c.text.push_str("/he");
        c.sync_slash(&cat);
        assert!(c.slash.active);
        let cmd = c.slash.current_command(&cat).expect("filter has a match");
        assert_eq!(cmd.id(), "help");
    }

    #[test]
    fn slash_state_deactivates_on_whitespace() {
        let cat = fixture_catalog();
        let mut c = ComposerState::default();
        c.text.push_str("/help me");
        c.sync_slash(&cat);
        assert!(
            !c.slash.active,
            "space after the slash word kills the popup"
        );
    }

    #[test]
    fn slash_state_clears_when_text_clears() {
        let cat = fixture_catalog();
        let mut c = ComposerState::default();
        c.text.push_str("/quit");
        c.sync_slash(&cat);
        assert!(c.slash.active);
        c.text.clear();
        c.sync_slash(&cat);
        assert!(!c.slash.active);
    }

    #[test]
    fn slash_state_empty_filter_when_no_matches() {
        let cat = fixture_catalog();
        let mut c = ComposerState::default();
        c.text.push_str("/zzzz");
        c.sync_slash(&cat);
        assert!(c.slash.active);
        assert!(c.slash.filtered.is_empty());
        assert!(c.slash.current_command(&cat).is_none());
    }

    #[test]
    fn slash_action_maps_known_ids() {
        let cat = fixture_catalog();
        assert_eq!(cat.action_for("help"), SlashAction::Help);
        assert_eq!(cat.action_for("quit"), SlashAction::Quit);
        assert_eq!(cat.action_for("clear"), SlashAction::Clear);
        assert_eq!(cat.action_for("new"), SlashAction::NewSession);
        assert_eq!(cat.action_for("unknown"), SlashAction::Unknown);
    }

    #[test]
    fn slash_state_hides_panels_by_default() {
        let cat = fixture_catalog();
        let mut c = ComposerState::default();
        c.text.push_str("/panels");
        c.sync_slash(&cat);
        assert!(c.slash.active);
        assert!(c.slash.filtered.iter().all(|id| id != "panels"));
    }

    #[test]
    fn slash_state_opens_tier_2_submenu_for_parent() {
        let cat = fixture_catalog();
        let mut c = ComposerState::default();
        c.text.push_str("/keys");
        c.sync_slash(&cat);
        let parent_id = c
            .slash
            .current_command(&cat)
            .expect("keys should be selected")
            .id()
            .to_string();

        assert!(c.slash.open_submenu(&cat, &parent_id));
        assert_eq!(c.slash.query, "keys");
        assert_eq!(c.slash.selection_len(&cat), 7);
        let (parent, shell_base, item) = c
            .slash
            .selected_subcommand(&cat)
            .expect("first child should be selected");
        assert_eq!(parent, "keys");
        assert_eq!(shell_base, "jekko keys");
        assert_eq!(item.id, "set <PROVIDER>");
        assert!(c.slash.current_command(&cat).is_none());
    }

    #[test]
    fn slash_state_navigates_and_pops_submenu() {
        let cat = fixture_catalog();
        let mut c = ComposerState::default();
        c.text.push_str("/daemon");
        c.sync_slash(&cat);
        assert!(c.slash.open_submenu(&cat, "daemon"));

        c.slash.move_next(&cat);
        c.slash.move_next(&cat);
        let (_, _, item) = c
            .slash
            .selected_subcommand(&cat)
            .expect("third child should be selected");
        assert_eq!(item.id, "status");

        assert!(c.slash.pop_submenu());
        assert!(c.slash.submenu.is_none());
        assert_eq!(
            c.slash
                .current_command(&cat)
                .expect("parent filter should still be active")
                .id(),
            "daemon"
        );
        assert!(!c.slash.pop_submenu());
    }

    #[test]
    fn slash_submenu_child_notice_points_to_shell_fallback() {
        let cat = fixture_catalog();
        let submenu = cat.submenu_for("mcp").expect("mcp submenu");
        let item = submenu.item(1).expect("attach child");
        let msg = submenu_child_notice_for(submenu.shell_base, item);
        assert!(msg.contains("Run `jekko mcp attach <NAME> <TARGET>`"));
        assert!(msg.contains("in-TUI sub-action execution is a follow-up"));
    }

    #[test]
    fn composer_collapses_long_paste_to_chip() {
        let cat = fixture_catalog();
        let (_dir, idx) = fixture_index();
        let mut composer = ComposerState::default();
        let payload = long_multiline_paste();
        composer.insert_paste(payload.clone(), &idx, &cat);
        assert!(composer.text.contains("[paste #1"));
        assert_eq!(composer.paste.records().len(), 1);
        let expanded = composer.take_expanded_text();
        assert_eq!(expanded, payload);
    }

    #[test]
    fn composer_keeps_short_paste_inline() {
        let cat = fixture_catalog();
        let (_dir, idx) = fixture_index();
        let mut composer = ComposerState::default();
        composer.insert_paste("short paste".to_string(), &idx, &cat);
        assert_eq!(composer.text, "short paste");
        assert!(composer.paste.records().is_empty());
    }

    use std::fs;
    use tempfile::TempDir;

    fn fixture_index() -> (TempDir, FileIndex) {
        let dir = TempDir::new().unwrap();
        for rel in [
            "src/main.rs",
            "src/lib.rs",
            "src/runner.rs",
            "docs/README.md",
        ] {
            let full = dir.path().join(rel);
            fs::create_dir_all(full.parent().unwrap()).unwrap();
            fs::write(full, "x").unwrap();
        }
        let idx = FileIndex::build(dir.path(), 1000);
        (dir, idx)
    }

    fn long_multiline_paste() -> String {
        let mut s = String::new();
        for i in 0..(crate::prompt::PASTE_LINE_THRESHOLD + 2) {
            s.push_str(&format!("line {i}\n"));
        }
        s
    }

    #[test]
    fn detect_mention_trigger_finds_at_start() {
        let (offset, q) = detect_mention_trigger("@src").unwrap();
        assert_eq!(offset, 0);
        assert_eq!(q, "src");
    }

    #[test]
    fn detect_mention_trigger_finds_after_space() {
        let (offset, q) = detect_mention_trigger("look at @main").unwrap();
        assert_eq!(offset, 8);
        assert_eq!(q, "main");
    }

    #[test]
    fn detect_mention_trigger_ignores_email_like() {
        assert!(detect_mention_trigger("user@example").is_none());
    }

    #[test]
    fn detect_mention_trigger_breaks_on_whitespace() {
        assert!(detect_mention_trigger("@foo bar").is_none());
    }

    #[test]
    fn mention_state_activates_on_at_sign() {
        let cat = fixture_catalog();
        let (_dir, idx) = fixture_index();
        let mut c = ComposerState::default();
        c.text.push('@');
        c.sync_popups(&idx, &cat);
        assert!(c.mention.active);
        assert!(!c.mention.filtered.is_empty());
    }

    #[test]
    fn mention_state_filters_by_query() {
        let cat = fixture_catalog();
        let (_dir, idx) = fixture_index();
        let mut c = ComposerState::default();
        c.text.push_str("@run");
        c.sync_popups(&idx, &cat);
        assert!(c.mention.active);
        let first = c.mention.current_path().expect("at least one match");
        assert!(first.file_name().unwrap().to_string_lossy().contains("run"));
    }

    #[test]
    fn mention_state_clears_on_whitespace() {
        let cat = fixture_catalog();
        let (_dir, idx) = fixture_index();
        let mut c = ComposerState::default();
        c.text.push_str("@main fix");
        c.sync_popups(&idx, &cat);
        assert!(!c.mention.active);
    }

    #[test]
    fn mention_state_yields_to_slash() {
        let cat = fixture_catalog();
        let (_dir, idx) = fixture_index();
        let mut c = ComposerState::default();
        c.text.push('/');
        c.sync_popups(&idx, &cat);
        assert!(c.slash.active);
        assert!(!c.mention.active, "slash wins when both could trigger");
    }

    #[test]
    fn accept_mention_replaces_query_with_path() {
        let cat = fixture_catalog();
        let (_dir, idx) = fixture_index();
        let mut c = ComposerState::default();
        c.text.push_str("see @run");
        c.sync_popups(&idx, &cat);
        assert!(c.mention.active);
        c.accept_mention();
        assert!(c.text.starts_with("see @"));
        assert!(c.text.contains("runner.rs"));
        assert!(!c.mention.active);
    }

    #[test]
    fn bottom_rule_full_width_when_no_branch() {
        let mut buf = ratatui::buffer::Buffer::empty(Rect::new(0, 0, 40, 1));
        render_bottom_rule_with_branch(&mut buf, Rect::new(0, 0, 40, 1), None);
        let row: String = (0..40).map(|x| buf[(x, 0)].symbol().to_string()).collect();
        assert!(
            row.chars().all(|c| c == '─'),
            "expected all dashes, got: {row}"
        );
    }

    #[test]
    fn bottom_rule_overlays_branch_tab_on_right() {
        let mut buf = ratatui::buffer::Buffer::empty(Rect::new(0, 0, 40, 1));
        render_bottom_rule_with_branch(&mut buf, Rect::new(0, 0, 40, 1), Some("main"));
        let row: String = (0..40).map(|x| buf[(x, 0)].symbol().to_string()).collect();
        // " main " = 6 cells on the right; the rest is the dash rule.
        assert!(
            row.ends_with(" main "),
            "expected branch tab on right, got: {row}"
        );
        assert!(row.starts_with("─"), "left should still be the rule");
    }

    #[test]
    fn bottom_rule_truncates_long_branch_from_left() {
        let mut buf = ratatui::buffer::Buffer::empty(Rect::new(0, 0, 30, 1));
        render_bottom_rule_with_branch(
            &mut buf,
            Rect::new(0, 0, 30, 1),
            Some("feature/very-long-branch-name-that-overflows"),
        );
        let row: String = (0..30).map(|x| buf[(x, 0)].symbol().to_string()).collect();
        // Truncation marker `…` should appear; tail should contain "name " (last
        // chars of the original branch).
        assert!(
            row.contains("…"),
            "expected ellipsis truncation, got: {row}"
        );
    }

    // ── T1-V1b: live composer chevron wiring ────────────────────────────────

    #[test]
    fn composer_row_paints_blue_chevron_on_row_zero() {
        let area = Rect::new(0, 0, 30, 1);
        let mut buf = Buffer::empty(area);
        render_composer_row(&mut buf, area, "hi", false, None, None);

        let cell0 = &buf[(0, 0)];
        assert_eq!(cell0.symbol(), PROMPT_GLYPH, "col 0 row 0 must hold `›`");
        assert_eq!(
            cell0.style().fg,
            Some(BLUE_PATH),
            "chevron must be styled with BLUE_PATH"
        );

        let cell1 = &buf[(1, 0)];
        assert_eq!(cell1.symbol(), " ", "col 1 row 0 must be blank");

        // Body starts at col 2 (PROMPT_PREFIX_WIDTH = 2).
        assert_eq!(buf[(2, 0)].symbol(), "h", "body should start at col 2");
        assert_eq!(buf[(3, 0)].symbol(), "i");
    }

    #[test]
    fn composer_row_continuation_rows_have_blank_gutter() {
        // Simulate a future multi-line composer growth: a 4-row prompt area.
        // Only row 0 carries `›`; rows 1..3 must be entirely blank in cols 0-1.
        let area = Rect::new(0, 0, 20, 4);
        let mut buf = Buffer::empty(area);
        render_composer_row(&mut buf, area, "first\nsecond", false, None, None);

        assert_eq!(buf[(0, 0)].symbol(), PROMPT_GLYPH, "row 0 col 0 = `›`");
        assert_eq!(buf[(1, 0)].symbol(), " ", "row 0 col 1 = blank");

        for y in 1u16..4 {
            assert_ne!(
                buf[(0, y)].symbol(),
                PROMPT_GLYPH,
                "row {y} must not carry a second chevron"
            );
            assert_eq!(buf[(0, y)].symbol(), " ", "row {y} col 0 must be blank");
            assert_eq!(buf[(1, y)].symbol(), " ", "row {y} col 1 must be blank");
        }
    }

    #[test]
    fn composer_row_body_text_shifts_right_by_prefix_width() {
        // The body must start at column PROMPT_PREFIX_WIDTH (=2); the literal
        // first character of `input` belongs in the cell at col 2, not col 0.
        let area = Rect::new(0, 0, 30, 1);
        let mut buf = Buffer::empty(area);
        render_composer_row(&mut buf, area, "hello", false, None, None);

        let prefix_width = PROMPT_PREFIX_WIDTH;
        for (i, ch) in "hello".chars().enumerate() {
            let cell = &buf[(prefix_width + i as u16, 0)];
            assert_eq!(
                cell.symbol(),
                ch.to_string(),
                "char {ch:?} should land at col {}",
                prefix_width as usize + i
            );
        }
    }

    #[test]
    fn composer_row_narrow_area_falls_back_without_panicking() {
        // 2-col area cannot afford prefix + ≥1 body col. Must still render
        // without panicking and without writing past the area.
        let area = Rect::new(0, 0, 2, 1);
        let mut buf = Buffer::empty(area);
        render_composer_row(&mut buf, area, "anything", false, None, None);
        // No chevron painted into the gutter in fallback mode (would overflow
        // the body otherwise). Just ensure the call succeeds.
    }

    #[test]
    fn composer_row_zero_area_is_noop() {
        let area = Rect::new(0, 0, 0, 0);
        let mut buf = Buffer::empty(area);
        // Just verifying no panic on a degenerate input.
        render_composer_row(&mut buf, area, "", false, None, None);
    }

    #[test]
    fn composer_row_renders_empty_hint_when_empty() {
        let area = Rect::new(0, 0, 24, 1);
        let mut buf = Buffer::empty(area);
        render_composer_row(&mut buf, area, "", false, None, Some("answer the question"));
        let mut text = String::new();
        for x in 0..24 {
            text.push_str(buf[(x, 0)].symbol());
        }
        assert!(text.contains("answer the question"));
    }

    #[test]
    fn composer_shell_render_paints_blue_chevron_in_layout() {
        // Confirm the full ComposerShell wiring still surfaces the blue `›` at
        // the composer's first content row (row 1 of the 3-row shell: top
        // rule, prompt, bottom rule).
        let area = Rect::new(0, 0, 30, 3);
        let mut buf = Buffer::empty(area);
        ComposerShell {
            input: "x",
            streaming: false,
            streaming_preview: None,
            empty_hint: None,
            branch: None,
        }
        .render(area, &mut buf);

        let cell = &buf[(0, 1)];
        assert_eq!(cell.symbol(), PROMPT_GLYPH);
        assert_eq!(cell.style().fg, Some(BLUE_PATH));
        // Body char at col 2.
        assert_eq!(buf[(2, 1)].symbol(), "x");
    }

    #[test]
    fn composer_shell_streaming_keeps_blue_chevron_gutter() {
        // Even while streaming, the gutter chevron must remain (the chevron
        // identifies the composer surface; the streaming indicator is body
        // content shifted into the inner rect).
        let area = Rect::new(0, 0, 40, 3);
        let mut buf = Buffer::empty(area);
        ComposerShell {
            input: "",
            streaming: true,
            streaming_preview: Some("tok"),
            empty_hint: None,
            branch: None,
        }
        .render(area, &mut buf);

        let cell = &buf[(0, 1)];
        assert_eq!(cell.symbol(), PROMPT_GLYPH);
        assert_eq!(cell.style().fg, Some(BLUE_PATH));
    }

    // ── T1-V2/V3/V4: bottom-chrome layout helpers ────────────────────────────

    #[test]
    fn permission_hint_for_each_focus_area() {
        // The banner picks one of three canonical hints based on focus. The
        // mapping lives entirely in `permission_hint_for`, so a one-line
        // table-driven test pins the contract.
        assert_eq!(permission_hint_for(FocusArea::Composer), HINT_CHAT_FOCUS);
        assert_eq!(
            permission_hint_for(FocusArea::Agents),
            HINT_AGENT_PANEL_FOCUS
        );
    }

    #[test]
    fn footer_info_for_uses_boot_context_fields() {
        // The footer builder copies cwd + branch verbatim from BootContext and
        // pulls model/effort from env. We assert the values that don't depend
        // on env (cwd/branch/profile) directly.
        let ctx = BootContext {
            version: "9.9.9".into(),
            cwd_display: "~/somewhere".into(),
            branch: Some("feature/x".into()),
        };
        let info = footer_info_for(&ctx, None, None);
        assert_eq!(info.cwd, "~/somewhere");
        assert_eq!(info.branch.as_deref(), Some("feature/x"));
        assert!(
            info.profile.is_none(),
            "profile stays None when no override"
        );
    }

    #[test]
    fn footer_info_drops_branch_when_no_git() {
        let ctx = BootContext {
            version: "0.1.0".into(),
            cwd_display: "/tmp".into(),
            branch: None,
        };
        let info = footer_info_for(&ctx, None, None);
        assert!(info.branch.is_none());
    }

    #[test]
    fn footer_info_threads_profile_through() {
        // T-INLINE-CLUSTER #1: when the caller passes a profile string, it
        // lands in `FooterInfo.profile` so the footer renderer can surface it
        // as `[dev]` etc. Tied to `InlineRuntimeOptions.profile`.
        let ctx = BootContext {
            version: "0.1.0".into(),
            cwd_display: "/tmp".into(),
            branch: None,
        };
        let info = footer_info_for(&ctx, Some("dev"), None);
        assert_eq!(info.profile.as_deref(), Some("dev"));
    }

    // ── T1-V5b: diff dispatch ───────────────────────────────────────────────

    pub(super) fn sample_diff_payload() -> (String, Vec<DiffBlockLine>) {
        use crate::transcript::inline_cards::DiffLineKind;
        let path = "src/lib.rs".to_string();
        let hunks = vec![
            DiffBlockLine {
                kind: DiffLineKind::Context,
                old_lineno: Some(1),
                new_lineno: Some(1),
                text: "line one".into(),
            },
            DiffBlockLine {
                kind: DiffLineKind::Removed,
                old_lineno: Some(2),
                new_lineno: None,
                text: "before".into(),
            },
            DiffBlockLine {
                kind: DiffLineKind::Added,
                old_lineno: None,
                new_lineno: Some(2),
                text: "after".into(),
            },
        ];
        (path, hunks)
    }

    #[test]
    fn render_diff_lines_from_payload_emits_header_plus_one_row_per_hunk() {
        let (path, hunks) = sample_diff_payload();
        let lines = render_diff_lines_from_payload(&path, &hunks);
        // Header + 3 body lines.
        assert_eq!(lines.len(), 4);
        let header: String = lines[0]
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect::<Vec<_>>()
            .join("");
        assert!(header.contains("Edit"));
        assert!(header.contains("src/lib.rs"));
    }

    // ── T2-P4: responsive layout breakpoints ────────────────────────────────

    pub(super) fn layout_inputs(working: bool, popup: u16, panel_rows: u16) -> LayoutInputs {
        layout_inputs_with_content(working, popup, panel_rows, u16::MAX)
    }

    /// Test fixture: build LayoutInputs with explicit content_rows so tests
    /// can pin the growing-bottom shape (small content = blank tail) vs the
    /// compatibility bottom-anchored shape (overflow = fill + chrome at bottom).
    fn layout_inputs_with_content(
        working: bool,
        popup: u16,
        panel_rows: u16,
        content_rows: u16,
    ) -> LayoutInputs {
        LayoutInputs {
            working_strip_active: working,
            desired_popup_height: popup,
            desired_panel_rows: panel_rows,
            content_rows,
        }
    }

    #[test]
    fn layout_height_tier_picker_matches_spec() {
        assert_eq!(HeightTier::from_height(9), HeightTier::Emergency);
        assert_eq!(HeightTier::from_height(10), HeightTier::Mini);
        assert_eq!(HeightTier::from_height(17), HeightTier::Mini);
        assert_eq!(HeightTier::from_height(18), HeightTier::Compact);
        assert_eq!(HeightTier::from_height(27), HeightTier::Compact);
        assert_eq!(HeightTier::from_height(28), HeightTier::Full);
        assert_eq!(HeightTier::from_height(60), HeightTier::Full);
    }

    #[test]
    fn layout_width_tier_picker_matches_spec() {
        assert_eq!(WidthTier::from_width(49), WidthTier::SingleLine);
        assert_eq!(WidthTier::from_width(50), WidthTier::HideTokens);
        assert_eq!(WidthTier::from_width(69), WidthTier::HideTokens);
        assert_eq!(WidthTier::from_width(70), WidthTier::Abbreviate);
        assert_eq!(WidthTier::from_width(99), WidthTier::Abbreviate);
        assert_eq!(WidthTier::from_width(100), WidthTier::Full);
    }

    #[test]
    fn layout_emergency_height_9() {
        // Height < 10: banner + rail + footer hidden, composer collapses to
        // one row, transcript gets the leftovers.
        let area = Rect::new(0, 0, 120, 9);
        let plan = compute_layout(area, layout_inputs(true, 6, 4));
        assert!(plan.working_strip.is_none(), "no strip in emergency");
        assert!(plan.permission_banner.is_none(), "no banner in emergency");
        assert!(plan.popup.is_none(), "no popup in emergency");
        assert!(plan.agent_rail.is_none(), "no agent rail in emergency");
        assert!(plan.footer.is_none(), "no footer in emergency");
        assert_eq!(plan.composer.height, 1, "composer collapses to 1 row");
        // transcript gets the remaining 8 rows.
        assert_eq!(plan.transcript.height, 8);
        // Sum of all visible rects must equal area height.
        assert_eq!(plan.transcript.height + plan.composer.height, area.height);
    }

    #[test]
    fn layout_mini_height_15() {
        // Mini: composer 1 row, agent rail hidden, banner + footer keep.
        let area = Rect::new(0, 0, 120, 15);
        let plan = compute_layout(area, layout_inputs(true, 6, 5));
        assert!(plan.permission_banner.is_some(), "banner stays in mini");
        assert!(plan.footer.is_some(), "footer stays in mini");
        assert!(plan.agent_rail.is_none(), "rail hidden in mini");
        assert!(plan.working_strip.is_none(), "strip suppressed in mini");
        assert!(plan.popup.is_none(), "popup hidden in mini");
        assert_eq!(plan.composer.height, 1);
        let banner_h = plan.permission_banner.unwrap().height;
        let footer_h = plan.footer.unwrap().height;
        assert_eq!(banner_h, 1);
        assert_eq!(footer_h, 1);
        // Transcript fills remaining: 15 - 1 (composer) - 1 (banner) - 1 (footer) = 12.
        assert_eq!(plan.transcript.height, 12);
    }

    #[test]
    fn layout_compact_height_22() {
        // Compact: composer 3 rows; agent rail capped at 2; strip visible
        // when active.
        let area = Rect::new(0, 0, 120, 22);
        let plan = compute_layout(area, layout_inputs(true, 6, 8));
        assert_eq!(plan.composer.height, 3, "compact composer is 3 rows");
        let rail = plan.agent_rail.expect("compact keeps the rail");
        assert!(
            rail.height <= 2,
            "compact caps rail at 2 rows (got {})",
            rail.height
        );
        assert!(plan.working_strip.is_some(), "strip visible when active");
        assert!(plan.permission_banner.is_some());
        assert!(plan.footer.is_some());
        // Sum must match area.
        let sum = plan.transcript.height
            + plan.working_strip.unwrap().height
            + plan.permission_banner.unwrap().height
            + plan.popup.map(|r| r.height).unwrap_or(0)
            + plan.composer.height
            + rail.height
            + plan.footer.unwrap().height;
        assert_eq!(sum, area.height);
    }

}
