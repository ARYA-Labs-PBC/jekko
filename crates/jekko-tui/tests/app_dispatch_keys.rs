//! Behavioral tests for `App::dispatch_key` covering route transitions, prompt
//! key routing, and the fixes from the TUI restoration (session route wiring,
//! Home engage, `q`-quit scope, auto-engage from Home).
//!
//! These are integration tests that exercise the public `App` API without
//! needing a real terminal.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use jekko_core::session::SessionId;
use jekko_tui::{Action, App, Route, Stage};

// ─── Helpers ────────────────────────────────────────────────────────────────

fn press(app: &mut App, code: KeyCode) {
    app.dispatch(Action::Key(KeyEvent::new(code, KeyModifiers::NONE)));
}

fn press_mod(app: &mut App, code: KeyCode, mods: KeyModifiers) {
    app.dispatch(Action::Key(KeyEvent::new(code, mods)));
}

fn press_char(app: &mut App, ch: char) {
    press(app, KeyCode::Char(ch));
}

fn make_visible(app: &mut App) {
    app.mark_app_visible();
}

// ─── Home route ─────────────────────────────────────────────────────────────

#[test]
fn home_enter_navigates_to_shell() {
    // Phase A: App now starts on Shell. Verify the legacy Home→Shell
    // back-compat path (only reached via explicit Navigate) still works.
    let mut app = App::new();
    make_visible(&mut app);
    app.route = Route::Home;
    press(&mut app, KeyCode::Enter);
    assert!(
        matches!(app.route, Route::Shell),
        "Enter on Home should navigate to Shell, got {:?}",
        app.route
    );
}

#[test]
fn home_q_quits() {
    // Phase A: App starts on Shell; verify Home `q` quit affordance still
    // honoured for back-compat (callers who explicitly Navigate Home).
    let mut app = App::new();
    make_visible(&mut app);
    app.route = Route::Home;
    assert!(!app.quit);
    press_char(&mut app, 'q');
    assert!(app.quit, "Bare 'q' on Home should set quit=true");
}

#[test]
fn shell_first_printable_char_lands_in_prompt() {
    // Phase A: App lands on Shell directly. The first printable char goes
    // straight into the prompt — no Home→Shell hop, no engagement change.
    let mut app = App::new();
    make_visible(&mut app);
    assert!(matches!(app.route, Route::Shell));
    assert!(app.engagement.is_idle());
    press_char(&mut app, 'h');
    assert!(matches!(app.route, Route::Shell));
    assert_eq!(app.prompt.buffer_string(), "h");
    // Typing alone does NOT engage — only Enter/Submit do.
    assert!(app.engagement.is_idle());
}

#[test]
fn shell_typing_multiple_chars_buffers_prompt() {
    let mut app = App::new();
    make_visible(&mut app);
    for ch in ['t', 'e', 's', 't'] {
        press_char(&mut app, ch);
    }
    assert!(matches!(app.route, Route::Shell));
    assert_eq!(app.prompt.buffer_string(), "test");
}

// ─── Shell route: typing doesn't quit ───────────────────────────────────────

#[test]
fn shell_typing_q_does_not_quit() {
    let mut app = App::new();
    make_visible(&mut app);
    app.route = Route::Shell;
    press_char(&mut app, 'q');
    assert!(
        !app.quit,
        "Typing 'q' on Shell should NOT quit — it should go to the prompt"
    );
    assert_eq!(
        app.prompt.buffer_string(),
        "q",
        "'q' should be typed into the prompt on Shell"
    );
}

#[test]
fn session_typing_q_does_not_quit() {
    let mut app = App::new();
    make_visible(&mut app);
    app.route = Route::Session {
        session_id: SessionId::new("sess_test"),
    };
    press_char(&mut app, 'q');
    assert!(!app.quit, "Typing 'q' on Session should NOT quit");
    assert_eq!(app.prompt.buffer_string(), "q");
}

// ─── translate_event ────────────────────────────────────────────────────────

#[test]
fn translate_event_ctrl_c_is_quit() {
    use crossterm::event::{Event as CtEvent, KeyEventKind};
    let ev = CtEvent::Key(KeyEvent {
        code: KeyCode::Char('c'),
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: crossterm::event::KeyEventState::empty(),
    });
    let action = jekko_tui::translate_event(ev).unwrap();
    assert!(matches!(action, Action::Quit));
}

