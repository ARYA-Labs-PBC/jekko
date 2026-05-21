/// Per-frame chrome data passed into [`draw`]. Bundles the cross-cutting
/// state that the renderer needs but doesn't belong on the long argument
/// list (pager overlay, toast stack, runtime-supplied permission/profile
/// labels). Kept as a borrowed snapshot so the runtime owns the master state.
#[derive(Clone, Copy)]
struct DrawContext<'a> {
    /// Active pager overlay (T-INLINE-CLUSTER #2). When `Some`, it takes over
    /// the transcript area entirely; the bottom chrome continues to render
    /// untouched so the user can still see the permission/footer rows.
    pager: Option<&'a PagerState>,
    /// Toast stack rendered into the lower-right corner after the rest of the
    /// chrome paints. Stays empty until /copy, the pager-yank path, or the
    /// mouse selection path push entries (T-INLINE-CLUSTER #2/#3/#4).
    toasts: &'a ToastStack,
    /// Effective permission mode label — sourced from
    /// [`InlineRuntimeOptions::permission_mode`] when supplied
    /// (T-INLINE-CLUSTER #11), else uses the empty hint until the
    /// sandbox enforcer plumbs the resolved policy back (T-SANDBOX-ENF).
    permission_mode_label: &'a str,
    /// Optional profile label — threaded from `--profile` via
    /// [`InlineRuntimeOptions::profile`] (T-INLINE-CLUSTER #1).
    profile: Option<&'a str>,
    /// Compact Jnoccio boot status label for the footer.
    jnoccio_boot_label: Option<&'a str>,
    /// Resolved UI config snapshot. Used by the few renderers that need the
    /// richer provenance than the derived bools above.
    ui_config: Option<&'a jekko_core::config::ui::UiConfig>,
    /// T-INLINE-WAVE3 #3: scaffold field for the background-terminal count.
    /// Sourced from [`InlineRuntimeOptions::background_count`]; remains `0`
    /// until the background-terminal manager exists (T-BG-COUNT-MANAGER).
    background_count: u32,
    /// T-INLINE-WAVE3 #1: pre-resolved motion-enabled flag. Some sibling
    /// components (notably `render_tool_call_live` in
    /// `transcript/inline_cards.rs`) take the bool directly rather than a
    /// `UiConfig`. Resolving once per frame here keeps every renderer in a
    /// single draw consistent (and avoids re-computing the same answer N
    /// times across the chrome).
    motion_enabled: bool,
}

