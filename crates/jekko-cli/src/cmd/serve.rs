//! `jekko serve` — start the headless HTTP server.
//!
//! Mirrors `packages/jekko/src/cli/cmd/serve.ts`. The actual `Server::listen`
//! lives in `jekko-server`, which is being filled in by another packet — for
//! now we just surface the parsed args.

use anyhow::Result;
use clap::Args;

use crate::cli::GlobalOpts;

/// `jekko serve` arguments.
///
/// Example: `jekko serve --port 8080 --hostname 0.0.0.0`.
#[derive(Args, Debug)]
pub struct ServeArgs {
    /// Port to bind. Defaults to a random free port.
    #[arg(long, default_value_t = 0, value_name = "N")]
    pub port: u16,

    /// Hostname to bind. Defaults to `127.0.0.1`.
    #[arg(long, default_value = "127.0.0.1", value_name = "H")]
    pub hostname: String,

    /// HTTP basic-auth password. Reads `JEKKO_SERVER_PASSWORD` from env when
    /// omitted; if neither is set the server runs unsecured (with a warning).
    #[arg(long, env = "JEKKO_SERVER_PASSWORD")]
    pub password: Option<String>,

    /// HTTP basic-auth username.
    #[arg(long, env = "JEKKO_SERVER_USERNAME", default_value = "jekko")]
    pub username: String,
}

impl Default for ServeArgs {
    fn default() -> Self {
        Self {
            port: 0,
            hostname: "127.0.0.1".to_string(),
            password: None,
            username: "jekko".to_string(),
        }
    }
}

/// Launch the HTTP server.
pub fn run(_global: &GlobalOpts, args: &ServeArgs) -> Result<()> {
    if args.password.is_none() {
        eprintln!("warning: JEKKO_SERVER_PASSWORD is not set; server would run unsecured.");
    }
    eprintln!(
        "jekko serve: would listen on http://{}:{} (pending server packet)",
        args.hostname, args.port
    );
    Ok(())
}
