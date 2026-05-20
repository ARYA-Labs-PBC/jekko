//! Terminal lifecycle: acquire/release the chat runtime's terminal modes.
//!
//! After the R3 legacy purge, only the agent-terminal path remains. The
//! `--no-alt-screen` compatibility branch reuses the inline-viewport mode
//! used by the original Claude-style spike; the default branch acquires the
//! fullscreen alt-screen + raw mode + bracketed paste needed by the new
//! Codex-style renderer. Mouse capture is opt-in so native terminal text
//! selection keeps working by default.

use std::io::{self, Write};
use std::panic;

use anyhow::{Context, Result};
use crossterm::cursor::MoveTo;
use crossterm::event::{
    DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
    LeaveAlternateScreen, SetTitle,
};
use ratatui::backend::CrosstermBackend;
use ratatui::{Terminal, TerminalOptions, Viewport};

pub type Tty = Terminal<CrosstermBackend<io::Stdout>>;

/// Default `--no-alt-screen` compatibility viewport height in rows.
pub const INLINE_VIEWPORT_ROWS: u16 = 8;

/// Terminal-restore escape sequence used after a fatal crash: shows cursor,
/// leaves alt-screen, disables mouse 1000/1002/1003/1006, and disables
/// bracketed paste 2004.
pub const FATAL_RESTORE_BYTES: &[u8] =
    b"\x1b[?25h\x1b[?1049l\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1006l\x1b[?2004l\r\n";

/// Synchronous fatal-restore. Used by the panic hook and any error path that
/// cannot rely on the normal leave path (e.g. mid-render error).
pub fn restore_for_fatal() {
    let mut stdout = io::stdout();
    let _ = stdout.write_all(FATAL_RESTORE_BYTES);
    let _ = stdout.flush();
    let _ = disable_raw_mode();
}

/// Terminal mode for the agent chat runtime.
pub struct AgentTerminalOptions {
    pub no_alt_screen: bool,
    pub viewport_rows: u16,
    pub mouse: bool,
    pub bracketed_paste: bool,
    pub terminal_title: Option<String>,
}

impl Default for AgentTerminalOptions {
    fn default() -> Self {
        Self {
            no_alt_screen: false,
            viewport_rows: INLINE_VIEWPORT_ROWS,
            mouse: false,
            bracketed_paste: true,
            terminal_title: Some("Jekko".to_string()),
        }
    }
}

/// Acquire raw mode + (optionally) alt-screen + mouse + bracketed paste.
/// Installs a panic hook that restores the terminal before any panic message
/// is printed so the terminal stays usable.
pub fn enter_agent_terminal(opts: &AgentTerminalOptions) -> Result<Tty> {
    install_panic_restore();
    enable_raw_mode().context("enable raw mode")?;
    let mut stdout = io::stdout();

    if !opts.no_alt_screen {
        execute!(stdout, EnterAlternateScreen).context("enter alternate screen")?;
        // Alt-screen inherits the shell cursor position on some terminals.
        // Reset to the top-left before the first draw so Jekko opens at row 0.
        execute!(stdout, Clear(ClearType::All), MoveTo(0, 0))
            .context("reset alternate screen cursor")?;
        if opts.mouse {
            execute!(stdout, EnableMouseCapture).context("enable mouse capture")?;
        }
    }

    if opts.bracketed_paste {
        execute!(stdout, EnableBracketedPaste).context("enable bracketed paste")?;
    }

    if let Some(title) = opts.terminal_title.as_deref() {
        let _ = execute!(stdout, SetTitle(title));
    }

    let backend = CrosstermBackend::new(stdout);
    if opts.no_alt_screen {
        Terminal::with_options(
            backend,
            TerminalOptions {
                viewport: Viewport::Inline(opts.viewport_rows.max(1)),
            },
        )
        .context("create inline ratatui terminal")
    } else {
        Terminal::new(backend).context("create ratatui terminal")
    }
}

/// Reverse of `enter_agent_terminal`. Idempotent and best-effort: every step
/// is attempted even if a previous step failed, so the terminal lands in a
/// usable state.
pub fn leave_agent_terminal(mut tty: Tty, opts: &AgentTerminalOptions) -> Result<()> {
    // Clear the inline viewport so the cursor doesn't land on a
    // half-rendered composer when raw mode is disabled.
    if opts.no_alt_screen {
        let _ = tty.clear();
    }
    let _ = tty.show_cursor();
    let mut stdout = io::stdout();
    if opts.bracketed_paste {
        let _ = execute!(stdout, DisableBracketedPaste);
    }
    if !opts.no_alt_screen {
        if opts.mouse {
            let _ = execute!(stdout, DisableMouseCapture);
        }
        let _ = execute!(stdout, LeaveAlternateScreen);
    }
    if opts.terminal_title.is_some() {
        let _ = execute!(stdout, SetTitle(""));
    }
    let _ = disable_raw_mode();
    let _ = stdout.flush();
    Ok(())
}

