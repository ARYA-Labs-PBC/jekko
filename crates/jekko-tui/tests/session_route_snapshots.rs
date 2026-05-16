//! Insta snapshot tests for the Session route composed layout.
//!
//! Locks the visual parity of the session compositor (transcript scroll +
//! right sidebar + sticky bottom prompt) against the TS reference in
//! `routes/session/session-view.tsx` (commits 102c0359e, f12069089).
//!
//! All tests use `TestBackend` — no binary or PTY required.

use insta::assert_snapshot;
use ratatui::backend::TestBackend;
use ratatui::Terminal;

use jekko_tui::transcript::route::{PreviewPrompt, SessionRoute};
use jekko_tui::{
    AssistantCard, AssistantPart, AssistantPartKind, DaemonStatus, SidebarPanel, SystemCard,
    SystemKind, Transcript, UserCard,
};

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn render_session(
    transcript: &Transcript,
    sidebar: Option<&SidebarPanel>,
    width: u16,
    height: u16,
) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            let area = frame.area();
            let prompt = PreviewPrompt::new().with_hint("submit · ctrl+c clear · esc back");
            let route = SessionRoute::new(transcript, prompt)
                .with_footer_hint("submit · ctrl+c clear · esc back");
            let route = if let Some(s) = sidebar {
                route.with_sidebar(s)
            } else {
                route
            };
            frame.render_widget(route, area);
        })
        .unwrap();
    terminal.backend().to_string()
}

fn empty_transcript() -> Transcript {
    Transcript::new()
}

fn one_turn_transcript() -> Transcript {
    let mut t = Transcript::new();
    t.push_user(UserCard::new("What is the capital of France?".to_string()));
    t.push_assistant(
        AssistantCard::new(vec![AssistantPart::new(
            AssistantPartKind::Text,
            "The capital of France is Paris.".to_string(),
        )])
        .with_model("mock"),
    );
    t
}

fn multi_turn_transcript() -> Transcript {
    let mut t = Transcript::new();
    t.push_user(UserCard::new("Hello Jekko.".to_string()));
    t.push_assistant(AssistantCard::new(vec![AssistantPart::new(
        AssistantPartKind::Text,
        "Hello! How can I help you today?".to_string(),
    )]));
    t.push_user(UserCard::new("Explain async/await in Rust.".to_string()));
    t.push_assistant(AssistantCard::new(vec![AssistantPart::new(
        AssistantPartKind::Text,
        "async/await in Rust is built on futures. An async fn returns a Future \
         that you can .await to drive it to completion."
            .to_string(),
    )]));
    t.push_user(UserCard::new("Show me a simple example.".to_string()));
    t
}

// ─── Empty transcript ─────────────────────────────────────────────────────────

#[test]
fn session_route_empty_no_sidebar_120x30() {
    let t = empty_transcript();
    let out = render_session(&t, None, 120, 30);
    assert_snapshot!("session_route_empty_no_sidebar_120x30", out);
    // Prompt affordance should always render.
    assert!(
        out.contains("Awaiting Prompt") || out.contains(">"),
        "prompt missing"
    );
}

#[test]
fn session_route_empty_no_sidebar_60x20() {
    // Narrow: sidebar hidden (width check in SessionRoute).
    let t = empty_transcript();
    let out = render_session(&t, None, 60, 20);
    assert_snapshot!("session_route_empty_no_sidebar_60x20", out);
}

// ─── With sidebar ─────────────────────────────────────────────────────────────

#[test]
fn session_route_empty_sidebar_daemon_online_120x30() {
    let t = empty_transcript();
    let sidebar = SidebarPanel::new("Test Session")
        .with_session_id("sess_online")
        .with_daemon_status(DaemonStatus::Online);
    let out = render_session(&t, Some(&sidebar), 120, 30);
    assert_snapshot!("session_route_empty_sidebar_daemon_online_120x30", out);
    assert!(
        out.contains("Test Session") || out.contains("online"),
        "sidebar content missing"
    );
}

#[test]
fn session_route_empty_sidebar_daemon_offline_120x30() {
    let t = empty_transcript();
    let sidebar = SidebarPanel::new("Offline Session")
        .with_session_id("sess_offline")
        .with_daemon_status(DaemonStatus::Offline);
    let out = render_session(&t, Some(&sidebar), 120, 30);
    assert_snapshot!("session_route_empty_sidebar_daemon_offline_120x30", out);
}

// ─── With transcript content ──────────────────────────────────────────────────

#[test]
fn session_route_one_turn_120x30() {
    let t = one_turn_transcript();
    let out = render_session(&t, None, 120, 30);
    assert_snapshot!("session_route_one_turn_120x30", out);
    assert!(out.contains("France"), "user card text missing");
    assert!(out.contains("Paris"), "assistant card text missing");
}

#[test]
fn session_route_one_turn_with_sidebar_120x30() {
    let t = one_turn_transcript();
    let sidebar = SidebarPanel::new("Paris Session")
        .with_session_id("sess_paris")
        .with_daemon_status(DaemonStatus::Online);
    let out = render_session(&t, Some(&sidebar), 120, 30);
    assert_snapshot!("session_route_one_turn_with_sidebar_120x30", out);
    assert!(out.contains("France"), "transcript missing with sidebar");
    assert!(
        out.contains("Paris"),
        "assistant reply missing with sidebar"
    );
}

#[test]
fn session_route_multi_turn_120x40() {
    let t = multi_turn_transcript();
    let out = render_session(&t, None, 120, 40);
    assert_snapshot!("session_route_multi_turn_120x40", out);
    assert!(out.contains("Hello"), "first user card missing");
    assert!(out.contains("async/await"), "second assistant card missing");
}

#[test]
fn session_route_multi_turn_200x60() {
    // Widest resolution — full horizontal space available.
    let t = multi_turn_transcript();
    let sidebar = SidebarPanel::new("Wide Session")
        .with_session_id("sess_wide")
        .with_daemon_status(DaemonStatus::Online);
    let out = render_session(&t, Some(&sidebar), 200, 60);
    assert_snapshot!("session_route_multi_turn_200x60", out);
    assert!(out.contains("async/await"), "content missing at 200x60");
}

// ─── System cards in transcript ───────────────────────────────────────────────

#[test]
fn session_route_system_cards_120x30() {
    let mut t = Transcript::new();
    t.push_system(SystemCard::new("Session started", SystemKind::Info));
    t.push_user(UserCard::new("Run tests.".to_string()));
    t.push_system(SystemCard::new("Token budget at 80%", SystemKind::Warning));
    t.push_assistant(AssistantCard::new(vec![AssistantPart::new(
        AssistantPartKind::Text,
        "Running cargo test...".to_string(),
    )]));
    let out = render_session(&t, None, 120, 30);
    assert_snapshot!("session_route_system_cards_120x30", out);
    assert!(out.contains("Session started"), "system info card missing");
}

// ─── Footer hint ─────────────────────────────────────────────────────────────

#[test]
fn session_route_footer_hint_120x30() {
    let t = one_turn_transcript();
    let backend = TestBackend::new(120, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            let area = frame.area();
            let prompt = PreviewPrompt::new();
            let route =
                SessionRoute::new(&t, prompt).with_footer_hint("submit · ctrl+c clear · esc back");
            frame.render_widget(route, area);
        })
        .unwrap();
    let out = terminal.backend().to_string();
    assert_snapshot!("session_route_footer_hint_120x30", out);
    assert!(out.contains("submit"), "footer hint text missing");
}
