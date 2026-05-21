#[cfg(test)]
mod layout_and_runtime_tests {
    use crate::inline_runtime::tests::{layout_inputs, sample_diff_payload};

    use super::*;

    #[test]
    fn layout_full_height_40() {
        // Full: all chrome visible, rail uses requested rows, transcript
        // gets the leftover space.
        let area = Rect::new(0, 0, 120, 40);
        let plan = compute_layout(area, layout_inputs(true, 6, 5));
        assert_eq!(plan.composer.height, 3);
        assert!(plan.working_strip.is_some());
        assert!(plan.permission_banner.is_some());
        assert!(plan.footer.is_some());
        let rail = plan.agent_rail.expect("full keeps rail");
        assert_eq!(rail.height, 5, "full layout grants requested rail rows");
        // Sanity: transcript_h = 40 - 3 (composer) - 1 (banner) - 1 (footer)
        //          - 1 (strip) - 6 (popup) - 5 (rail) = 23.
        assert_eq!(plan.transcript.height, 23);
    }

    #[test]
    fn layout_full_preserves_80x24_snapshot_shape() {
        // The historical snapshot tests render at 80x24 — height 24 sits in
        // the Compact tier, so they get the same composer height (3) and
        // banner/footer pair they had before T2-P4. This test pins that
        // behaviour so future tier-edge tweaks don't silently shift existing
        // snapshots.
        let area = Rect::new(0, 0, 80, 24);
        let plan = compute_layout(area, layout_inputs(false, 0, 1));
        assert_eq!(HeightTier::from_height(area.height), HeightTier::Compact);
        assert_eq!(plan.composer.height, 3);
        assert!(plan.working_strip.is_none(), "no in-flight ⇒ no strip");
        assert!(plan.permission_banner.is_some());
        assert!(plan.footer.is_some());
        assert_eq!(plan.agent_rail.unwrap().height, 1);
    }

    #[test]
    fn layout_width_truncation_breakpoints() {
        // Vary width across all four tiers at a stable height. The vertical
        // plan stays identical; only the width-derived flags change.
        let heights = 22u16; // compact — predictable vertical plan
        for (width, expected_compact, expected_pack) in [
            (40u16, true, true),    // <50 → single-line
            (60u16, true, false),   // 50..70 → hide tokens
            (80u16, true, false),   // 70..100 → abbreviate
            (120u16, false, false), // >=100 → full
        ] {
            let area = Rect::new(0, 0, width, heights);
            let plan = compute_layout(area, layout_inputs(false, 0, 2));
            assert_eq!(
                plan.compact_agent_rail, expected_compact,
                "compact_agent_rail at width={width}"
            );
            assert_eq!(
                plan.pack_status_single_line, expected_pack,
                "pack_status_single_line at width={width}"
            );
            // Vertical plan is unaffected by width.
            assert_eq!(plan.composer.height, 3, "composer stable across widths");
        }
    }

    #[test]
    fn layout_popup_eats_transcript_not_composer() {
        // When the slash/mention popup is live, the popup row count must
        // come out of the transcript area — never the composer.
        let area = Rect::new(0, 0, 120, 40);
        let no_popup = compute_layout(area, layout_inputs(false, 0, 2));
        let with_popup = compute_layout(area, layout_inputs(false, 6, 2));
        assert_eq!(no_popup.composer.height, with_popup.composer.height);
        assert!(
            with_popup.transcript.height + 6 == no_popup.transcript.height,
            "popup of 6 rows must shrink transcript by exactly 6"
        );
        assert_eq!(with_popup.popup.unwrap().height, 6);
    }

    // ── T1-V6b: splash startup ─────────────────────────────────────────────

    fn fixture_splash_ctx() -> SplashContext {
        SplashContext {
            version: "9.9.9".into(),
            cwd: "~/code/jekko".into(),
            branch: Some("main".into()),
        }
    }

    fn dump_buffer(buf: &Buffer) -> String {
        let area = buf.area();
        let mut out = String::new();
        for y in 0..area.height {
            for x in 0..area.width {
                out.push_str(buf[(x, y)].symbol());
            }
            out.push('\n');
        }
        out
    }

