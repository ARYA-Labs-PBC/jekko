//! Ratatui `TestBackend` + `insta` snapshots for Phase 8 components and
//! dialogs (Packet G). Locks visual parity for the foundation set of widgets.

use insta::assert_snapshot;
use ratatui::backend::TestBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::Terminal;

use jekko_tui::{
    CommandEntry, CommandPalette, Dialog, DialogStack, Logo, NavigationHeader,
    SelectDialog, SelectOption, Splash, Toast, ToastStack,
};
use jekko_tui::components::FooterBandLegacy as FooterBand;

fn buf_to_string<F: FnOnce(&mut ratatui::Frame)>(width: u16, height: u16, f: F) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(f).unwrap();
    terminal.backend().to_string()
}

fn render_at(width: u16, height: u16, draw: impl Fn(&mut ratatui::Frame, Rect)) -> String {
    buf_to_string(width, height, |frame| {
        let area = frame.area();
        draw(frame, area);
    })
}

#[test]
fn nav_header_home_active_80x24() {
    let header = NavigationHeader::home_back_jnoccio(true, false, false);
    let out = render_at(80, 1, |frame, area| frame.render_widget(&header, area));
    assert_snapshot!("nav_header_home_active_80x1", out);
}

#[test]
fn nav_header_with_jnoccio_120x1() {
    let header = NavigationHeader::home_back_jnoccio(false, true, true);
    let out = render_at(120, 1, |frame, area| frame.render_widget(&header, area));
    assert_snapshot!("nav_header_with_jnoccio_120x1", out);
}

#[test]
fn footer_band_80x3() {
    let footer = FooterBand::new(vec![
        ("Ctrl+P", "commands"),
        ("Ctrl+C", "quit"),
        ("?", "help"),
    ]);
    let out = render_at(80, 3, |frame, area| frame.render_widget(&footer, area));
    assert_snapshot!("footer_band_80x3", out);
}

#[test]
fn logo_80x10() {
    let logo = Logo;
    let out = render_at(80, 10, |frame, area| frame.render_widget(&logo, area));
    assert_snapshot!("logo_80x10", out);
}

#[test]
fn logo_ascii_fallback_40x6() {
    // Narrow terminal: auto-pick should fall back to the legible ASCII face.
    let logo = Logo;
    let out = render_at(40, 6, |frame, area| frame.render_widget(&logo, area));
    assert_snapshot!("logo_ascii_fallback_40x6", out);
}

#[test]
fn logo_pixel_with_subtitle_80x8() {
    let logo = Logo::pixel()
        .with_support("ZYAL")
        .with_status("safe autonomous coding ready");
    let out = render_at(80, 8, |frame, area| frame.render_widget(&logo, area));
    assert_snapshot!("logo_pixel_with_subtitle_80x8", out);
}

#[test]
fn splash_120x30() {
    let splash = Splash::new("Code with an agent that lives in your terminal.", "v1.0.3");
    let out = render_at(120, 30, |frame, area| frame.render_widget(&splash, area));
    assert_snapshot!("splash_120x30", out);
}

#[test]
fn toast_stack_pinned_bottom_right_120x30() {
    let mut stack = ToastStack::default();
    stack.push(Toast::info("Loaded workspace."));
    stack.push(Toast::success("Session saved."));
    stack.push(Toast::warning("Token budget at 80%."));
    let out = render_at(120, 30, |frame, area| frame.render_widget(&stack, area));
    assert_snapshot!("toast_stack_120x30", out);
}

#[test]
fn select_dialog_centered_100x30() {
    let options = vec![
        SelectOption::new("dark", "Dark").with_hint("default"),
        SelectOption::new("light", "Light"),
        SelectOption::new("dracula", "Dracula").with_hint("preset"),
        SelectOption::new("nord", "Nord").with_hint("preset"),
    ];
    let mut dialog = SelectDialog::new("Theme", options);
    dialog.move_cursor(1);
    let out = render_at(100, 30, |frame, area| frame.render_widget(&dialog, area));
    assert_snapshot!("select_dialog_theme_100x30", out);
}

// ─── Additional parity coverage snapshots (Workstream 1) ─────────────────────

#[test]
fn splash_narrow_80x24() {
    // Narrow terminal: logo falls back to ASCII face.
    let splash = Splash::new("Code with an agent that lives in your terminal.", "v1.0.3");
    let out = render_at(80, 24, |frame, area| frame.render_widget(&splash, area));
    assert_snapshot!("splash_narrow_80x24", out);
}

