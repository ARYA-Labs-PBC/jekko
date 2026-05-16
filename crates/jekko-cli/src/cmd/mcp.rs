//! `jekko mcp` — Model Context Protocol server management.
//!
//! Mirrors `packages/jekko/src/cli/cmd/mcp.ts`.

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::cli::GlobalOpts;

#[derive(Args, Debug)]
pub struct McpArgs {
    #[command(subcommand)]
    pub command: McpCommand,
}

#[derive(Subcommand, Debug)]
pub enum McpCommand {
    /// List configured MCP servers.
    List,
    /// Attach a new MCP server to the active config.
    Attach(McpAttachArgs),
    /// Detach an MCP server.
    Detach(McpNameArgs),
    /// Show server status.
    Status(McpNameArgs),
}

#[derive(Args, Debug)]
pub struct McpAttachArgs {
    /// MCP server name.
    pub name: String,
    /// Command to run for stdio transport, or URL for SSE transport.
    pub target: String,
}

#[derive(Args, Debug)]
pub struct McpNameArgs {
    /// MCP server name.
    pub name: String,
}

pub fn run(_global: &GlobalOpts, args: &McpArgs) -> Result<()> {
    match &args.command {
        McpCommand::List => list(),
        McpCommand::Attach(opts) => attach(opts),
        McpCommand::Detach(opts) => detach(opts),
        McpCommand::Status(opts) => status(opts),
    }
}

fn list() -> Result<()> {
    eprintln!("jekko mcp list: pending MCP integration");
    Ok(())
}

fn attach(args: &McpAttachArgs) -> Result<()> {
    eprintln!(
        "jekko mcp attach {} {}: pending MCP integration",
        args.name, args.target
    );
    Ok(())
}

fn detach(args: &McpNameArgs) -> Result<()> {
    eprintln!("jekko mcp detach {}: pending MCP integration", args.name);
    Ok(())
}

fn status(args: &McpNameArgs) -> Result<()> {
    eprintln!("jekko mcp status {}: pending MCP integration", args.name);
    Ok(())
}