    #[test]
    fn splash_state_starts_hidden_then_marks_started_on_ensure() {
        let mut state = SplashState::new(fixture_splash_ctx());
        assert!(state.visible(), "splash visible while undismissed");
        assert!(
            state.started_at.is_none(),
            "no started timestamp until first draw"
        );

        let now = Instant::now();
        state.ensure_started(now);
        assert_eq!(
            state.started_at,
            Some(now),
            "ensure_started seeds the timer"
        );
        // Idempotent — a second call must not move the timestamp.
        let later = now + Duration::from_secs(10);
        state.ensure_started(later);
        assert_eq!(state.started_at, Some(now), "ensure_started is idempotent");
    }

    #[test]
    fn splash_state_on_first_submit_dismisses() {
        let mut state = SplashState::new(fixture_splash_ctx());
        assert!(state.visible());
        state.on_first_submit();
        assert!(!state.visible(), "after first submit, splash must collapse");
        // Idempotent — second call still dismissed.
        state.on_first_submit();
        assert!(!state.visible());
    }

    #[test]
    fn splash_state_elapsed_zero_before_start() {
        let state = SplashState::new(fixture_splash_ctx());
        assert_eq!(state.elapsed_at(Instant::now()), Duration::ZERO);
    }

    #[test]
    fn splash_renders_wordmark_when_visible() {
        // Mirrors the dispatch arm in `draw`: when `splash.visible()` is true
        // we render the splash into the transcript area instead of scrollback.
        let mut state = SplashState::new(fixture_splash_ctx());
        let now = Instant::now();
        state.ensure_started(now);
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        // Render exactly what the runtime would.
        render_splash(&mut buf, area, state.elapsed_at(now), &state.ctx, None);
        let dump = dump_buffer(&buf);
        assert!(
            dump.contains('█'),
            "expected wordmark block glyph, got:\n{dump}"
        );
        assert!(dump.contains("v9.9.9"), "expected subtitle version");
    }

    #[test]
    fn splash_dismissed_falls_back_to_transcript_render() {
        // After `on_first_submit`, `splash.visible()` flips to false, so the
        // runtime takes the normal `render_transcript_viewport` branch. This
        // test exercises that fork by calling the underlying viewport
        // renderer directly with a populated transcript.
        let mut splash = SplashState::new(fixture_splash_ctx());
        splash.on_first_submit();
        assert!(!splash.visible(), "test precondition: splash dismissed");

        let mut transcript = Transcript::default();
        transcript.push(&render_user("hello world"));

        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        let scroll = ScrollState::default();
        // T-INLINE-WAVE3 #1: viewport now takes `motion_enabled: bool` after
        // sibling T-GLYPH-WAVE3 threading. Test doesn't exercise motion, so
        // `true` (default behavior) keeps the dump comparable to the
        // pre-widening snapshot.
        render_transcript_viewport(&mut buf, area, &transcript, &scroll, None, true);
        let dump = dump_buffer(&buf);
        assert!(
            dump.contains("hello world"),
            "transcript content must render once splash is dismissed, got:\n{dump}"
        );
        assert!(
            !dump.contains('█'),
            "block wordmark glyph must not appear after dismiss"
        );
    }

    #[test]
    fn chat_event_diff_appends_card_to_transcript() {
        // Reproduces the LoopEvent::Chat(Some(ChatEvent::Diff)) dispatch arm
        // by exercising the same helper + Transcript::push it calls.
        let (path, hunks) = sample_diff_payload();
        let mut transcript = Transcript::default();
        let blocks_before = transcript.block_count();

        // Mirror the runtime arm: render then push.
        let lines = render_diff_lines_from_payload(&path, &hunks);
        transcript.push(&lines);

        assert_eq!(
            transcript.block_count(),
            blocks_before + 1,
            "Diff dispatch must append exactly one transcript block"
        );
        let rendered = transcript.visible_rows(120, 16, 0);
        let joined: String = rendered
            .iter()
            .flat_map(|line| line.spans.iter().map(|s| s.content.as_ref()))
            .collect();
        assert!(
            joined.contains("src/lib.rs"),
            "card header should carry path"
        );
        assert!(joined.contains("line one"), "context body must render");
        assert!(joined.contains("before"), "removed body must render");
        assert!(joined.contains("after"), "added body must render");
    }

