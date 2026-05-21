//! Ratatui widgets used by the Claude/Codex-style chat surface.
//!
//! The legacy `Logo`/`NavBar`/`FooterBand`/etc. widgets were removed in the
//! R3 legacy purge. What survives is the inline boot block (rendered once at
//! session start), the streaming spinner glyph, and the toast stack.

pub mod boot_inline;
pub mod footer_status;
pub mod output_pager;
pub mod permission_banner;
pub mod spinner;
pub mod splash;
pub mod toast;
pub mod working_strip;

pub use spinner::Spinner;
pub use toast::{Toast, ToastKind, ToastStack};
