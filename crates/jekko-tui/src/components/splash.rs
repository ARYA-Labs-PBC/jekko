//! Static startup splash (COWBOY T1-V6, new scope).
//!
//! Pure-function renderer for the boot-time "JEKKO" wordmark + subtitle. The
//! runtime calls [`render_splash`] during startup and stops calling
//! it once the user submits the first prompt. There is no internal state and no
//! self-dismiss logic; the lifecycle is the runtime's job.

mod context;
mod render;

#[cfg(test)]
mod tests;

pub use context::SplashContext;
#[cfg(test)]
pub(crate) use render::render_splash_static_for_tests;
pub use render::{render_splash, snapshot_lines, SPLASH_ROW_COUNT};
