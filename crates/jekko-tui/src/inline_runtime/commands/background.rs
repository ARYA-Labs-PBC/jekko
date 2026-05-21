fn cancel_level_label(level: CancelLevel) -> &'static str {
    match level {
        CancelLevel::None => "none",
        CancelLevel::Soft => "soft interrupt",
        CancelLevel::Hard => "hard stop",
        CancelLevel::Force => "force kill",
    }
}

/// T-BG-COUNT-MANAGER: extract trailing args from a slash command line.
///
/// Strips the leading `/<id>` and returns the trimmed remainder, e.g.
/// `/stop 12` → `"12"`, `/ps` → `""`. Tolerates extra whitespace and the
/// leading `/` being absent (defensive — slash dispatch only fires when the
/// line started with `/`, but keeping this resilient avoids panics in tests
/// that synthesize the input).
fn slash_args(line: &str, id: &str) -> String {
    let trimmed = line.trim_start();
    let without_slash = trimmed.strip_prefix('/').unwrap_or(trimmed);
    let without_id = without_slash.strip_prefix(id).unwrap_or(without_slash);
    without_id.trim().to_string()
}

/// T-PERMISSIONS-PLUMB / T-BG-RUN: parse the trailing argument of `/run`,
/// extracting the optional `--background` / `--bg` flag.
///
/// Returns `(background, cmd)` where `background` indicates the user requested
/// detached execution and `cmd` is the trimmed shell-command tail.
fn parse_run_args(args: &str) -> (bool, &str) {
    let trimmed = args.trim();
    if let Some(rest) = trimmed.strip_prefix("--background ") {
        (true, rest.trim())
    } else if let Some(rest) = trimmed.strip_prefix("--bg ") {
        (true, rest.trim())
    } else if trimmed == "--background" || trimmed == "--bg" {
        // Flag without a command — treat the cmd as empty so the dispatcher
        // emits the "missing command" notice instead of silently spawning
        // `sh -c ""`.
        (true, "")
    } else {
        (false, trimmed)
    }
}

/// T-PERMISSIONS-PLUMB / T-BG-RUN: run a shell command as a backgrounded job.
///
/// Wraps the user-supplied command in `sh -c "<cmd>"` so shell features
/// (pipes, redirects, globs) work as expected, polls the
/// [`CancellationToken`] alongside `child.wait`, and returns the matching
/// [`JobStatus`]. Caller (the `/run --background` dispatcher) is responsible
/// for handing the status back to [`BackgroundJobManager::finalize`].
async fn run_background_shell(cmd: String, cancel: CancellationToken) -> JobStatus {
    let mut child = match tokio::process::Command::new("sh")
        .arg("-c")
        .arg(&cmd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(err) => return JobStatus::Failed(format!("spawn: {err}")),
    };
    let mut cancelled = false;
    loop {
        tokio::select! {
            wait = child.wait() => {
                return match wait {
                    Ok(status) => {
                        if cancelled {
                            JobStatus::Cancelled
                        } else if status.success() {
                            JobStatus::Completed
                        } else {
                            JobStatus::Failed(format!(
                                "exit {}",
                                status.code().unwrap_or(-1)
                            ))
                        }
                    }
                    Err(err) => JobStatus::Failed(format!("wait: {err}")),
                };
            }
            _ = tokio::time::sleep(Duration::from_millis(100)) => {
                if cancel.is_cancelled() && !cancelled {
                    let _ = child.start_kill();
                    cancelled = true;
                }
            }
        }
    }
}

/// T-BG-COUNT-MANAGER: format the `/ps` notice body for a snapshot of
/// background jobs. Separated so tests can assert on the rendered text
/// without having to spin up the whole runtime.
fn render_ps_body(jobs: &[crate::background::JobSummary]) -> String {
    if jobs.is_empty() {
        return String::from("no background jobs running");
    }
    let mut body = String::from("background jobs:\n");
    for job in jobs {
        let status_label = match &job.status {
            JobStatus::Running => Cow::Borrowed("running"),
            JobStatus::Completed => Cow::Borrowed("done"),
            JobStatus::Cancelled => Cow::Borrowed("cancelled"),
            JobStatus::Failed(err) => Cow::Owned(format!("failed: {err}")),
        };
        let pid = match job.pid {
            Some(pid) => format!(" (pid {pid})"),
            None => String::new(),
        };
        body.push_str(&format!(
            "  [{}] {}{} · {} · {}\n",
            job.id,
            job.name,
            pid,
            elapsed_label(job.elapsed),
            status_label,
        ));
    }
    body
}