#[allow(clippy::too_many_arguments)]
fn draw(
    terminal: &mut Tty,
    mode: RuntimeMode,
    transcript: &Transcript,
    scroll: &ScrollState,
    composer: &ComposerState,
    in_flight: Option<&InFlight>,
    agent_panel: &AgentPanelState,
    focus: FocusArea,
    index: &FileIndex,
    catalog: &SlashCatalog,
    branch: Option<&str>,
    ctx: &BootContext,
    splash: &SplashState,
    composer_empty_hint: Option<&str>,
    draw_ctx: DrawContext<'_>,
) -> Result<Rect> {
    let mut transcript_rect = Rect::default();
    terminal.draw(|frame| {
        let area = frame.area();
        let desired_popup_height = if composer.slash.active || composer.mention.active {
            6u16
        } else {
            0
        };

        let background_count: usize = draw_ctx.background_count as usize;
        let working_in_flight = in_flight.is_some();
        let working_strip_active = working_in_flight || background_count > 0;

        let build_activity = || -> Option<crate::agents::panel::PanelStreamStatus> {
            in_flight.map(|state| {
                let active_tool = state.latest_tool().map(|tool| {
                    let mut text = tool.name.clone();
                    if let Some(input) = &tool.input {
                        if !input.trim().is_empty() {
                            text = format!("{text}({input})");
                        }
                    }
                    truncate_to_width(&text, area.width.saturating_sub(20) as usize)
                });
                crate::agents::panel::PanelStreamStatus {
                    spinner: Some(state.spinner_glyph(draw_ctx.motion_enabled).to_string()),
                    active_tool,
                    elapsed: Some(elapsed_label(state.started_at.elapsed())),
                }
            })
        };

        let probe_opts = PanelRenderOptions {
            permission_mode_label: Cow::Borrowed("bypass permissions"),
            max_agents: 8,
            max_visible_rows: area.height as usize,
            width: area.width,
            activity: build_activity(),
            compact: false,
            motion_enabled: draw_ctx.motion_enabled,
        };
        let probe_lines = render_agent_panel(agent_panel, Instant::now(), &probe_opts);
        let desired_panel_rows = probe_lines.len() as u16;

        let content_rows: u16 = if splash.visible() {
            SPLASH_ROW_COUNT
        } else {
            transcript.row_count(area.width).min(u16::MAX as usize) as u16
        };

        let plan = compute_layout(
            area,
            LayoutInputs {
                working_strip_active,
                desired_popup_height,
                desired_panel_rows,
                content_rows,
            },
        );
        transcript_rect = plan.transcript;

        let panel_max_rows = plan.agent_rail.map(|r| r.height as usize).unwrap_or(0);
        let panel_lines = if panel_max_rows == 0 {
            Vec::new()
        } else if panel_max_rows >= probe_lines.len() && !plan.compact_agent_rail {
            probe_lines
        } else {
            let panel_opts = PanelRenderOptions {
                permission_mode_label: Cow::Owned(draw_ctx.permission_mode_label.to_string()),
                max_agents: 8,
                max_visible_rows: panel_max_rows,
                width: area.width,
                activity: build_activity(),
                compact: plan.compact_agent_rail,
                motion_enabled: draw_ctx.motion_enabled,
            };
            render_agent_panel(agent_panel, Instant::now(), &panel_opts)
        };

        let permission_mode_label = draw_ctx.permission_mode_label;
        let agent_count = agent_panel.local_running_count();
        let focus_hint = permission_hint_for(focus);
        let footer_info = footer_info_for(ctx, draw_ctx.profile, draw_ctx.jnoccio_boot_label);
        let strip_elapsed = in_flight.map(|s| s.started_at.elapsed());

        let pack_single = plan.pack_status_single_line;

        if mode == RuntimeMode::Fullscreen {
            if let Some(pager_state) = draw_ctx.pager {
                render_pager(frame.buffer_mut(), plan.transcript, pager_state);
            } else if splash.visible() {
                render_splash(
                    frame.buffer_mut(),
                    plan.transcript,
                    splash.elapsed_at(Instant::now()),
                    &splash.ctx,
                    draw_ctx.ui_config,
                );
            } else {
                render_transcript_viewport(
                    frame.buffer_mut(),
                    plan.transcript,
                    transcript,
                    scroll,
                    in_flight,
                    draw_ctx.motion_enabled,
                );
            }
            render_bottom_chrome(
                frame.buffer_mut(),
                &plan,
                BottomChromeArgs {
                    composer,
                    in_flight,
                    index_len: index.len(),
                    catalog,
                    branch,
                    composer_empty_hint,
                    panel_lines: &panel_lines,
                    footer_info: &footer_info,
                    permission_mode_label,
                    agent_count,
                    focus_hint,
                    strip_elapsed,
                    background_count,
                    pack_single,
                    ui_config: draw_ctx.ui_config,
                },
            );
            draw_ctx.toasts.render(area, frame.buffer_mut());
            return;
        }

        // NoAltScreen compatibility path: native scrollback owns transcript.
        render_bottom_chrome(
            frame.buffer_mut(),
            &plan,
            BottomChromeArgs {
                composer,
                in_flight,
                index_len: index.len(),
                catalog,
                branch,
                composer_empty_hint,
                panel_lines: &panel_lines,
                footer_info: &footer_info,
                permission_mode_label,
                agent_count,
                focus_hint,
                strip_elapsed,
                background_count,
                pack_single,
                ui_config: draw_ctx.ui_config,
            },
        );
        draw_ctx.toasts.render(area, frame.buffer_mut());
    })?;
    Ok(transcript_rect)
}

struct BottomChromeArgs<'a> {
    composer: &'a ComposerState,
    in_flight: Option<&'a InFlight>,
    index_len: usize,
    catalog: &'a SlashCatalog,
    branch: Option<&'a str>,
    composer_empty_hint: Option<&'a str>,
    panel_lines: &'a [Line<'static>],
    footer_info: &'a FooterInfo,
    permission_mode_label: &'a str,
    agent_count: usize,
    focus_hint: &'static str,
    strip_elapsed: Option<Duration>,
    background_count: usize,
    pack_single: bool,
    ui_config: Option<&'a jekko_core::config::ui::UiConfig>,
}

fn render_bottom_chrome(buf: &mut Buffer, plan: &LayoutPlan, args: BottomChromeArgs<'_>) {
    if let Some(strip_area) = plan.working_strip {
        render_working_strip(
            buf,
            strip_area,
            args.strip_elapsed,
            args.background_count,
            args.pack_single,
            args.ui_config,
        );
    }
    if let Some(banner_area) = plan.permission_banner {
        render_permission_banner(
            buf,
            banner_area,
            args.permission_mode_label,
            args.agent_count,
            args.focus_hint,
            args.pack_single,
        );
    }
    if let Some(popup_area) = plan.popup {
        render_popups(
            buf,
            popup_area,
            args.composer,
            args.index_len,
            args.catalog,
        );
    }
    ComposerShell {
        input: &args.composer.text,
        streaming: args.in_flight.is_some(),
        streaming_preview: args.in_flight.map(|s| s.buffer.as_str()),
        empty_hint: args.composer_empty_hint,
        branch: args.branch,
    }
    .render(plan.composer, buf);
    if let Some(panel_area) = plan.agent_rail {
        render_agent_panel_lines(buf, panel_area, args.panel_lines);
    }
    if let Some(footer_area) = plan.footer {
        render_footer_status(buf, footer_area, args.footer_info, args.pack_single);
    }
}
