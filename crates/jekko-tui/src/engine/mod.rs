//! Tool execution engine (COWBOY.md Phase F).
//!
//! - `plain_runner` (F1): `tokio::process::Command` for non-interactive tools
//!   (git, cargo, JSON-producing). Streams stdout/stderr line-by-line as
//!   `ToolEvent` chunks.
//! - `ansi` (F3): convert raw byte streams that contain ANSI SGR escapes into
//!   styled `Vec<Span>` for transcript rendering. Sanitizes dangerous OSC
//!   sequences (52 clipboard, 0/2 title, alternate-screen toggles, bracketed-
//!   paste toggles) that a child process might emit.
//! - `pty_runner` (F2) lives separately and depends on portable-pty.

pub mod ansi;
pub mod cancel;
pub mod output_collapse;
pub mod plain_runner;
pub mod pty_runner;
pub mod sandbox_linux;
pub mod sandbox_macos;
pub mod sandbox_policy;
