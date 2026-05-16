//! Ratatui widgets ported from `packages/jekko/src/cli/cmd/tui/component/`
//! and `packages/jekko/src/cli/cmd/tui/ui/`. Phase 8 of the migration plan.

pub mod activity_pulse;
pub mod footer;
pub mod logo;
pub mod nav_header;
pub mod spinner;
pub mod splash;
pub mod toast;

pub use activity_pulse::{sample as sample_activity_pulse, PulseSample};
pub use footer::{FooterBand, FooterBandLegacy};
pub use logo::{Logo, LogoBuilder};
pub use nav_header::{AppHeader, AuditStatus, NavBar, NavigationHeader, NavigationTab, StatusBar};
pub use spinner::Spinner;
pub use splash::{Splash, SplashState};
pub use toast::{Toast, ToastKind, ToastStack};
