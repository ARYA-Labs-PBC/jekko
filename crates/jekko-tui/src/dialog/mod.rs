//! Modal dialog framework. Ports `ui/dialog.tsx`, `ui/dialog-select.tsx`,
//! `component/dialog-command.tsx` and friends to Ratatui widgets.

pub mod command;
pub mod frame;
pub mod select;
pub mod stack;

pub use command::{CommandEntry, CommandPalette};
pub use frame::DialogFrame;
pub use select::{SelectDialog, SelectOption};
pub use stack::{Dialog, DialogStack};