#[test]
fn splash_wide_200x60() {
    // Widest canonical resolution: logo pixel mode with large gutter.
    let splash = Splash::new("Code with an agent that lives in your terminal.", "v1.0.3");
    let out = render_at(200, 60, |frame, area| frame.render_widget(&splash, area));
    assert_snapshot!("splash_wide_200x60", out);
}

#[test]
fn nav_header_shell_route_120x1() {
    // Shell route: Home tab inactive, Jnoccio tab shown but inactive.
    let header = NavigationHeader::home_back_jnoccio(false, false, false);
    let out = render_at(120, 1, |frame, area| frame.render_widget(&header, area));
    assert_snapshot!("nav_header_shell_route_120x1", out);
}

#[test]
fn nav_header_session_with_jnoccio_120x1() {
    // Session route with Jnoccio connected.
    let header = NavigationHeader::home_back_jnoccio(false, false, true);
    let out = render_at(120, 1, |frame, area| frame.render_widget(&header, area));
    assert_snapshot!("nav_header_session_with_jnoccio_120x1", out);
}

#[test]
fn footer_band_shell_hints_80x3() {
    // Shell route footer hint set.
    let footer = FooterBand::new(vec![
        ("Ctrl+P", "commands"),
        ("Ctrl+X", "leader"),
        ("Ctrl+H", "back"),
        ("Ctrl+C", "quit"),
    ]);
    let out = render_at(80, 3, |frame, area| frame.render_widget(&footer, area));
    assert_snapshot!("footer_band_shell_hints_80x3", out);
}

#[test]
fn footer_band_session_hints_80x3() {
    // Session route footer hint set.
    let footer = FooterBand::new(vec![
        ("Ctrl+P", "commands"),
        ("Ctrl+X", "leader"),
        ("Esc", "interrupt"),
        ("Ctrl+C", "quit"),
    ]);
    let out = render_at(80, 3, |frame, area| frame.render_widget(&footer, area));
    assert_snapshot!("footer_band_session_hints_80x3", out);
}

#[test]
fn command_palette_empty_query_120x30() {
    let entries = vec![
        CommandEntry::new("session.new", "New session").with_keybind("Ctrl+X N"),
        CommandEntry::new("model.list", "Model picker")
            .with_keybind("Ctrl+X M")
            .with_description("Choose model"),
        CommandEntry::new("theme.list", "Theme picker").with_keybind("Ctrl+X T"),
        CommandEntry::new("session.export", "Export session")
            .with_keybind("Ctrl+X X")
            .with_description("Write transcript to disk"),
        CommandEntry::new("plugin.manager", "Plugin manager"),
    ];
    let palette = CommandPalette::new(entries);
    let out = render_at(120, 30, |frame, area| frame.render_widget(&palette, area));
    assert_snapshot!("command_palette_empty_120x30", out);
}

#[test]
fn command_palette_filtered_120x30() {
    let entries = vec![
        CommandEntry::new("session.new", "New session"),
        CommandEntry::new("session.list", "List sessions"),
        CommandEntry::new("session.export", "Export session"),
        CommandEntry::new("model.list", "Model picker"),
        CommandEntry::new("theme.list", "Theme picker"),
    ];
    let mut palette = CommandPalette::new(entries);
    palette.type_char('s');
    palette.type_char('e');
    palette.type_char('s');
    let out = render_at(120, 30, |frame, area| frame.render_widget(&palette, area));
    assert_snapshot!("command_palette_filtered_120x30", out);
}

#[test]
fn dialog_stack_renders_top_only_100x30() {
    let mut stack = DialogStack::default();
    stack.push(Dialog::Select(SelectDialog::new(
        "Bottom",
        vec![SelectOption::new("a", "Hidden A")],
    )));
    stack.push(Dialog::Command(CommandPalette::new(vec![
        CommandEntry::new("top", "Top palette"),
    ])));
    let out = render_at(100, 30, |frame, area| frame.render_widget(&stack, area));
    assert_snapshot!("dialog_stack_top_command_100x30", out);
}

#[test]
fn home_route_with_header_footer_120x30() {
    let header = NavigationHeader::home_back_jnoccio(true, true, false);
    let footer = FooterBand::new(vec![("Ctrl+P", "commands"), ("Ctrl+C", "quit")]);
    let out = buf_to_string(120, 30, |frame| {
        let area = frame.area();
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(2),
            ])
            .split(area);
        frame.render_widget(&header, rows[0]);
        frame.render_widget(&Splash::new("Hello jekko.", "v1.0.3"), rows[1]);
        frame.render_widget(&footer, rows[2]);
    });
    assert_snapshot!("home_chrome_120x30", out);
}