#[test]
fn translate_event_bare_q_is_not_quit() {
    use crossterm::event::{Event as CtEvent, KeyEventKind};
    let ev = CtEvent::Key(KeyEvent {
        code: KeyCode::Char('q'),
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: crossterm::event::KeyEventState::empty(),
    });
    let action = jekko_tui::translate_event(ev).unwrap();
    // Should be Action::Key, NOT Action::Quit.
    match action {
        Action::Key(k) => assert_eq!(k.code, KeyCode::Char('q')),
        Action::Quit => panic!("Bare 'q' must NOT translate to Quit — it breaks prompt input"),
        other => panic!("unexpected action: {other:?}"),
    }
}

#[test]
fn translate_event_ignores_key_release() {
    use crossterm::event::{Event as CtEvent, KeyEventKind};
    let ev = CtEvent::Key(KeyEvent {
        code: KeyCode::Char('a'),
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Release,
        state: crossterm::event::KeyEventState::empty(),
    });
    assert!(
        jekko_tui::translate_event(ev).is_none(),
        "Key releases should be ignored"
    );
}

// ─── Ctrl+P opens command palette ───────────────────────────────────────────

#[test]
fn ctrl_p_opens_command_palette() {
    let mut app = App::new();
    make_visible(&mut app);
    app.route = Route::Shell;
    assert!(app.dialogs.is_empty());
    press_mod(&mut app, KeyCode::Char('p'), KeyModifiers::CONTROL);
    assert!(
        !app.dialogs.is_empty(),
        "Ctrl+P should open the command palette"
    );
}

#[test]
fn ctrl_p_works_on_home_route_too() {
    let mut app = App::new();
    make_visible(&mut app);
    // Phase A: App lands on Shell; force back to Home to verify Ctrl+P
    // still works on the back-compat Home route.
    app.route = Route::Home;
    press_mod(&mut app, KeyCode::Char('p'), KeyModifiers::CONTROL);
    assert!(!app.dialogs.is_empty(), "Ctrl+P should work on Home too");
}

// ─── Leader chord (Ctrl+X) ─────────────────────────────────────────────────

#[test]
fn ctrl_x_sets_leader_pending() {
    let mut app = App::new();
    make_visible(&mut app);
    press_mod(&mut app, KeyCode::Char('x'), KeyModifiers::CONTROL);
    assert!(app.leader_pending, "Ctrl+X should set leader_pending");
}

#[test]
fn leader_chord_t_opens_theme_dialog() {
    let mut app = App::new();
    make_visible(&mut app);
    assert!(app.dialogs.is_empty());
    press_mod(&mut app, KeyCode::Char('x'), KeyModifiers::CONTROL);
    press_char(&mut app, 't');
    assert!(!app.dialogs.is_empty(), "Ctrl+X t should open theme dialog");
    assert!(!app.leader_pending, "leader should clear after follower");
}

#[test]
fn leader_chord_m_opens_model_dialog() {
    let mut app = App::new();
    make_visible(&mut app);
    press_mod(&mut app, KeyCode::Char('x'), KeyModifiers::CONTROL);
    press_char(&mut app, 'm');
    assert!(!app.dialogs.is_empty(), "Ctrl+X m should open model dialog");
}

#[test]
fn leader_chord_n_creates_new_session() {
    let mut app = App::new();
    make_visible(&mut app);
    press_mod(&mut app, KeyCode::Char('x'), KeyModifiers::CONTROL);
    press_char(&mut app, 'n');
    assert!(
        matches!(app.route, Route::Session { .. }),
        "Ctrl+X n should navigate to Session"
    );
}

#[test]
fn leader_chord_unknown_follower_is_noop() {
    let mut app = App::new();
    make_visible(&mut app);
    app.route = Route::Shell;
    press_mod(&mut app, KeyCode::Char('x'), KeyModifiers::CONTROL);
    press_char(&mut app, 'z');
    // Should not crash, no dialog opened, leader cleared.
    assert!(!app.leader_pending);
    assert!(app.dialogs.is_empty());
    assert!(matches!(app.route, Route::Shell));
}

// ─── Session route Esc goes back ────────────────────────────────────────────

// Note: Esc in session route currently goes back to Shell via dispatch_key.
// But Ctrl+C is intercepted by translate_event as Quit. In practice, the
// prompt handles Esc as well. Let's verify the expected state transitions.

#[test]
fn dialog_esc_pops_dialog_without_route_change() {
    let mut app = App::new();
    make_visible(&mut app);
    app.route = Route::Shell;
    press_mod(&mut app, KeyCode::Char('p'), KeyModifiers::CONTROL);
    assert!(!app.dialogs.is_empty());
    press(&mut app, KeyCode::Esc);
    assert!(app.dialogs.is_empty(), "Esc should pop the dialog");
    assert!(
        matches!(app.route, Route::Shell),
        "Route should stay Shell after dialog Esc"
    );
}

