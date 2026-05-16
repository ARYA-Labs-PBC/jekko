//! Route group modules. Each submodule mounts one logical group of the
//! `/api/v1` (or `/api/v2`) surface against the Axum router.

pub mod config;
pub mod daemon;
pub mod events;
pub mod experimental;
pub mod file;
pub mod instance;
pub mod mcp;
pub mod openapi;
pub mod permission;
pub mod provider;
pub mod pty;
pub mod question;
pub mod session;
pub mod sync;
pub mod tui;
pub mod v2;
pub mod workspace;
pub mod ws;
