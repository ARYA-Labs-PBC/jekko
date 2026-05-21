//! Layout primitives shared by status-row widgets (COWBOY.md T1-V9).
//!
//! Today this is only [`status_pack`], the priority-based truncator used
//! by the permission banner, working strip, and footer to keep status
//! rows on a single line at every terminal width.

pub mod status_pack;
