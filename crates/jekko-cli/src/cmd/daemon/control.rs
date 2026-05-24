use std::fs::{self, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};
use jekko_runtime::daemon_transport::{log_path, pid_path, socket_path};

use super::args::{DaemonLogsArgs, DaemonStartArgs};
use super::metadata::read_metadata;
use super::{port_run, status_report};

pub(super) fn start(args: &DaemonStartArgs) -> Result<()> {
    if args.foreground {
        return foreground_loop();
    }
    if args.port_run.is_some() {
        return port_run::start_port_run(args);
    }
    let pid = read_pid().ok();
    if let Some(pid) = pid {
        if is_pid_alive(pid) {
            println!("jekko daemon already running (pid {pid})");
            return Ok(());
        }
    }
    prepare_daemon_dir()?;
    let log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path()?)
        .context("open daemon log")?;
    let exe = std::env::current_exe().context("resolve current jekko executable")?;
    let child = Command::new(exe)
        .args(["daemon", "start", "--foreground"])
        .stdin(Stdio::null())
        .stdout(Stdio::from(log.try_clone()?))
        .stderr(Stdio::from(log))
        .spawn()
        .context("spawn daemon foreground child")?;
    write_pid(child.id())?;
    append_log(&format!("spawned daemon child pid={}", child.id()))?;
    println!("jekko daemon started");
    println!("pid: {}", child.id());
    println!("socket: {}", socket_path()?.display());
    println!("log: {}", log_path()?.display());
    Ok(())
}

pub(super) fn stop() -> Result<()> {
    let pid = read_pid().ok();
    let metadata = read_metadata().ok();
    let path = pid_path()?;
    if path.exists() {
        fs::remove_file(&path).with_context(|| format!("remove {}", path.display()))?;
    }
    match pid {
        Some(pid) => {
            let _ = Command::new("kill")
                .arg("-TERM")
                .arg(pid.to_string())
                .status();
            append_log(&format!("stop requested for pid={pid}"))?;
            if let Some(meta) = metadata.as_ref() {
                status_report::mark_durable_run_stopped(meta, "stopped")?;
            }
            println!("jekko daemon stop requested (pid {pid})");
        }
        None => {
            println!("jekko daemon not running");
        }
    }
    Ok(())
}

pub(super) fn logs(args: &DaemonLogsArgs) -> Result<()> {
    let path = match read_metadata().ok().and_then(|meta| meta.event_log_path()) {
        Some(path) if path.exists() => path,
        _ => log_path()?,
    };
    let mut printed = print_tail(&path, args.lines)?;
    if args.follow {
        loop {
            thread::sleep(Duration::from_secs(1));
            let text = fs::read_to_string(&path).unwrap_or_default();
            let lines: Vec<&str> = text.lines().collect();
            for line in lines.iter().skip(printed) {
                println!("{line}");
            }
            printed = lines.len();
        }
    }
    Ok(())
}

fn foreground_loop() -> Result<()> {
    prepare_daemon_dir()?;
    let pid = std::process::id();
    write_pid(pid)?;
    append_log(&format!("daemon foreground loop started pid={pid}"))?;
    loop {
        thread::sleep(Duration::from_secs(1));
        match read_pid() {
            Ok(current) if current == pid && is_pid_alive(pid) => {}
            _ => break,
        }
    }
    append_log(&format!("daemon foreground loop stopped pid={pid}"))?;
    Ok(())
}

pub(super) fn prepare_daemon_dir() -> Result<()> {
    if let Some(parent) = pid_path()?.parent() {
        fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    Ok(())
}

pub(super) fn write_pid(pid: u32) -> Result<()> {
    let path = pid_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    fs::write(&path, pid.to_string()).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

pub(super) fn read_pid() -> Result<u32> {
    let text = fs::read_to_string(pid_path()?).context("read daemon pid")?;
    Ok(text.trim().parse::<u32>()?)
}

pub(super) fn is_pid_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    Command::new("sh")
        .arg("-c")
        .arg(format!("kill -0 {pid} 2>/dev/null"))
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub(super) fn append_log(message: &str) -> Result<()> {
    let path = log_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("open {}", path.display()))?;
    writeln!(file, "{} {message}", super::metadata::now_secs())?;
    Ok(())
}

fn print_tail(path: &std::path::Path, limit: usize) -> Result<usize> {
    let mut file = match OpenOptions::new().read(true).open(path) {
        Ok(file) => file,
        Err(_) => return Ok(0),
    };
    let _ = file.seek(SeekFrom::Start(0));
    let text = fs::read_to_string(path).unwrap_or_default();
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.len().saturating_sub(limit);
    for line in &lines[start..] {
        println!("{line}");
    }
    Ok(lines.len())
}
