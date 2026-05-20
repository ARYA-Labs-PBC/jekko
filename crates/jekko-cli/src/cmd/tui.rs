//! `jekko tui` — explicit TUI launch (default subcommand).
//!
//! Mirrors the implicit "no subcommand" branch of
//! `packages/jekko/src/index.ts`. The actual frame loop lives in
//! `jekko-tui` (Packets G/H).

use anyhow::Result;
use clap::Args;
use jekko_tui::action::JnoccioBootStatus;
use jekko_tui::chat_bridge_backend::{ChatBridgeBackend, ChatBridgeConfig};
use jekko_tui::inline_runtime::{run_inline, InlineRuntimeOptions};

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

    // Spawn the Jnoccio boot thread unless explicitly disabled (PTY tests,
    // CI, or environments without jnoccio-fusion configured).
    let jnoccio_rx = spawn_jnoccio_boot_bridge();

    let opts = InlineRuntimeOptions {
        no_alt_screen: global.headless,
        jnoccio_boot_status: if jnoccio_rx.is_some() {
            JnoccioBootStatus::Checking
        } else {
            JnoccioBootStatus::Disabled
        },
        jnoccio_boot_rx: jnoccio_rx,
        ..InlineRuntimeOptions::default()
    };
    run_inline(ChatBridgeBackend::new(ChatBridgeConfig::default()), opts)
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
    if !jekko_jnoccio_boot::unlock::is_unlocked() {
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
                            BootStatus::Unavailable => JnoccioBootStatus::NotInstalled,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use tempfile::TempDir;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    struct EnvGuard {
        prev_home: Option<std::ffi::OsString>,
        prev_dev: Option<std::ffi::OsString>,
        prev_disable: Option<std::ffi::OsString>,
        _lock: std::sync::MutexGuard<'static, ()>,
    }

    impl EnvGuard {
        fn locked(home: &std::path::Path) -> Self {
            let lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            let prev_home = std::env::var_os("HOME");
            let prev_dev = std::env::var_os("JNOCCIO_DEVELOPER_KEY");
            let prev_disable = std::env::var_os("JEKKO_DISABLE_JNOCCIO_BOOT");
            std::env::set_var("HOME", home);
            std::env::remove_var("JNOCCIO_DEVELOPER_KEY");
            std::env::remove_var("JEKKO_DISABLE_JNOCCIO_BOOT");
            Self {
                prev_home,
                prev_dev,
                prev_disable,
                _lock: lock,
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.prev_home {
                Some(v) => std::env::set_var("HOME", v),
                None => std::env::remove_var("HOME"),
            }
            match &self.prev_dev {
                Some(v) => std::env::set_var("JNOCCIO_DEVELOPER_KEY", v),
                None => std::env::remove_var("JNOCCIO_DEVELOPER_KEY"),
            }
            match &self.prev_disable {
                Some(v) => std::env::set_var("JEKKO_DISABLE_JNOCCIO_BOOT", v),
                None => std::env::remove_var("JEKKO_DISABLE_JNOCCIO_BOOT"),
            }
        }
    }

    #[test]
    fn jnoccio_boot_bridge_stays_disabled_when_locked() {
        let home = TempDir::new().unwrap();
        let _guard = EnvGuard::locked(home.path());

        assert!(spawn_jnoccio_boot_bridge().is_none());
    }
}