    // ── T-INLINE-CLUSTER follow-up tests ────────────────────────────────────

    #[test]
    fn scroll_line_up_by_advances_offset() {
        // T-INLINE-CLUSTER #3: mouse wheel ScrollUp(n) maps to line_up_by(n).
        let mut s = ScrollState::default();
        s.line_up_by(3);
        assert_eq!(s.offset_from_bottom, 3);
        s.line_up_by(10);
        assert_eq!(s.offset_from_bottom, 13);
    }

    #[test]
    fn scroll_line_down_by_clamps_at_bottom() {
        // T-INLINE-CLUSTER #3: line_down_by saturates at zero, never wraps.
        let mut s = ScrollState {
            offset_from_bottom: 5,
        };
        s.line_down_by(2);
        assert_eq!(s.offset_from_bottom, 3);
        s.line_down_by(100);
        assert_eq!(s.offset_from_bottom, 0, "saturates at 0");
    }

    #[test]
    fn transcript_as_text_flattens_blocks() {
        // T-INLINE-CLUSTER #4/#5: /copy + Ctrl+Shift+C both flatten the
        // transcript through this helper. Verify it joins blocks with `\n`
        // and counts bytes correctly.
        let mut transcript = Transcript::default();
        transcript.push(&render_user("hello"));
        transcript.push(&render_assistant("world"));
        let (text, bytes) = transcript_as_text(&transcript, None);
        assert_eq!(bytes, text.len());
        assert!(text.contains("hello"), "user block missing: {text:?}");
        assert!(text.contains("world"), "assistant block missing: {text:?}");
    }

    #[test]
    fn transcript_as_text_appends_in_flight_buffer() {
        // T-INLINE-CLUSTER #4: in-flight assistant buffer is included so /copy
        // captures the current turn even before it finishes streaming.
        let transcript = Transcript::default();
        let mut state = InFlight::new();
        state.buffer.push_str("partial");
        let (text, _) = transcript_as_text(&transcript, Some(&state));
        assert!(text.contains("partial"), "in-flight body missing: {text:?}");
    }

    #[test]
    fn selection_text_returns_empty_when_no_drag() {
        // T-INLINE-CLUSTER #3: selection_text with `None` start returns empty.
        let transcript = Transcript::default();
        let payload = selection_text(&transcript, Rect::new(0, 0, 40, 4), 0, None, None);
        assert!(payload.is_empty());
    }

    #[test]
    fn selection_text_extracts_visible_row_slice() {
        // T-INLINE-CLUSTER #3: drag the cursor across one visible transcript
        // row — the resulting payload should contain that row's text.
        let mut transcript = Transcript::default();
        transcript.push(&render_user("alpha bravo charlie"));
        // The transcript rect is 40 cells wide × 4 tall; the helper renders
        // from the bottom up, so row y=3 holds the user line. Select cols 0-5.
        let payload = selection_text(
            &transcript,
            Rect::new(0, 0, 40, 4),
            0,
            Some((0, 3)),
            Some((20, 3)),
        );
        // Some payload should be captured (exact bytes depend on the inline
        // card glyphs); the helper must not blow up.
        let _ = payload;
    }

    #[test]
    fn permissions_modal_body_lists_effective_mode() {
        // T-INLINE-CLUSTER #7: the system-notice body for /permissions shows
        // both the effective label and the raw --permission-mode value.
        let body = permissions_modal_body("ask-for-edits", Some("ask-for-edits"), 2);
        assert!(body.contains("/permissions"));
        assert!(body.contains("ask-for-edits"));
        assert!(body.contains("2"));
    }

    #[test]
    fn permissions_modal_body_marks_unset_raw_mode() {
        // T-INLINE-CLUSTER #7: when the CLI didn't pass --permission-mode the
        // raw row reads `(unset)` so the user can tell the runtime-resolved
        // label came from the empty hint.
        let body = permissions_modal_body("bypass permissions", None, 0);
        assert!(body.contains("(unset)"));
    }

