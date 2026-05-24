use std::path::PathBuf;

use clap::{Args, Subcommand};

#[derive(Args, Debug)]
pub struct DaemonArgs {
    #[command(subcommand)]
    pub command: DaemonCommand,
}

#[derive(Subcommand, Debug)]
pub enum DaemonCommand {
    /// Start the daemon.
    Start(DaemonStartArgs),
    /// Stop the daemon.
    Stop,
    /// Print daemon status.
    Status,
    /// Tail daemon logs.
    Logs(DaemonLogsArgs),
}

#[derive(Args, Debug, Default)]
pub struct DaemonStartArgs {
    /// Detach into the background.
    #[arg(long)]
    pub detach: bool,
    /// Run the daemon loop in the foreground.
    #[arg(long, hide = true)]
    pub foreground: bool,
    /// Start a durable ZYAL port run from a JSON/TOML config.
    #[arg(long, value_name = "CONFIG")]
    pub port_run: Option<PathBuf>,
    /// Repository root for `--port-run`.
    #[arg(long, value_name = "PATH")]
    pub repo: Option<PathBuf>,
    /// Run id for `--port-run`.
    #[arg(long)]
    pub run_id: Option<String>,
    /// Use live model calls for `--port-run`.
    #[arg(long)]
    pub live: bool,
    /// Provider override for live `--port-run`.
    #[arg(long)]
    pub provider: Option<String>,
    /// Model override for live `--port-run`.
    #[arg(long)]
    pub model: Option<String>,
    /// Maximum port-run ticks.
    #[arg(long)]
    pub max_ticks: Option<u64>,
    /// Seconds between port-run ticks.
    #[arg(long, default_value_t = 30)]
    pub tick_interval_secs: u64,
    /// Stop-file path for the port runner.
    #[arg(long)]
    pub stop_file: Option<PathBuf>,
    /// Run the port runner until stopped.
    #[arg(long)]
    pub forever: bool,
}

#[derive(Args, Debug, Default)]
pub struct DaemonLogsArgs {
    /// Follow new log lines as they are appended.
    #[arg(long, short = 'f')]
    pub follow: bool,
    /// Number of trailing lines to print.
    #[arg(long, short = 'n', default_value_t = 80)]
    pub lines: usize,
}
