//! `jekko watch <run_id>` — tail a ZYAL run's NDJSON event stream and emit
//! per-tick snapshots + remediation actions.
//!
//! Wires Phase G1's pure `fold_events` + `detect_and_remediate` (in the
//! `jankurai-runner` crate) to a shippable operator surface. The watcher
//! opens `target/zyal/runs/<run_id>/events.jsonl`, reads any existing lines,
//! and then either exits (`--once`) or follows the file via the `notify`
//! crate, re-folding on every append batch.
//!
//! Three output formats:
//! - `plain` (default) — newline-delimited per-event summary plus any
//!   triggered remediation rules.
//! - `json` — pretty-printed `{snapshot, actions}` JSON object per tick.
//! - `tui` — placeholder for Phase G2's Ratatui dashboard; currently falls
//!   back to plain mode after printing a notice on stderr.
//!
//! All four remediation rules surface: `stall_detected`,
//! `provider_error_burst`, `parity_gaps_growing`, `jankurai_regression`.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, RecvTimeoutError};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use clap::{Args, ValueEnum};
use jankurai_runner::events::{run_event_file_rel, Event};
use jankurai_runner::watcher::{
    detect_and_remediate, fold_events, RemediationAction, RemediationRule, WatcherSnapshot,
};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;

use crate::cli::GlobalOpts;

/// `jekko watch <run_id>` arguments.
#[derive(Args, Debug)]
pub struct WatchArgs {
    /// Run id to watch. Looks up `target/zyal/runs/<run_id>/events.jsonl`
    /// relative to the current working directory (or the explicit
    /// `--repo-root`).
    #[arg(value_name = "RUN_ID")]
    pub run_id: String,

    /// Override the repo root used to locate the events file. Defaults to
    /// the current working directory.
    #[arg(long, value_name = "PATH")]
    pub repo_root: Option<PathBuf>,

    /// Output format.
    #[arg(long, value_enum, default_value_t = WatchFormat::Plain)]
    pub format: WatchFormat,

    /// Read the existing file once and exit without watching for new events.
    #[arg(long)]
    pub once: bool,

    /// Disable the default file-follow behavior. Equivalent to `--once`
    /// after the initial drain.
    #[arg(long = "no-follow")]
    pub no_follow: bool,

    /// Seconds without a progress event before `StallDetected` fires.
    #[arg(long = "stall-threshold", default_value_t = 300, value_name = "SECS")]
    pub stall_threshold: u64,

    /// Provider error rate (0.0..=1.0) that triggers
    /// `ProviderErrorBurst`.
    #[arg(
        long = "error-rate-threshold",
        default_value_t = 0.5,
        value_name = "FLOAT"
    )]
    pub error_rate_threshold: f64,
}

/// Output format for the watcher.
#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
#[value(rename_all = "lowercase")]
pub enum WatchFormat {
    /// Newline-delimited human-readable summary.
    Plain,
    /// Pretty JSON `{snapshot, actions}` per tick.
    Json,
    /// Placeholder for the Phase G2 Ratatui dashboard; falls back to plain.
    Tui,
}

/// Default debounce window between file-change events. Notify fires once per
/// platform-specific atomic write batch; we coalesce within this window so a
/// single 5-line append doesn't yield 5 separate ticks.
const DEBOUNCE: Duration = Duration::from_millis(150);

/// Entry point invoked from `main.rs`.
pub fn run(_global: &GlobalOpts, args: &WatchArgs) -> Result<()> {
    let repo_root = match args.repo_root.clone() {
        Some(p) => p,
        None => std::env::current_dir().context("resolving working directory")?,
    };
    let events_path = repo_root.join(run_event_file_rel(&args.run_id));

    let effective_format = match args.format {
        WatchFormat::Tui => {
            eprintln!("tui mode not yet implemented; falling back to plain");
            WatchFormat::Plain
        }
        other => other,
    };

    // Drain whatever already exists. The file may not be present yet — in
    // that case `read_from_offset` returns an empty initial tick.
    let mut offset: u64 = 0;
    let mut tick_state = TickState::default();
    let (new_events, new_offset) = read_from_offset(&events_path, offset)?;
    offset = new_offset;
    emit_tick(
        &new_events,
        &mut tick_state,
        args,
        effective_format,
        /* initial */ true,
    )?;

    if args.once || args.no_follow {
        return Ok(());
    }

    follow(&events_path, offset, &mut tick_state, args, effective_format)
}