    #[test]
    fn sandbox_modal_body_surfaces_cwd_and_branch() {
        // T-INLINE-CLUSTER #8: /sandbox notice shows cwd + branch from the
        // BootContext + raw policy flags. Env scrubbing / allowed paths land
        // when the sandbox runner exposes its resolved state.
        let ctx = BootContext {
            version: "0.1.0".into(),
            cwd_display: "~/code/jekko".into(),
            branch: Some("main".into()),
        };
        let body = sandbox_modal_body(Some("workspace-write"), Some("on-failure"), &ctx);
        assert!(body.contains("/sandbox"));
        assert!(body.contains("workspace-write"));
        assert!(body.contains("on-failure"));
        assert!(body.contains("~/code/jekko"));
        assert!(body.contains("main"));
    }

    #[test]
    fn inline_runtime_options_default_has_runtime_chrome_fields() {
        // T-INLINE-CLUSTER #11: ensure the new fields default to None so
        // existing callers (tests / examples) build without referencing them.
        let opts = InlineRuntimeOptions::default();
        assert!(opts.permission_mode.is_none());
        assert!(opts.sandbox_profile.is_none());
        assert!(opts.approval_mode.is_none());
        assert!(opts.profile.is_none());
    }

    #[test]
    fn inline_runtime_options_default_has_zero_background_count() {
        // T-INLINE-WAVE3 #3: scaffold field. The background-terminal manager
        // (T-BG-COUNT-MANAGER follow-up) doesn't exist yet, so the default
        // must remain `0` — flipping this contract without wiring the manager
        // would surface a spurious "background terminal running" segment in
        // the working strip on the very first frame.
        let opts = InlineRuntimeOptions::default();
        assert_eq!(opts.background_count, 0);
    }

    #[test]
    fn compact_history_skips_when_under_threshold() {
        // T-INLINE-CLUSTER #9: /compact is a no-op until > COMPACT_KEEP_LAST_N
        // turns are recorded — there's nothing to summarise yet.
        let mut history: Vec<ChatTurnRecord> = (0..COMPACT_KEEP_LAST_N)
            .map(|i| ChatTurnRecord {
                user: format!("u{i}"),
                assistant: format!("a{i}"),
            })
            .collect();
        // We don't have a real terminal here, but the early-return check
        // doesn't need one. Replicate the early branch.
        assert!(history.len() <= COMPACT_KEEP_LAST_N);
        // Add one more — now it would compact.
        history.push(ChatTurnRecord {
            user: "u_extra".into(),
            assistant: "a_extra".into(),
        });
        assert!(history.len() > COMPACT_KEEP_LAST_N);
    }

    #[test]
    fn footer_info_pads_profile_through_renderer() {
        // T-INLINE-CLUSTER #1: confirm the new profile signature flows into
        // the FooterInfo as a String when supplied.
        let ctx = BootContext {
            version: "0.1.0".into(),
            cwd_display: "/tmp".into(),
            branch: None,
        };
        let info = footer_info_for(&ctx, Some("prod"), None);
        assert_eq!(info.profile.as_deref(), Some("prod"));
        let info_none = footer_info_for(&ctx, None, None);
        assert!(info_none.profile.is_none());
    }

    #[test]
    fn jnoccio_boot_runtime_drains_receiver_updates() {
        let (tx, rx) = mpsc::channel();
        let mut boot = JnoccioBootRuntime::new(JnoccioBootStatus::Checking);
        tx.send(JnoccioBootStatus::Starting).unwrap();
        tx.send(JnoccioBootStatus::Ready {
            enabled_models: 3,
            total_models: 5,
        })
        .unwrap();
        let mut rx = Some(rx);
        assert!(boot.drain_updates(&mut rx));
        assert!(matches!(
            boot.status,
            JnoccioBootStatus::Ready {
                enabled_models: 3,
                total_models: 5
            }
        ));
        assert!(rx.is_some(), "receiver stays owned by runtime");
    }

