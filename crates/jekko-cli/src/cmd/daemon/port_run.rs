use std::process::{Command, Stdio};

use anyhow::{Context, Result};
use jekko_runtime::daemon_transport::log_path;

use super::args::DaemonStartArgs;
use super::control::{append_log, prepare_daemon_dir, write_pid};
use super::metadata::{
    now_secs, random_run_id, resolve_runner_bin, write_metadata, DaemonMetadata,
};

pub(super) fn start_port_run(args: &DaemonStartArgs) -> Result<()> {
    let config = args
        .port_run
        .as_ref()
        .context("--port-run requires a config path")?;
    let repo = match args.repo.clone() {
        Some(repo) => repo,
        None => std::env::current_dir().context("resolve current directory")?,
    };
    let run_id = args.run_id.clone().unwrap_or_else(random_run_id);
    prepare_daemon_dir()?;
    let log = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path()?)
        .context("open daemon log")?;
    let runner = resolve_runner_bin()?;
    let mut command = Command::new(&runner);
    command
        .arg("--repo")
        .arg(&repo)
        .arg("--run-id")
        .arg(&run_id)
        .arg("port-run")
        .arg("--config")
        .arg(config);
    if args.live {
        command.arg("--live");
    }
    if let Some(provider) = args.provider.as_deref() {
        command.arg("--provider").arg(provider);
    }
    if let Some(model) = args.model.as_deref() {
        command.arg("--model").arg(model);
    }
    if let Some(max_ticks) = args.max_ticks {
        command.arg("--max-ticks").arg(max_ticks.to_string());
    } else if args.forever || args.port_run.is_some() {
        command.arg("--forever");
    }
    command
        .arg("--tick-interval-secs")
        .arg(args.tick_interval_secs.to_string());
    if let Some(stop_file) = args.stop_file.as_ref() {
        command.arg("--stop-file").arg(stop_file);
    }
    let child = command
        .stdin(Stdio::null())
        .stdout(Stdio::from(log.try_clone()?))
        .stderr(Stdio::from(log))
        .spawn()
        .with_context(|| format!("spawn {}", runner.display()))?;
    write_pid(child.id())?;
    let meta = DaemonMetadata {
        pid: child.id(),
        kind: "port_run".to_string(),
        run_id: Some(run_id.clone()),
        repo: Some(repo.clone()),
        port_config: Some(config.clone()),
        started_at: now_secs(),
    };
    write_metadata(&meta)?;
    append_log(&format!(
        "spawned port run pid={} run_id={} repo={} config={}",
        child.id(),
        run_id,
        repo.display(),
        config.display()
    ))?;
    println!("jekko daemon started port run");
    println!("pid: {}", child.id());
    println!("run_id: {run_id}");
    println!("events: {}", meta.event_log_path().unwrap().display());
    println!("log: {}", log_path()?.display());
    Ok(())
}