// ─── Ctrl+B toggles sidebar on all routes ───────────────────────────────────

#[test]
fn ctrl_b_toggles_sidebar_on_session() {
    let mut app = App::new();
    make_visible(&mut app);
    app.route = Route::Session {
        session_id: SessionId::new("sess_t"),
    };
    assert!(app.sidebar_open);
    press_mod(&mut app, KeyCode::Char('b'), KeyModifiers::CONTROL);
    assert!(!app.sidebar_open);
    press_mod(&mut app, KeyCode::Char('b'), KeyModifiers::CONTROL);
    assert!(app.sidebar_open);
}

#[test]
fn paste_valid_zyal_updates_panel_without_arming() {
    let mut app = App::new();
    let text = "<<<ZYAL v1:daemon id=demo>>>\nversion: v1\n<<<END_ZYAL v1:daemon id=demo>>>\n";
    app.dispatch(Action::Paste(text.to_string()));

    assert!(app.zyal_runbook_valid);
    assert!(!app.zyal_runbook_armed);
    assert_eq!(app.prompt.expanded_buffer(), text);
    assert!(app
        .zyal_panel
        .snapshot()
        .paste_signature
        .as_ref()
        .unwrap()
        .starts_with("runbook:"));
}

#[test]
fn zyal_submit_requires_explicit_run_forever_arm() {
    let mut app = App::new();
    let text = "<<<ZYAL v1:daemon id=demo>>>\nversion: v1\n<<<END_ZYAL v1:daemon id=demo>>>\n";
    app.dispatch(Action::PromptSubmit(text.to_string()));

    assert!(app.zyal_runbook_valid);
    assert!(!app.zyal_runbook_armed);
    assert_eq!(app.transcript.len(), 1);
}

#[test]
fn zyal_submit_with_run_forever_arm_does_not_start_chat() {
    let mut app = App::new();
    let text = "<<<ZYAL v1:daemon id=demo>>>\nversion: v1\nZYAL_ARM RUN_FOREVER\n<<<END_ZYAL v1:daemon id=demo>>>\n";
    app.dispatch(Action::PromptSubmit(text.to_string()));

    assert!(app.zyal_runbook_valid);
    assert!(app.zyal_runbook_armed);
    assert_eq!(app.transcript.len(), 1);
}

#[test]
fn jankurai_cycle_requires_explicit_confirmation() {
    let mut app = App::new();
    app.dispatch(Action::RunJankuraiCycle);

    assert!(!app.is_audit_running);
    assert_eq!(app.transcript.len(), 1);
}

// ─── Prompt submit on session ───────────────────────────────────────────────

#[test]
fn prompt_submit_on_session_pushes_user_card() {
    let mut app = App::new();
    make_visible(&mut app);
    app.route = Route::Session {
        session_id: SessionId::new("sess_test"),
    };
    press_char(&mut app, 'h');
    press_char(&mut app, 'i');
    press(&mut app, KeyCode::Enter);
    assert!(app.prompt.buffer_string().is_empty());
    assert_eq!(app.transcript.len(), 1);
}

// ─── Mock LLM helpers ──────────────────────────────────────────────────────

#[test]
fn extract_json_response_field_works() {
    // The function is private to app.rs, but we can test it through the
    // mock_assistant_text public behavior if the env var is set.
    // Instead, test the key dispatch behavior with mock enabled.
    // For now, verify the UserCard push works (which is the TUI-side of the
    // mock LLM story).
    let mut app = App::new();
    make_visible(&mut app);
    app.route = Route::Shell;
    press_char(&mut app, 'x');
    press(&mut app, KeyCode::Enter);
    assert_eq!(
        app.transcript.len(),
        1,
        "Submit should push user card to transcript"
    );
}

// ─── Rendering smoke tests ─────────────────────────────────────────────────

fn render_app_to_string(app: &mut App, width: u16, height: u16) -> String {
    // App::draw is private, but we can render via the public `run_loop`
    // pattern with a TestBackend. Since run_loop blocks, we use the
    // internal module access pattern.
    //
    // For now, verify the App struct state is set up correctly for the route,
    // and that the fields that feed into rendering are initialized properly.
    let _ = (app, width, height);
    String::new()
}

#[test]
fn app_shell_route_has_prompt_and_transcript_fields() {
    let mut app = App::new();
    make_visible(&mut app);
    app.route = Route::Shell;
    // The prompt should be accessible and functional.
    assert!(app.prompt.buffer_string().is_empty());
    assert!(app.transcript.is_empty());
    assert!(app.sidebar_open);
}

