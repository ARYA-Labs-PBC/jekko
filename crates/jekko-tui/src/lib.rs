//! Jekko TUI lifecycle and Ratatui frame loop.
//!
//! Replaces the previous JS TUI runtime in `packages/jekko/src/cli/cmd/tui/`.
//! Phase 7 of the migration plan defines the scope: terminal lifecycle (raw
//! mode, alt-screen, mouse capture, bracketed paste, terminal title), first-
//! frame watchdog, panic restore, startup screen widget, and the `Action`
//! dispatch enum.
//!
//! Components, dialogs, transcript rendering, and feature plugins arrive in
//! later packets (G, H, I, J).

use std::sync::mpsc::Receiver;
use std::time::Instant;

use anyhow::Result;
use serde::{Deserialize, Serialize};

pub mod action;
pub mod app;
pub mod chat_bridge;
pub mod components;
pub mod dialog;
pub mod engagement;
pub mod feature_plugins;
pub mod keybind;
pub mod lifecycle;
pub mod prompt;
pub mod startup_screen;
pub mod theme;
pub mod transcript;
pub mod watchdog;

pub use action::{
    default_initial_theme, Action, Route, RuntimeEvent, FIRST_FRAME_WATCHDOG, FRAME_TICK,
};
pub use app::{run_loop, translate_event, App, Stage};
pub use components::{
    FooterBand, Logo, NavigationHeader, NavigationTab, Spinner, Splash, SplashState, Toast,
    ToastKind, ToastStack,
};
pub use dialog::{
    CommandEntry, CommandPalette, Dialog, DialogFrame, DialogStack, SelectDialog, SelectOption,
};
pub use engagement::{EngagementState, LOGO_SLIDE_DURATION};
pub use feature_plugins::{
    FeaturePanel, JankuraiPanel, JankuraiSnapshot, JnoccioPanel, JnoccioSnapshot, JnoccioTab,
    PluginManager, PluginRow, PluginRowKind, Sidebar, SidebarEntry, ZyalPanel, ZyalSnapshot,
};
pub use lifecycle::{
    enter_terminal, leave_terminal, print_fatal_startup_error, restore_for_fatal, EnterOptions,
    Tty, FATAL_RESTORE_BYTES,
};
pub use transcript::{
    parse_unified_diff, tokenize_terminal, tokenize_yaml, AssistantCard, AssistantPart,
    AssistantPartKind, DaemonStatus, DiffFile, DiffHunk, DiffLine, DiffLineKind, PermissionAction,
    PermissionCard, PermissionChoice, PermissionDecisionEvent, PermissionStage, QuestionCard,
    QuestionChoice, QuestionEvent, QuestionMode, ReasoningCard, ScrollIntent, SessionRoute,
    SidebarPanel, StickyBottomIndicator, SubagentFooter, SystemCard, SystemKind, TerminalScope,
    TerminalSpan, ToolCard, ToolStatus, Transcript, TranscriptEntry, UserCard, YamlScope, YamlSpan,
};
pub use watchdog::FirstFrameWatchdog;
// Packet H (Phase 9) — prompt widget re-exports. Additions only; no changes
// above this line are owned by Packet H.
pub use prompt::{
    builtin_commands, display_width, grapheme_count, grapheme_offsets, truncate_to_width, Frecency,
    FrecencyRank, MentionCandidate, MentionPopup, PasteBuffer, PasteRecord, Prompt, PromptHistory,
    PromptOutcome, PromptSnapshot, PromptStash, RouteKey, SlashCommand, SlashPopup,
    PASTE_BYTE_THRESHOLD, PASTE_LINE_THRESHOLD,
};

/// Options forwarded from `jekko-cli`. Pure data — runtime channels are passed
/// separately to `run_with_runtime` to avoid forcing Clone/Eq through the
/// `Receiver` type.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct TuiOptions {
    /// `--pure` flag — disables side-effects useful for snapshot/PTY tests.
    pub pure: bool,
    /// `--headless` flag — render once to a buffer and exit. Used by the
    /// TUIwright matrix capture lane.
    pub headless: bool,
}