/// Per-tick mutable state carried across iterations of the watch loop.
#[derive(Default)]
struct TickState {
    /// Rolling buffer of every event observed so far. We re-fold from
    /// scratch on every tick so the snapshot stays consistent even if a
    /// previous append failed mid-line.
    all_events: Vec<Event>,
    /// Most recent `parity_gaps_open` value, three ticks ago. We approximate
    /// the spec's "growing for 3 ticks" by comparing the current value
    /// against the value from up to three observations back.
    prior_gaps_history: Vec<i64>,
    /// Last `jankurai_hard_findings` value we observed; used to detect
    /// regression. None until the first audit event.
    prior_hard_findings: Option<i64>,
}

fn follow(
    path: &Path,
    mut offset: u64,
    state: &mut TickState,
    args: &WatchArgs,
    format: WatchFormat,
) -> Result<()> {
    // Watch the *parent* directory so we still receive events if the file
    // doesn't exist yet (notify can't subscribe to a missing path).
    let watch_dir = path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    if !watch_dir.exists() {
        std::fs::create_dir_all(&watch_dir)
            .with_context(|| format!("mkdir -p {}", watch_dir.display()))?;
    }

    let (tx, rx) = channel::<notify::Result<notify::Event>>();
    let mut watcher: RecommendedWatcher = notify::recommended_watcher(move |res| {
        let _ = tx.send(res);
    })
    .context("create file watcher")?;
    watcher
        .watch(&watch_dir, RecursiveMode::NonRecursive)
        .with_context(|| format!("watch {}", watch_dir.display()))?;

    loop {
        // Wait for a change. We also wake every 5s so stall detection can
        // fire even when there's no fs activity.
        let timeout = Duration::from_secs(5);
        match rx.recv_timeout(timeout) {
            Ok(Ok(_event)) => {
                // Coalesce burst of notifications within the debounce window
                // before reading the file.
                let deadline = std::time::Instant::now() + DEBOUNCE;
                while let Ok(remaining) = deadline
                    .checked_duration_since(std::time::Instant::now())
                    .ok_or(())
                {
                    match rx.recv_timeout(remaining) {
                        Ok(_) => continue,
                        Err(_) => break,
                    }
                }
            }
            Ok(Err(err)) => {
                eprintln!("watch error: {err}");
                continue;
            }
            Err(RecvTimeoutError::Timeout) => {
                // Periodic tick so stall rules still fire on a quiet stream.
            }
            Err(RecvTimeoutError::Disconnected) => break,
        }

        let (new_events, new_offset) = read_from_offset(path, offset)?;
        offset = new_offset;
        emit_tick(&new_events, state, args, format, /* initial */ false)?;

        if state.all_events.iter().any(|ev| {
            matches!(
                ev.kind,
                jankurai_runner::events::EventKind::RunFinished
            )
        }) {
            break;
        }
    }
    Ok(())
}

/// Read any new lines appended past `offset` and return the parsed events
/// plus the new offset. Lines that fail to parse are skipped (with a stderr
/// notice) so a single malformed event doesn't kill the watcher.
fn read_from_offset(path: &Path, offset: u64) -> Result<(Vec<Event>, u64)> {
    if !path.exists() {
        return Ok((Vec::new(), 0));
    }
    let mut file = File::open(path).with_context(|| format!("open {}", path.display()))?;
    let len = file.metadata()?.len();
    // File was rotated / truncated — restart from the beginning.
    let read_from = if offset > len { 0 } else { offset };
    file.seek(SeekFrom::Start(read_from))?;
    let reader = BufReader::new(file);
    let mut events = Vec::new();
    let mut consumed = read_from;
    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(err) => {
                eprintln!("read {}: {err}", path.display());
                break;
            }
        };
        // +1 for the newline that BufRead stripped.
        consumed = consumed.saturating_add(line.len() as u64 + 1);
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<Event>(&line) {
            Ok(ev) => events.push(ev),
            Err(err) => {
                eprintln!("skip malformed event line: {err}");
            }
        }
    }
    // Clamp consumed to the real file length so we don't drift past EOF if
    // the last line had no trailing newline.
    let consumed = consumed.min(len);
    Ok((events, consumed))
}

/// Compute the snapshot, run remediation, and emit per-format output for a
/// single batch of new events.
fn emit_tick(
    new_events: &[Event],
    state: &mut TickState,
    args: &WatchArgs,
    format: WatchFormat,
    initial: bool,
) -> Result<()> {
    state.all_events.extend(new_events.iter().cloned());
    let snap = fold_events(&state.all_events);

    let prior_gaps = if state.prior_gaps_history.len() >= 3 {
        Some(state.prior_gaps_history[state.prior_gaps_history.len() - 3])
    } else {
        None
    };

    let now_ts = now_epoch_secs();
    let actions = detect_and_remediate(
        &snap,
        now_ts,
        args.stall_threshold,
        args.error_rate_threshold,
        prior_gaps,
        state.prior_hard_findings,
    );

    match format {
        WatchFormat::Plain => emit_plain(new_events, &snap, &actions, initial)?,
        WatchFormat::Json => emit_json(&snap, &actions)?,
        WatchFormat::Tui => unreachable!("tui falls back to plain in run()"),
    }

    state.prior_gaps_history.push(snap.parity_gaps_open);
    if let Some(hf) = snap.last_jankurai_hard_findings {
        state.prior_hard_findings = Some(hf);
    }
    Ok(())
}