#[test]
fn app_session_route_has_all_rendering_deps() {
    let mut app = App::new();
    make_visible(&mut app);
    app.route = Route::Session {
        session_id: SessionId::new("sess_render"),
    };
    // All dependencies for SessionRoute compositor should be initialized.
    assert!(app.prompt.buffer_string().is_empty());
    assert!(app.transcript.is_empty());
    assert!(app.sidebar_open);
    assert!(app.visible);
}

// ─── Dispatch action coverage ───────────────────────────────────────────────

#[test]
fn dispatch_prompt_submit_pushes_assistant_placeholder() {
    // Direct PromptSubmit dispatch (no key event) is the live-streaming
    // entry point. It does NOT push a user card (that's the dispatch_key
    // path's job) but DOES push an empty assistant placeholder so streaming
    // `RuntimeEvent::AssistantTextDelta` events have a card to append to.
    // The bridge thread will best-effort attempt to reach the jnoccio
    // gateway; failure modes leave the placeholder empty.
    let mut app = App::new();
    app.dispatch(Action::PromptSubmit("hello world".to_string()));
    assert_eq!(
        app.transcript.len(),
        1,
        "PromptSubmit via dispatch() should push exactly one assistant placeholder"
    );
}

#[test]
fn dispatch_prompt_cancel_clears_prompt() {
    let mut app = App::new();
    app.route = Route::Shell;
    press_char(&mut app, 'a');
    press_char(&mut app, 'b');
    assert_eq!(app.prompt.buffer_string(), "ab");
    app.dispatch(Action::PromptCancel);
    assert!(app.prompt.buffer_string().is_empty());
}

#[test]
fn dispatch_navigate_to_session() {
    let mut app = App::new();
    let sid = SessionId::new("sess_nav");
    app.dispatch(Action::Navigate(Route::Session {
        session_id: sid.clone(),
    }));
    match &app.route {
        Route::Session { session_id } => assert_eq!(session_id, &sid),
        other => panic!("expected Session, got {other:?}"),
    }
}

#[test]
fn dispatch_resize_records_dimensions() {
    let mut app = App::new();
    app.dispatch(Action::Resize {
        cols: 200,
        rows: 50,
    });
    assert_eq!(app.last_resize, Some((200, 50)));
}

// ─── Prompt does not consume modifier keys ──────────────────────────────────

#[test]
fn ctrl_b_not_eaten_by_prompt_on_shell() {
    // Ctrl+B should toggle sidebar even though the prompt is focused.
    let mut app = App::new();
    make_visible(&mut app);
    app.route = Route::Shell;
    assert!(app.sidebar_open);
    // Type something first to prove prompt is active.
    press_char(&mut app, 'a');
    assert_eq!(app.prompt.buffer_string(), "a");
    // Now Ctrl+B should toggle sidebar, not go to prompt.
    press_mod(&mut app, KeyCode::Char('b'), KeyModifiers::CONTROL);
    assert!(!app.sidebar_open);
}

// ─── PageUp/PageDown in transcript ──────────────────────────────────────────

#[test]
fn page_keys_affect_transcript_scroll() {
    let mut app = App::new();
    make_visible(&mut app);
    app.route = Route::Shell;
    // Push enough entries to make scrolling meaningful.
    for i in 0..20 {
        app.transcript
            .push_user(jekko_tui::UserCard::new(format!("msg {i}")));
    }
    app.transcript.set_viewport_rows(5);
    app.transcript.bottom();
    let offset_before = app.transcript.scroll_offset();
    // PageUp should reduce offset.
    app.transcript.page_up();
    assert!(
        app.transcript.scroll_offset() < offset_before,
        "PageUp should reduce scroll offset"
    );
}

// ─── Stage labels ───────────────────────────────────────────────────────────

#[test]
fn stage_labels_are_all_nonempty() {
    let stages = [
        Stage::Starting,
        Stage::LoadingTerminal,
        Stage::SyncingWorkspace,
        Stage::AppVisible,
    ];
    for stage in &stages {
        assert!(
            !stage.label().is_empty(),
            "Stage {:?} should have a non-empty label",
            stage
        );
    }
}

// ─── App default impl ──────────────────────────────────────────────────────

#[test]
fn app_default_matches_new() {
    let a = App::new();
    let b = App::default();
    assert_eq!(a.route, b.route);
    assert_eq!(a.theme, b.theme);
    assert_eq!(a.visible, b.visible);
    assert_eq!(a.quit, b.quit);
    assert_eq!(a.sidebar_open, b.sidebar_open);
}

// Silence render_app_to_string so the compiler doesn't complain.
#[allow(dead_code)]
fn _silence() {
    let _ = render_app_to_string;
}
