use std::io::{self, Write};
use std::panic;

use anyhow::{Context, Result};
use crossterm::event::{
    DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen, SetTitle,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

pub type Tty = Terminal<CrosstermBackend<io::Stdout>>;

/// Terminal-restore escape sequence used after a fatal crash: shows cursor,
/// leaves alt-screen, disables mouse 1000/1002/1003/1006, and disables
/// bracketed paste 2004.
pub const FATAL_RESTORE_BYTES: &[u8] =
    b"\x1b[?25h\x1b[?1049l\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1006l\x1b[?2004l\r\n";

/// Configuration for `enter_terminal`.
pub struct EnterOptions {
    pub mouse: bool,
    pub bracketed_paste: bool,
    pub terminal_title: Option<String>,
}

impl Default for EnterOptions {
    fn default() -> Self {
        Self {
            mouse: true,
            bracketed_paste: true,
            terminal_title: Some("Jekko".to_string()),
        }
    }
}

/// Acquire raw mode, alt-screen, optional mouse + bracketed paste.
/// Installs a panic hook that restores the terminal before any panic message
/// is printed so the terminal stays usable.
pub fn enter_terminal(opts: &EnterOptions) -> Result<Tty> {
    install_panic_restore();
    enable_raw_mode().context("enable raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).context("enter alternate screen")?;
    if opts.mouse {
        execute!(stdout, EnableMouseCapture).context("enable mouse capture")?;
    }
    if opts.bracketed_paste {
        execute!(stdout, EnableBracketedPaste).context("enable bracketed paste")?;
    }
    if let Some(title) = opts.terminal_title.as_deref() {
        let _ = execute!(stdout, SetTitle(title));
    }
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend).context("create ratatui terminal")?;
    Ok(terminal)
}

/// Reverse of `enter_terminal`. Idempotent and best-effort: every step is
/// attempted even if a previous step failed, so the terminal lands in a
/// usable state.
pub fn leave_terminal(mut tty: Tty, opts: &EnterOptions) -> Result<()> {
    let _ = tty.show_cursor();
    let mut stdout = io::stdout();
    if opts.bracketed_paste {
        let _ = execute!(stdout, DisableBracketedPaste);
    }
    if opts.mouse {
        let _ = execute!(stdout, DisableMouseCapture);
    }
    let _ = execute!(stdout, LeaveAlternateScreen);
    if opts.terminal_title.is_some() {
        let _ = execute!(stdout, SetTitle(""));
    }
    let _ = disable_raw_mode();
    let _ = stdout.flush();
    Ok(())
}

/// Synchronous fatal-restore. Used by the panic hook and any error path that
/// cannot rely on the normal `leave_terminal` flow (e.g. mid-render error).
pub fn restore_for_fatal() {
    let mut stdout = io::stdout();
    let _ = stdout.write_all(FATAL_RESTORE_BYTES);
    let _ = stdout.flush();
    let _ = disable_raw_mode();
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