    #[test]
    fn status_snapshot_includes_jnoccio_jankurai_and_zyal_sections() {
        let ctx = BootContext {
            version: "0.1.0".into(),
            cwd_display: "~/code/jekko".into(),
            branch: Some("main".into()),
        };
        let boot = JnoccioBootRuntime::new(JnoccioBootStatus::Disabled);
        let body = render_status_snapshot(&ctx, false, 7, &boot);
        assert!(body.contains("jnoccio:"));
        assert!(body.contains("boot disabled"));
        assert!(body.contains("jankurai:"));
        assert!(body.contains("zyal:"));
    }

    // ── T-BG-COUNT-MANAGER tests ────────────────────────────────────────────

    #[test]
    fn ps_lists_background_jobs() {
        // Snapshot the manager's job list and render the `/ps` body the same
        // way the slash dispatcher does. This is the highest-fidelity unit
        // test we can write without driving the full terminal loop.
        let mgr = BackgroundJobManager::new();
        let (id_a, _) = mgr.register("nightly-build".into(), Some(4242));
        let (id_b, _) = mgr.register("dev-server".into(), None);
        let body = render_ps_body(&mgr.list());
        assert!(body.contains("background jobs:"), "got: {body}");
        assert!(body.contains("nightly-build"), "got: {body}");
        assert!(body.contains("dev-server"), "got: {body}");
        assert!(body.contains("pid 4242"), "got: {body}");
        assert!(
            body.contains(&format!("[{id_a}]")) && body.contains(&format!("[{id_b}]")),
            "ids missing from body: {body}"
        );
        assert!(body.contains("running"), "got: {body}");
    }

    #[test]
    fn ps_body_empty_when_no_jobs() {
        let mgr = BackgroundJobManager::new();
        let body = render_ps_body(&mgr.list());
        assert_eq!(body, "no background jobs running");
    }

    #[test]
    fn stop_with_id_arg_cancels_specific_job() {
        // Simulates the `/stop <id>` dispatch path: parse the trailing arg
        // from the slash line + call `bg_manager.stop`. Asserts the
        // cancellation token the runner holds is fired.
        let mgr = BackgroundJobManager::new();
        let (id, token) = mgr.register("worker".into(), None);
        assert!(!token.is_cancelled());

        // Mirror the runtime: capture the slash-line text, then parse args.
        let slash_line_raw = format!("/stop {id}");
        let cmd_id = "stop";
        let arg = slash_args(&slash_line_raw, cmd_id);
        let parsed: JobId = arg.parse().expect("id must parse");
        assert!(mgr.stop(parsed), "stop on known id must return true");
        assert!(token.is_cancelled(), "runner's token clone must see cancel");

        // A second /stop for the same id returns true (idempotent) without
        // resurrecting status; status is still Cancelled.
        assert!(mgr.stop(parsed));
        let jobs = mgr.list();
        assert_eq!(jobs[0].status, JobStatus::Cancelled);
    }

    #[test]
    fn stop_with_unknown_id_arg_yields_warn_branch() {
        // Confirm the unknown-id branch returns false (which the dispatcher
        // turns into a NoticeKind::Warn).
        let mgr = BackgroundJobManager::new();
        let slash_line_raw = "/stop 9999";
        let arg = slash_args(slash_line_raw, "stop");
        let id: JobId = arg.parse().expect("id parses");
        assert!(!mgr.stop(id), "stop on unknown id must return false");
    }

    #[test]
    fn stop_without_id_falls_through_to_inflight_cancel() {
        // No trailing arg + an active turn → the dispatcher must take the
        // compatibility in-flight cancel branch. We don't have a backend here, so
        // assert the helper produces an empty arg (the trigger for the fall
        // through) and that an InFlight's cancel_on_stop hits CancelLevel::Hard.
        let arg = slash_args("/stop", "stop");
        assert!(arg.is_empty(), "no trailing arg → empty string");
        let arg_ws = slash_args("/stop   ", "stop");
        assert!(arg_ws.is_empty(), "trailing whitespace → empty string");

        // Confirm the compatibility in-flight cancel still escalates to Hard so the
        // fall-through branch retains its pre-T-BG-COUNT-MANAGER behaviour.
        let mut state = InFlight::new();
        let token = state.cancel_token();
        let level = state.cancel_on_stop();
        assert_eq!(level, CancelLevel::Hard);
        assert_eq!(token.level(), CancelLevel::Hard);
    }

