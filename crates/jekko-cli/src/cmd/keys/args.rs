use clap::{Args, Subcommand};
use jekko_provider::key_pool::DEFAULT_USER_ID;

/// `jekko keys` args.
#[derive(Args, Debug)]
pub struct KeysArgs {
    /// User dir under `~/.jekko/users/<user>/`. Defaults to `user`. Extra
    /// user dirs require the Jnoccio developer unlock.
    #[arg(long, global = true, default_value = DEFAULT_USER_ID)]
    pub user: String,

    #[command(subcommand)]
    pub command: KeysCommand,
}

#[derive(Subcommand, Debug)]
pub enum KeysCommand {
    /// Set a key by name. Use `--value` or read from stdin.
    Set(KeysSetArgs),
    /// List currently configured keys (values redacted).
    List,
    /// Delete a key by name.
    Delete(KeysDeleteArgs),
    /// Print the canonical keys file path for the selected user.
    Path,
    /// Initialise the keys file if it does not exist.
    Init,
    /// Show machine-readable status.
    Status(KeysStatusArgs),
    /// List all detected user dirs and their key counts.
    Users(KeysUsersArgs),
}

#[derive(Args, Debug)]
pub struct KeysSetArgs {
    /// Key name (e.g. `ANTHROPIC_API_KEY`).
    pub name: String,
    /// Key value. When omitted, read from stdin.
    pub value: Option<String>,
}

#[derive(Args, Debug)]
pub struct KeysDeleteArgs {
    /// Key name.
    pub name: String,
}

#[derive(Args, Debug, Default)]
pub struct KeysStatusArgs {
    /// Emit machine-readable JSON.
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug, Default)]
pub struct KeysUsersArgs {
    /// Emit machine-readable JSON.
    #[arg(long)]
    pub json: bool,
}
