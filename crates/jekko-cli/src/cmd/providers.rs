//! `jekko providers` — manage AI providers and credentials.
//!
//! Mirrors `packages/jekko/src/cli/cmd/providers.ts` plus the auth helpers
//! under `packages/jekko/src/cli/cmd/providers/`.

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::cli::GlobalOpts;

/// `jekko providers` arguments.
#[derive(Args, Debug)]
pub struct ProvidersArgs {
    #[command(subcommand)]
    pub command: ProvidersCommand,
}

/// Provider subcommands.
#[derive(Subcommand, Debug)]
pub enum ProvidersCommand {
    /// List configured providers.
    List,
    /// Show details for a provider.
    Show(ProviderShowArgs),
    /// Mark a provider as enabled.
    Enable(ProviderToggleArgs),
    /// Mark a provider as disabled.
    Disable(ProviderToggleArgs),
    /// Begin an OAuth or API-key login flow.
    Login(ProviderToggleArgs),
    /// Forget stored credentials for a provider.
    Logout(ProviderToggleArgs),
}

#[derive(Args, Debug)]
pub struct ProviderShowArgs {
    /// Provider identifier (e.g. `anthropic`, `openai`).
    pub provider: String,
}

#[derive(Args, Debug)]
pub struct ProviderToggleArgs {
    /// Provider identifier (e.g. `anthropic`).
    pub provider: String,
}

pub fn run(_global: &GlobalOpts, args: &ProvidersArgs) -> Result<()> {
    match &args.command {
        ProvidersCommand::List => list(),
        ProvidersCommand::Show(opts) => show(opts),
        ProvidersCommand::Enable(opts) => enable(opts),
        ProvidersCommand::Disable(opts) => disable(opts),
        ProvidersCommand::Login(opts) => login(opts),
        ProvidersCommand::Logout(opts) => logout(opts),
    }
}

fn list() -> Result<()> {
    eprintln!("jekko providers list: pending provider integration");
    Ok(())
}

fn show(args: &ProviderShowArgs) -> Result<()> {
    eprintln!(
        "jekko providers show {}: pending Packet D provider integration",
        args.provider
    );
    Ok(())
}

fn enable(args: &ProviderToggleArgs) -> Result<()> {
    eprintln!(
        "jekko providers enable {}: pending Packet D provider integration",
        args.provider
    );
    Ok(())
}

fn disable(args: &ProviderToggleArgs) -> Result<()> {
    eprintln!(
        "jekko providers disable {}: pending Packet D provider integration",
        args.provider
    );
    Ok(())
}

fn login(args: &ProviderToggleArgs) -> Result<()> {
    eprintln!(
        "jekko providers login {}: pending Packet D provider integration",
        args.provider
    );
    Ok(())
}

fn logout(args: &ProviderToggleArgs) -> Result<()> {
    eprintln!(
        "jekko providers logout {}: pending Packet D provider integration",
        args.provider
    );
    Ok(())
}