/// Run the TUI to completion without a runtime event source. Restores the
/// terminal on every path, including panics (via the panic hook installed by
/// `enter_terminal`).
pub fn run(options: TuiOptions) -> Result<()> {
    run_with_runtime(options, None)
}

/// Run the TUI with an optional runtime event receiver wired in.
pub fn run_with_runtime(
    options: TuiOptions,
    runtime_rx: Option<Receiver<RuntimeEvent>>,
) -> Result<()> {
    run_with_jnoccio(options, runtime_rx, None)
}

/// Run the TUI with an optional runtime event receiver and Jnoccio boot
/// channel. The boot channel carries [`jekko_jnoccio_boot::BootEvent`]s from
/// the background boot/re-poll thread; they are converted to
/// `Action::JnoccioBootUpdate` on each frame's drain pass.
pub fn run_with_jnoccio(
    options: TuiOptions,
    runtime_rx: Option<Receiver<RuntimeEvent>>,
    jnoccio_rx: Option<Receiver<action::JnoccioBootStatus>>,
) -> Result<()> {
    let started_at = Instant::now();
    let enter_opts = EnterOptions {
        // PTY/tuiwright tests run with mouse disabled to avoid escape-sequence
        // pollution in the recorded trace.
        mouse: std::env::var("TUIWRIGHT").ok().as_deref() != Some("1") && !options.headless,
        bracketed_paste: !options.headless,
        terminal_title: if options.headless {
            None
        } else {
            Some("Jekko".to_string())
        },
    };
    let mut terminal = match enter_terminal(&enter_opts) {
        Ok(t) => t,
        Err(err) => {
            restore_for_fatal();
            print_fatal_startup_error(&err, None);
            return Err(err);
        }
    };

    let mut app = App::new();
    // Splash boot window: leave `visible = false` so the first frame paints
    // the `RootStartupFallback`. `run_loop` flips `visible` after the first
    // successful draw, so the home route paints from frame 2 onward.
    app.stage = Stage::SyncingWorkspace;

    // Bridge the Jnoccio boot channel into the action stream. The run_loop
    // drains the jnoccio_rx on each tick and converts BootStatus to
    // Action::JnoccioBootUpdate before dispatching through the normal path.
    let result = run_loop_with_jnoccio(&mut app, &mut terminal, runtime_rx, jnoccio_rx, started_at);
    let _ = leave_terminal(terminal, &enter_opts);
    result
}