fn emit_plain(
    new_events: &[Event],
    snap: &WatcherSnapshot,
    actions: &[RemediationAction],
    initial: bool,
) -> Result<()> {
    if initial {
        println!(
            "watcher: starting (events_seen={}, lanes_started={}, lanes_finished={})",
            snap.lanes_started + snap.lanes_finished,
            snap.lanes_started,
            snap.lanes_finished
        );
    }
    for ev in new_events {
        let kind = serde_json::to_string(&ev.kind)
            .unwrap_or_else(|_| "\"unknown\"".to_string())
            .trim_matches('"')
            .to_string();
        println!("ts={} kind={} run={}", ev.ts, kind, ev.run_id);
    }
    println!(
        "snapshot: lanes={}/{} workers_pass={} workers_fail={} gaps_open={} model_attempts={} model_failures={} spend_usd={:.4}",
        snap.lanes_started,
        snap.lanes_finished,
        snap.workers_pass,
        snap.workers_fail,
        snap.parity_gaps_open,
        snap.model_attempts,
        snap.model_failures,
        snap.model_spend_usd
    );
    for action in actions {
        println!(
            "remediation: rule={} summary={}",
            rule_label(action.rule),
            action.summary
        );
        for (k, v) in &action.detail {
            println!("  {k}={v}");
        }
    }
    Ok(())
}

fn emit_json(snap: &WatcherSnapshot, actions: &[RemediationAction]) -> Result<()> {
    let payload = JsonTick {
        snapshot: SnapshotJson::from(snap),
        actions: actions.iter().map(ActionJson::from).collect(),
    };
    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}

/// Stable string label for a [`RemediationRule`] — matches the spec names
/// used in the per-tick output and in tests.
fn rule_label(rule: RemediationRule) -> &'static str {
    match rule {
        RemediationRule::StallDetected => "stall_detected",
        RemediationRule::ProviderErrorBurst => "provider_error_burst",
        RemediationRule::ParityGapsGrowing => "parity_gaps_growing",
        RemediationRule::JankuraiRegression => "jankurai_regression",
    }
}

fn now_epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[derive(Serialize)]
struct JsonTick {
    snapshot: SnapshotJson,
    actions: Vec<ActionJson>,
}

#[derive(Serialize)]
struct SnapshotJson {
    lanes_started: u64,
    lanes_finished: u64,
    workers_pass: u64,
    workers_fail: u64,
    parity_gaps_open: i64,
    parity_gaps_closed: u64,
    model_attempts: u64,
    model_failures: u64,
    errors_by_provider: BTreeMap<String, u64>,
    model_spend_usd: f64,
    last_progress_ts: Option<u64>,
    last_jankurai_score: Option<i64>,
    last_jankurai_hard_findings: Option<i64>,
    finished: bool,
    error_rate: f64,
}

impl From<&WatcherSnapshot> for SnapshotJson {
    fn from(snap: &WatcherSnapshot) -> Self {
        Self {
            lanes_started: snap.lanes_started,
            lanes_finished: snap.lanes_finished,
            workers_pass: snap.workers_pass,
            workers_fail: snap.workers_fail,
            parity_gaps_open: snap.parity_gaps_open,
            parity_gaps_closed: snap.parity_gaps_closed,
            model_attempts: snap.model_attempts,
            model_failures: snap.model_failures,
            errors_by_provider: snap.errors_by_provider.clone(),
            model_spend_usd: snap.model_spend_usd,
            last_progress_ts: snap.last_progress_ts,
            last_jankurai_score: snap.last_jankurai_score,
            last_jankurai_hard_findings: snap.last_jankurai_hard_findings,
            finished: snap.finished,
            error_rate: snap.error_rate(),
        }
    }
}

#[derive(Serialize)]
struct ActionJson {
    rule: &'static str,
    summary: String,
    detail: BTreeMap<String, String>,
}

impl From<&RemediationAction> for ActionJson {
    fn from(action: &RemediationAction) -> Self {
        Self {
            rule: rule_label(action.rule),
            summary: action.summary.clone(),
            detail: action.detail.clone(),
        }
    }
}
