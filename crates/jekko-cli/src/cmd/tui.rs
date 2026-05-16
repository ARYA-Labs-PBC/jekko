//! `jekko tui` — explicit TUI launch (default subcommand).
//!
//! Mirrors the implicit "no subcommand" branch of
//! `packages/jekko/src/index.ts`. The actual frame loop lives in
//! `jekko-tui` (Packets G/H).

use anyhow::Result;
use clap::Args;
use jekko_tui::{action::JnoccioBootStatus, TuiOptions};

use crate::cli::GlobalOpts;

/// Flags accepted by `jekko tui`. These are the TUI-specific ones; the
/// global flags (--pure, --headless) come in via [`GlobalOpts`].
#[derive(Args, Debug, Default)]
pub struct TuiArgs {
    /// Resume the last session if one exists.
    #[arg(long = "continue", short = 'c')]
    pub r#continue: bool,

    /// Open a specific session by id.
    #[arg(short = 's', value_name = "SESSION_ID")]
    pub session: Option<String>,
}

/// Launch the TUI.
pub fn run(global: &GlobalOpts, args: &TuiArgs) -> Result<()> {
    if let Some(session_id) = args.session.as_deref() {
        eprintln!("note: -s {session_id} not yet wired through TuiOptions");
    }
    if args.r#continue {
        eprintln!("note: --continue not yet wired through TuiOptions");
    }

    let opts = TuiOptions {
        pure: global.pure,
        headless: global.headless,
    };

    // Spawn the Jnoccio boot thread unless explicitly disabled (PTY tests,
    // CI, or environments without jnoccio-fusion configured).
    let jnoccio_rx = spawn_jnoccio_boot_bridge();

    jekko_tui::run_with_jnoccio(opts, None, jnoccio_rx)?;
    Ok(())
}

/// Spawn the Jnoccio boot thread and return a channel receiver that delivers
/// [`JnoccioBootStatus`] values to the TUI event loop.
///
/// The boot crate uses its own `BootStatus` type; a bridge thread converts
/// the types to keep the TUI crate free of a direct dep on
/// `jekko-jnoccio-boot`.
///
/// Returns `None` when `JEKKO_DISABLE_JNOCCIO_BOOT=1` is set so callers can
/// distinguish "no channel" from "channel active but server unavailable".
fn spawn_jnoccio_boot_bridge() -> Option<std::sync::mpsc::Receiver<JnoccioBootStatus>> {
    if std::env::var("JEKKO_DISABLE_JNOCCIO_BOOT").as_deref() == Ok("1") {
        return None;
    }

    let (boot_tx, boot_rx) = std::sync::mpsc::channel::<jekko_jnoccio_boot::BootEvent>();
    let (tui_tx, tui_rx) = std::sync::mpsc::channel::<JnoccioBootStatus>();

    // Launch the real boot thread.
    jekko_jnoccio_boot::spawn_boot_thread(boot_tx);

    // Bridge thread: convert BootEvent → JnoccioBootStatus and forward.
    // Exits when the boot thread drops its sender (TUI exit).
    std::thread::Builder::new()
        .name("jnoccio-boot-bridge".into())
        .spawn(move || {
            use jekko_jnoccio_boot::{BootEvent, BootStatus};
            #[allow(clippy::while_let_loop)]
            loop {
                match boot_rx.recv() {
                    Ok(BootEvent::StatusChanged(status)) => {
                        let tui_status = match status {
                            BootStatus::Idle => JnoccioBootStatus::Idle,
                            BootStatus::Checking => JnoccioBootStatus::Checking,
                            BootStatus::Starting => JnoccioBootStatus::Starting,
                            BootStatus::Ready {
                                enabled_models,
                                total_models,
                            } => JnoccioBootStatus::Ready {
                                enabled_models,
                                total_models,
                            },
                            BootStatus::Unavailable => JnoccioBootStatus::Unavailable,
                            BootStatus::Failed => JnoccioBootStatus::Failed,
                        };
                        if tui_tx.send(tui_status).is_err() {
                            break; // TUI exited
                        }
                    }
                    Err(_) => break, // boot thread gone
                }
            }
        })
        .ok();

    Some(tui_rx)
}