/// Extended run loop that also drains an optional Jnoccio boot channel.
fn run_loop_with_jnoccio(
    app: &mut App,
    terminal: &mut Tty,
    runtime_rx: Option<Receiver<RuntimeEvent>>,
    jnoccio_rx: Option<Receiver<action::JnoccioBootStatus>>,
    started_at: Instant,
) -> Result<()> {
    use action::{Action, JnoccioBootStatus};
    use std::sync::mpsc::TryRecvError;

    let watchdog = watchdog::FirstFrameWatchdog::install(started_at);
    let now = Instant::now();
    let mut last_draw = match now.checked_sub(action::FRAME_TICK) {
        Some(t) => t,
        None => now,
    };

    loop {
        // Drain internal action queue.
        loop {
            match app.action_rx.try_recv() {
                Ok(action) => app.dispatch(action),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }

        // Drain runtime events.
        if let Some(rx) = runtime_rx.as_ref() {
            loop {
                match rx.try_recv() {
                    Ok(evt) => app.dispatch(Action::Runtime(evt)),
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => break,
                }
            }
        }

        // Drain Jnoccio boot events — convert to Action::JnoccioBootUpdate.
        // Ignore the sentinel Idle re-polls used only to detect channel close.
        if let Some(rx) = jnoccio_rx.as_ref() {
            loop {
                match rx.try_recv() {
                    Ok(status) if status != JnoccioBootStatus::Idle => {
                        app.dispatch(Action::JnoccioBootUpdate(status));
                    }
                    Ok(_) => break, // Idle sentinel — skip
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => break,
                }
            }
        }

        // Read at most one crossterm event with a short poll budget.
        let poll_budget = action::FRAME_TICK
            .checked_sub(last_draw.elapsed())
            .unwrap_or(std::time::Duration::from_millis(0));
        if crossterm::event::poll(poll_budget).unwrap_or(false) {
            if let Ok(ev) = crossterm::event::read() {
                if let Some(action) = app::translate_event(ev) {
                    app.dispatch(action);
                }
            }
        }

        if app.quit {
            watchdog.cancel();
            return Ok(());
        }

        if !app.visible {
            app.splash.tick();
        }
        app.engagement.tick();
        terminal.draw(|frame| app.draw(frame))?;
        watchdog.mark_seen();
        if !app.visible && app.splash.ready_to_dismiss(app.is_ready()) {
            app.mark_app_visible();
        }
        last_draw = Instant::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feature_plugins::ShellTab;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use jekko_core::session::SessionId;
    use jekko_core::theme::ThemeMode;

    fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    #[test]
    fn default_options_are_safe() {
        let opts = TuiOptions::default();
        assert!(!opts.pure);
        assert!(!opts.headless);
    }

    #[test]
    fn default_route_is_home() {
        // `Route::default()` still resolves to `Home` so the existing nav
        // helpers keep their stable entry point; `App::new()` overrides this
        // at construction time so the user never sees the Home route (see
        // Phase A collapse).
        assert_eq!(Route::default(), Route::Home);
    }

    #[test]
    fn app_starts_on_shell_route() {
        // Phase A invariant: `App::new()` lands on Shell, bypassing Home.
        let app = App::new();
        assert!(
            matches!(app.route, Route::Shell),
            "App::new() must initialise to Route::Shell, got {:?}",
            app.route
        );
    }

    #[test]
    fn engagement_starts_idle() {
        let app = App::new();
        assert!(app.engagement.is_idle());
    }

    #[test]
    fn enter_on_empty_prompt_engages() {
        // Plain Enter on Shell with an empty prompt should dispatch
        // `Action::EngageSession`, which (after a drain) flips engagement
        // from Idle → Engaging.
        let mut app = App::new();
        assert!(app.engagement.is_idle());
        app.dispatch(Action::Key(key(KeyCode::Enter, KeyModifiers::NONE)));
        // The action sits on the queue; drain it.
        while let Ok(action) = app.action_rx.try_recv() {
            app.dispatch(action);
        }
        assert!(
            app.engagement.is_engaging(),
            "Enter on empty prompt must engage; state was {:?}",
            app.engagement
        );
    }

    #[test]
    fn prompt_submit_engages_once() {
        // Two consecutive PromptSubmits — the first transitions Idle →
        // Engaging; the second must NOT restart the slide. We capture the
        // initial slide start time via slide_progress monotonicity.
        let mut app = App::new();
        assert!(app.engagement.is_idle());
        app.dispatch(Action::PromptSubmit("hello".to_string()));
        assert!(app.engagement.is_engaging());
        let first_progress = app.engagement.slide_progress();
        std::thread::sleep(std::time::Duration::from_millis(20));
        app.dispatch(Action::PromptSubmit("world".to_string()));
        // Still engaging — the second submit didn't reset us back to Idle.
        assert!(app.engagement.is_engaging());
        // Progress is monotonically non-decreasing (proves no reset).
        let second_progress = app.engagement.slide_progress();
        assert!(
            second_progress >= first_progress,
            "second PromptSubmit must not restart the slide (before={first_progress}, after={second_progress})"
        );
    }

    #[test]
    fn engaging_completes_after_window() {
        // Manually set `started_at` to the past, then tick. Engagement should
        // promote to Engaged.
        let mut app = App::new();
        app.engagement = crate::engagement::EngagementState::Engaging {
            started_at: std::time::Instant::now()
                - crate::engagement::LOGO_SLIDE_DURATION
                - std::time::Duration::from_millis(50),
        };
        app.engagement.tick();
        assert!(app.engagement.is_engaged());
    }

    #[test]
    fn logo_slide_progress_clamped() {
        // Sanity check that progress stays in [0, 1] for the synthetic ages
        // a real run loop might observe.
        let state = crate::engagement::EngagementState::Engaging {
            started_at: std::time::Instant::now() - std::time::Duration::from_secs(10),
        };
        let p = state.slide_progress();
        assert!((0.0..=1.0).contains(&p));
    }

    #[test]
    fn app_dispatch_toggles_theme() {
        let mut app = App::new();
        app.theme = ThemeMode::Dark;
        app.dispatch(Action::ToggleTheme);
        assert_eq!(app.theme, ThemeMode::Light);
        app.dispatch(Action::ToggleTheme);
        assert_eq!(app.theme, ThemeMode::Dark);
    }

    #[test]
    fn app_dispatch_handles_quit() {
        let mut app = App::new();
        app.dispatch(Action::Quit);
        assert!(app.quit);
    }

    #[test]
    fn app_dispatch_records_resize() {
        let mut app = App::new();
        app.dispatch(Action::Resize {
            cols: 120,
            rows: 30,
        });
        assert_eq!(app.last_resize, Some((120, 30)));
    }

    #[test]
    fn app_phase1_initial_state_is_consistent() {
        let app = App::new();
        assert_eq!(app.shell_tab, ShellTab::Jnoccio);
        assert!(app.sidebar_open);
        assert!(app.prompt_focused);
        assert!(app.transcript.is_empty());
        assert!(app.prompt.buffer_string().is_empty());
    }

    #[test]
    fn app_shell_tab_cycle_action_wraps() {
        let mut app = App::new();
        app.dispatch(Action::ShellTabCycle { forward: true });
        assert_eq!(app.shell_tab, ShellTab::RepoIntel);
        app.dispatch(Action::ShellTabCycle { forward: true });
        assert_eq!(app.shell_tab, ShellTab::History);
        app.dispatch(Action::ShellTabCycle { forward: true });
        assert_eq!(app.shell_tab, ShellTab::Jnoccio);
        app.dispatch(Action::ShellTabCycle { forward: false });
        assert_eq!(app.shell_tab, ShellTab::History);
    }

    #[test]
    fn app_shell_tab_set_action_jumps() {
        let mut app = App::new();
        app.dispatch(Action::ShellTabSet(ShellTab::History));
        assert_eq!(app.shell_tab, ShellTab::History);
    }

    #[test]
    fn app_sidebar_toggle_action_flips() {
        let mut app = App::new();
        assert!(app.sidebar_open);
        app.dispatch(Action::SidebarToggle);
        assert!(!app.sidebar_open);
        app.dispatch(Action::SidebarToggle);
        assert!(app.sidebar_open);
    }

    #[test]
    fn app_ctrl_b_toggles_sidebar_regardless_of_route() {
        let mut app = App::new();
        app.route = Route::Home;
        app.dispatch(Action::Key(key(KeyCode::Char('b'), KeyModifiers::CONTROL)));
        assert!(!app.sidebar_open);
        app.route = Route::Shell;
        app.dispatch(Action::Key(key(KeyCode::Char('b'), KeyModifiers::CONTROL)));
        assert!(app.sidebar_open);
    }

    #[test]
    fn app_shell_tab_keybinds_cycle_on_shell_only() {
        let mut app = App::new();
        // Home + Tab should NOT touch shell_tab.
        app.route = Route::Home;
        app.dispatch(Action::Key(key(KeyCode::Tab, KeyModifiers::NONE)));
        assert_eq!(app.shell_tab, ShellTab::Jnoccio);

        // Shell + Tab: prompt is focused, so Tab should reach the prompt
        // (passthrough → falls to global tab cycle).
        app.route = Route::Shell;
        app.dispatch(Action::Key(key(KeyCode::Tab, KeyModifiers::NONE)));
        assert_eq!(app.shell_tab, ShellTab::RepoIntel);

        app.dispatch(Action::Key(key(KeyCode::BackTab, KeyModifiers::SHIFT)));
        assert_eq!(app.shell_tab, ShellTab::Jnoccio);
    }

    #[test]
    fn app_shell_numeric_keys_jump_to_tab() {
        let mut app = App::new();
        app.route = Route::Shell;
        // Prompt is focused; '1' should be typed into the prompt buffer first,
        // so it should NOT reach the shell-tab key bind.
        app.dispatch(Action::Key(key(KeyCode::Char('2'), KeyModifiers::NONE)));
        assert_eq!(app.shell_tab, ShellTab::Jnoccio);
        assert_eq!(app.prompt.buffer_string(), "2");

        // Defocus the prompt: '3' should now jump tabs.
        app.prompt_focused = false;
        app.dispatch(Action::Key(key(KeyCode::Char('3'), KeyModifiers::NONE)));
        assert_eq!(app.shell_tab, ShellTab::History);
    }

    #[test]
    fn app_prompt_enter_pushes_user_card_and_emits_submit() {
        let mut app = App::new();
        app.route = Route::Shell;
        // Type "hi" then press Enter.
        app.dispatch(Action::Key(key(KeyCode::Char('h'), KeyModifiers::NONE)));
        app.dispatch(Action::Key(key(KeyCode::Char('i'), KeyModifiers::NONE)));
        assert_eq!(app.prompt.buffer_string(), "hi");
        app.dispatch(Action::Key(key(KeyCode::Enter, KeyModifiers::NONE)));
        assert!(app.prompt.buffer_string().is_empty());
        assert_eq!(app.transcript.len(), 1);

        // The PromptSubmit action should be sitting on the queue.
        let queued = app.action_rx.try_recv().expect("queued submit");
        match queued {
            Action::PromptSubmit(text) => assert_eq!(text, "hi"),
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn app_prompt_ctrl_c_emits_cancel() {
        let mut app = App::new();
        app.route = Route::Session {
            session_id: SessionId::new("sess_test"),
        };
        app.dispatch(Action::Key(key(KeyCode::Char('a'), KeyModifiers::NONE)));
        assert_eq!(app.prompt.buffer_string(), "a");
        // We translate Ctrl+C in `translate_event`, but `dispatch_key` only
        // sees `Action::Key`s — Ctrl+C reaches the prompt as a key here.
        app.dispatch(Action::Key(key(KeyCode::Char('c'), KeyModifiers::CONTROL)));
        // Prompt cleared by the widget itself; queue carries PromptCancel.
        assert!(app.prompt.buffer_string().is_empty());
        let queued = app.action_rx.try_recv().expect("queued cancel");
        assert!(matches!(queued, Action::PromptCancel));
    }

    #[test]
    fn app_first_chars_land_in_prompt_from_start() {
        // Phase A collapse: the TUI starts on Shell, so the chat-Enter PTY
        // test's "type four chars then Enter" path now goes straight through
        // the prompt without a Home→Shell hop.
        let mut app = App::new();
        assert!(matches!(app.route, Route::Shell));
        for ch in ['t', 'e', 's', 't'] {
            app.dispatch(Action::Key(key(KeyCode::Char(ch), KeyModifiers::NONE)));
        }
        assert!(matches!(app.route, Route::Shell));
        assert_eq!(app.prompt.buffer_string(), "test");
    }

    #[test]
    fn app_home_q_still_quits() {
        // The bare-`q` Home quit affordance is preserved — any caller that
        // explicitly routes the App back to Home (only possible via
        // Action::Navigate today) keeps the existing quit keybind.
        let mut app = App::new();
        app.route = Route::Home;
        app.dispatch(Action::Key(key(KeyCode::Char('q'), KeyModifiers::NONE)));
        assert!(app.quit);
        assert!(matches!(app.route, Route::Home));
    }

    #[test]
    fn app_dispatch_navigates() {
        let mut app = App::new();
        let sid = SessionId::new("sess_abc");
        app.dispatch(Action::Navigate(Route::Session {
            session_id: sid.clone(),
        }));
        match &app.route {
            Route::Session { session_id } => assert_eq!(session_id, &sid),
            other => panic!("expected Session route, got {other:?}"),
        }
    }
}