/// RAII guard that restores terminal modes on early errors. Belt-and-
/// suspenders with the panic hook: the hook handles panic paths; this guard
/// handles early-`?` paths, normal exits, and any caller that forgets to
/// call `leave_agent_terminal`.
pub struct TerminalRestoreGuard {
    alt_screen: bool,
    mouse: bool,
    bracketed_paste: bool,
    title_set: bool,
    restored: bool,
}

impl TerminalRestoreGuard {
    pub fn for_agent(opts: &AgentTerminalOptions) -> Self {
        Self {
            alt_screen: !opts.no_alt_screen,
            mouse: !opts.no_alt_screen && opts.mouse,
            bracketed_paste: opts.bracketed_paste,
            title_set: opts.terminal_title.is_some(),
            restored: false,
        }
    }

    pub fn mark_restored(&mut self) {
        self.restored = true;
    }
}

impl Drop for TerminalRestoreGuard {
    fn drop(&mut self) {
        if self.restored {
            return;
        }
        let mut stdout = io::stdout();
        if self.bracketed_paste {
            let _ = execute!(stdout, DisableBracketedPaste);
        }
        if self.mouse {
            let _ = execute!(stdout, DisableMouseCapture);
        }
        if self.alt_screen {
            let _ = execute!(stdout, LeaveAlternateScreen);
        }
        if self.title_set {
            let _ = execute!(stdout, SetTitle(""));
        }
        let _ = disable_raw_mode();
        let _ = stdout.flush();
    }
}

fn install_panic_restore() {
    let prev = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        restore_for_fatal();
        prev(info);
    }));
}

/// Print a fatal startup error to stderr. Mirrors `printFatalStartupError`.
pub fn print_fatal_startup_error(error: &anyhow::Error, log_path: Option<&str>) {
    let suffix = match log_path {
        Some(p) => format!(" Check log file at {p}."),
        None => String::new(),
    };
    let _ = writeln!(
        io::stderr(),
        "Jekko TUI failed to start.{suffix}\n{error:#}"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_agent_options_use_alt_screen_without_mouse_capture() {
        let opts = AgentTerminalOptions::default();
        assert!(!opts.no_alt_screen);
        assert!(!opts.mouse);
        assert!(opts.bracketed_paste);
    }

    #[test]
    fn restore_guard_skips_when_marked() {
        let opts = AgentTerminalOptions::default();
        let mut guard = TerminalRestoreGuard::for_agent(&opts);
        guard.mark_restored();
        drop(guard);
    }

    /// Pragmatic smoke test (per T2-P5 spec — real TTY required for full
    /// verification): explicit mouse opt-in must still arm mouse-capture
    /// cleanup on the default alt-screen path.
    #[test]
    fn enter_agent_terminal_alt_screen_enables_mouse_capture() {
        let opts = AgentTerminalOptions {
            mouse: true,
            ..AgentTerminalOptions::default()
        };
        assert!(!opts.no_alt_screen);
        let guard = TerminalRestoreGuard::for_agent(&opts);
        assert!(guard.mouse, "guard must disable mouse capture on drop");
        assert!(guard.alt_screen);
    }

    /// With `--no-alt-screen` we must NOT enable mouse capture — otherwise
    /// the terminal's native scroll + selection break for inline mode.
    #[test]
    fn enter_agent_terminal_no_alt_screen_skips_mouse_capture() {
        let opts = AgentTerminalOptions {
            no_alt_screen: true,
            mouse: true, // even if explicitly true, alt-screen=off wins
            ..AgentTerminalOptions::default()
        };
        let guard = TerminalRestoreGuard::for_agent(&opts);
        assert!(
            !guard.mouse,
            "mouse capture must stay off in --no-alt-screen mode",
        );
        assert!(!guard.alt_screen);
    }

    /// The Drop impl must symmetrically disable mouse capture when the
    /// guard was armed for it. We can't intercept the crossterm escape
    /// writes from a unit test, so this is a "doesn't panic / completes
    /// cleanly" smoke check.
    #[test]
    fn terminal_restore_guard_disables_mouse_capture_on_drop() {
        let opts = AgentTerminalOptions {
            mouse: true,
            ..AgentTerminalOptions::default()
        };
        let guard = TerminalRestoreGuard::for_agent(&opts);
        assert!(guard.mouse);
        drop(guard); // exercises the disable path; must not panic
    }

    /// Explicit `mouse: false` opt-out must propagate even with alt-screen.
    #[test]
    fn restore_guard_respects_mouse_false_opt_out() {
        let opts = AgentTerminalOptions {
            mouse: false,
            ..AgentTerminalOptions::default()
        };
        let guard = TerminalRestoreGuard::for_agent(&opts);
        assert!(!guard.mouse);
        assert!(guard.alt_screen);
    }
}
