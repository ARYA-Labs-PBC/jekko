//! `jekko keys` — manage canonical model API keys.

mod actions;
mod args;
mod storage;
mod users;

#[cfg(test)]
mod tests;

use anyhow::Result;

use crate::cli::GlobalOpts;

pub use args::*;

pub fn run(_global: &GlobalOpts, args: &KeysArgs) -> Result<()> {
    storage::migrate_existing_jekko_env()?;
    storage::enforce_user_gate(&args.user, jekko_jnoccio_boot::unlock::is_unlocked())?;
    match &args.command {
        KeysCommand::Set(opts) => actions::set(&args.user, opts),
        KeysCommand::List => actions::list(&args.user),
        KeysCommand::Delete(opts) => actions::delete(&args.user, opts),
        KeysCommand::Path => actions::path(&args.user),
        KeysCommand::Init => actions::init(&args.user),
        KeysCommand::Status(opts) => actions::status(&args.user, opts),
        KeysCommand::Users(opts) => users::users(opts),
    }
}