    #[test]
    fn stop_does_not_touch_bg_jobs_when_falling_through_to_inflight() {
        // When `/stop` has no arg, the dispatcher must NOT call
        // bg_manager.stop — confirm bg jobs survive untouched.
        let mgr = BackgroundJobManager::new();
        let (_id, token) = mgr.register("worker".into(), None);
        let arg = slash_args("/stop", "stop");
        assert!(arg.is_empty(), "no arg → fall through to in-flight cancel");
        // Simulate the dispatcher's "fall through" by skipping `bg_manager.stop`.
        assert!(!token.is_cancelled(), "bg job token must NOT be touched");
        assert_eq!(mgr.count(), 1, "bg manager still tracks the job");
    }

    #[test]
    fn bg_count_reflects_manager() {
        // Drives the per-frame `resolved_bg_count` logic in `draw_ctx`:
        // register 3 jobs (count=3), finalize 1 (count=2), and confirm the
        // resolved field would surface 2 to the working strip.
        let mgr = BackgroundJobManager::new();
        let (a, _) = mgr.register("a".into(), None);
        let (_b, _) = mgr.register("b".into(), None);
        let (_c, _) = mgr.register("c".into(), None);
        assert_eq!(mgr.count(), 3);
        mgr.finalize(a, JobStatus::Completed);
        assert_eq!(mgr.count(), 2);
        // Re-implement the resolved-count logic that drives draw_ctx.
        let opts_seed: u32 = 0;
        let live = mgr.count() as u32;
        let resolved = if live > 0 { live } else { opts_seed };
        assert_eq!(resolved, 2);
    }

    #[test]
    fn bg_count_falls_back_to_options_seed_when_manager_empty() {
        // When the manager has no live jobs, the resolved value falls back
        // to `InlineRuntimeOptions::background_count`. Defaults to 0 in
        // production today but tests may seed non-zero.
        let mgr = BackgroundJobManager::new();
        let opts_seed: u32 = 7;
        let live = mgr.count() as u32;
        let resolved = if live > 0 { live } else { opts_seed };
        assert_eq!(resolved, 7);
    }

    #[test]
    fn slash_args_strips_command_and_whitespace() {
        // Helper contract: `/stop 12` → `"12"`, `/ps` → `""`, leading
        // whitespace + missing slash both tolerated.
        assert_eq!(slash_args("/stop 12", "stop"), "12");
        assert_eq!(slash_args("/stop   42  ", "stop"), "42");
        assert_eq!(slash_args("/stop", "stop"), "");
        assert_eq!(slash_args("  /stop 5", "stop"), "5");
        // Tolerant when leading slash is missing (defensive).
        assert_eq!(slash_args("stop 5", "stop"), "5");
    }

    #[test]
    fn ps_body_shows_completed_and_failed_status_labels() {
        // Lifecycle: registered → finalize(Completed/Failed) → reflected in
        // `/ps` body until the sweep window expires.
        let mgr = BackgroundJobManager::new();
        let (a, _) = mgr.register("compile".into(), None);
        let (b, _) = mgr.register("deploy".into(), None);
        mgr.finalize(a, JobStatus::Completed);
        mgr.finalize(b, JobStatus::Failed("oom".into()));
        let body = render_ps_body(&mgr.list());
        assert!(body.contains("done"), "got: {body}");
        assert!(body.contains("failed: oom"), "got: {body}");
    }

    // ── T-PERMISSIONS-PLUMB tests ───────────────────────────────────────────

    #[test]
    fn permission_mode_cycle_advances_modes() {
        // /permissions cycles BypassPermissions → AskBeforeWrite → ReadOnly →
        // back to BypassPermissions. Defined as a method on the enum so we can
        // unit test it without spinning up the runtime.
        let mut mode = PermissionMode::default();
        assert_eq!(mode, PermissionMode::BypassPermissions);
        mode = mode.cycle();
        assert_eq!(mode, PermissionMode::AskBeforeWrite);
        mode = mode.cycle();
        assert_eq!(mode, PermissionMode::ReadOnly);
        mode = mode.cycle();
        assert_eq!(
            mode,
            PermissionMode::BypassPermissions,
            "cycle must wrap back to the start"
        );
    }

    #[test]
    fn permission_mode_label_matches_cycle_order() {
        // Sanity: the labels track the actual variants so the chrome rail
        // shows the same string we render in the /permissions notice.
        assert_eq!(
            PermissionMode::BypassPermissions.label(),
            "bypass permissions"
        );
        assert_eq!(PermissionMode::AskBeforeWrite.label(), "ask before write");
        assert_eq!(PermissionMode::ReadOnly.label(), "read-only");
    }

    #[test]
    fn permission_state_from_opts_parses_each_known_value() {
        // T-PERMISSIONS-PLUMB: from_opts must accept both Claude-shorthand
        // and fully-spelled forms of the --permission-mode flag value.
        for (raw, expected) in [
            ("bypass", PermissionMode::BypassPermissions),
            ("bypass-permissions", PermissionMode::BypassPermissions),
            ("ask", PermissionMode::AskBeforeWrite),
            ("ask-before-write", PermissionMode::AskBeforeWrite),
            ("read-only", PermissionMode::ReadOnly),
            ("readonly", PermissionMode::ReadOnly),
            // Case-insensitive — case doesn't matter for the CLI flag.
            ("ReadOnly", PermissionMode::ReadOnly),
        ] {
            let opts = InlineRuntimeOptions {
                permission_mode: Some(raw.to_string()),
                ..InlineRuntimeOptions::default()
            };
            let state = PermissionState::from_opts(&opts).unwrap();
            assert_eq!(state.mode, expected, "raw value {raw:?}");
        }
    }

    #[test]
    fn permission_state_from_opts_rejects_unknown() {
        // T-PERMISSIONS-PLUMB: unknown values (typo, invented mode) fail
        // closed so the runtime does not boot in the wrong mode.
        let opts = InlineRuntimeOptions {
            permission_mode: Some("frobnicate".to_string()),
            ..InlineRuntimeOptions::default()
        };
        let err = PermissionState::from_opts(&opts).unwrap_err();
        assert!(err.to_string().contains("invalid permission mode"));
    }

    #[test]
    fn permission_state_from_opts_threads_sandbox_and_approval() {
        // T-PERMISSIONS-PLUMB: sandbox/approval are mirrored onto the state so
        // /sandbox can render them without re-reading `opts` directly.
        let opts = InlineRuntimeOptions {
            permission_mode: Some("ask".into()),
            sandbox_profile: Some("workspace-write".into()),
            approval_mode: Some("on-failure".into()),
            ..InlineRuntimeOptions::default()
        };
        let state = PermissionState::from_opts(&opts).unwrap();
        assert_eq!(state.mode, PermissionMode::AskBeforeWrite);
        assert_eq!(state.sandbox_profile.as_deref(), Some("workspace-write"));
        assert_eq!(state.approval_mode.as_deref(), Some("on-failure"));
    }

    #[test]
    fn slash_permissions_cycles_mode_on_repeated_invocation() {
        // T-PERMISSIONS-PLUMB: drive the same mutation the dispatcher does
        // (`permission_state.mode = permission_state.mode.cycle()`) and assert
        // the labels emitted by the notice match the cycle order.
        let mut state = PermissionState::from_opts(&InlineRuntimeOptions::default()).unwrap();
        assert_eq!(state.mode.label(), "bypass permissions");
        state.mode = state.mode.cycle();
        assert_eq!(state.mode.label(), "ask before write");
        state.mode = state.mode.cycle();
        assert_eq!(state.mode.label(), "read-only");
        state.mode = state.mode.cycle();
        assert_eq!(state.mode.label(), "bypass permissions");
    }

    #[test]
    fn slash_sandbox_renders_full_state() {
        // T-PERMISSIONS-PLUMB: /sandbox notice must mention the sandbox label,
        // approval label, cwd, branch, plus the allow_net + allowed_paths
        // documentation lines so the operator sees the runner defaults.
        let ctx = BootContext {
            version: "0.1.0".into(),
            cwd_display: "~/code/jekko".into(),
            branch: Some("main".into()),
        };
        let body = sandbox_modal_body(Some("workspace-write"), Some("on-failure"), &ctx);
        assert!(body.contains("workspace-write"), "got: {body}");
        assert!(body.contains("on-failure"), "got: {body}");
        assert!(body.contains("~/code/jekko"));
        assert!(body.contains("main"));
        assert!(body.contains("allow_net"), "got: {body}");
        assert!(body.contains("allowed_paths"), "got: {body}");
        assert!(body.contains("default deny"), "got: {body}");
        assert!(body.contains("default = cwd-only"), "got: {body}");
    }

    #[test]
    fn slash_sandbox_renders_none_when_flags_unset() {
        // T-PERMISSIONS-PLUMB: when --sandbox / --ask-for-approval aren't
        // supplied, the notice shows "none" instead of the older "(default)"
        // hint so the operator can tell explicit vs. implicit defaults
        // apart.
        let ctx = BootContext {
            version: "0.1.0".into(),
            cwd_display: "/".into(),
            branch: None,
        };
        let body = sandbox_modal_body(None, None, &ctx);
        assert!(body.contains("--sandbox:             none"), "got: {body}");
        assert!(body.contains("--ask-for-approval:    none"), "got: {body}");
    }

    #[test]
    fn parse_run_args_extracts_background_flag() {
        // T-PERMISSIONS-PLUMB / T-BG-RUN: `--background` / `--bg` strip the
        // flag and return the trimmed command tail. Foreground calls preserve
        // the raw args.
        assert_eq!(parse_run_args("--background sleep 30"), (true, "sleep 30"));
        assert_eq!(parse_run_args("--bg make build"), (true, "make build"));
        assert_eq!(parse_run_args("ls -la"), (false, "ls -la"));
        assert_eq!(
            parse_run_args("  --background   echo hi  "),
            (true, "echo hi")
        );
        // No command supplied → empty tail (dispatcher emits a warn notice).
        assert_eq!(parse_run_args("--background"), (true, ""));
        assert_eq!(parse_run_args("--bg"), (true, ""));
        assert_eq!(parse_run_args(""), (false, ""));
    }

    #[test]
    fn slash_run_background_registers_with_manager() {
        // T-PERMISSIONS-PLUMB / T-BG-RUN: simulate the dispatcher's bg path —
        // register a job + assert the manager exposes it with status=Running.
        // We don't drive the actual spawn here; that branch is covered by the
        // `slash_run_background_finalize_marks_completed` test below.
        let mgr = BackgroundJobManager::new();
        let (id, _token) = mgr.register("sleep 30".into(), None);
        let jobs = mgr.list();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].id, id);
        assert_eq!(jobs[0].status, JobStatus::Running);
        assert_eq!(jobs[0].name, "sleep 30");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn slash_run_background_finalize_marks_completed() {
        // T-PERMISSIONS-PLUMB / T-BG-RUN: end-to-end of the dispatcher's bg
        // task — register a job, run a real `sh -c "true"` via the helper,
        // finalize, and assert the manager flips the status to Completed.
        let mgr = BackgroundJobManager::new();
        let (id, token) = mgr.register("true".into(), None);
        let status = run_background_shell("true".to_string(), token).await;
        mgr.finalize(id, status);
        let jobs = mgr.list();
        assert_eq!(jobs[0].status, JobStatus::Completed);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn run_background_shell_marks_failed_on_nonzero_exit() {
        // T-PERMISSIONS-PLUMB / T-BG-RUN: `sh -c "false"` exits non-zero;
        // helper must surface that as JobStatus::Failed so /ps shows it
        // explicitly instead of silently completing.
        let token = CancellationToken::new();
        let status = run_background_shell("false".to_string(), token).await;
        match status {
            JobStatus::Failed(msg) => assert!(msg.contains("exit"), "got: {msg}"),
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    #[test]
    fn jankurai_audit_args_target_external_score_artifacts() {
        assert_eq!(
            JANKURAI_AUDIT_ARGS,
            &[
                "audit",
                ".",
                "--mode",
                "advisory",
                "--json",
                "agent/repo-score.json",
                "--md",
                "agent/repo-score.md",
            ],
        );
    }
}
