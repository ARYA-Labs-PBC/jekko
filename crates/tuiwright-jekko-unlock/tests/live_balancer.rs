//! LOCAL ONLY live smoke for the multi-user key balancer.
//!
//! Spawns the installed `jekko run` binary N times against a real upstream
//! provider, with `RUST_LOG=jekko_runtime::agent::provider=debug` so the
//! `selected credential from key pool` tracing line surfaces. Asserts that
//! both `user_1` and `user_2` were picked across the loop, proving the
//! [`jekko_runtime::key_balancer::KeyBalancer`] is wired end-to-end.
//!
//! Gates (all required):
//! - `JEKKO_LIVE_BALANCER=1`
//! - `JEKKO_BIN` pointing at the binary to test (default
//!   `/opt/homebrew/bin/jekko` when unset)
//! - real keys at both `~/.jekko/users/user_1/llm.env` and
//!   `~/.jekko/users/user_2/llm.env`
//! - jnoccio unlocked (auto-true inside the repo via plaintext signals)
//!
//! Never runs in CI: refuses to start when `CI=true`.

use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use anyhow::{bail, Context, Result};

/// Default model id for the smoke run. OpenRouter free-tier is broadly
/// available so it's the lowest-friction default; override with
/// `JEKKO_LIVE_BALANCER_PROVIDER` + `JEKKO_LIVE_BALANCER_MODEL`.
const DEFAULT_PROVIDER: &str = "openrouter";
const DEFAULT_MODEL: &str = "openrouter-gpt-oss-120b-free";
const DEFAULT_LOOP_COUNT: usize = 6;

fn enabled() -> bool {
    std::env::var("JEKKO_LIVE_BALANCER").as_deref() == Ok("1")
}

fn jekko_bin() -> PathBuf {
    std::env::var_os("JEKKO_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/opt/homebrew/bin/jekko"))
}

fn loop_count() -> usize {
    std::env::var("JEKKO_LIVE_BALANCER_COUNT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_LOOP_COUNT)
}

#[test]
#[ignore = "local-only live smoke; opt in with JEKKO_LIVE_BALANCER=1"]
fn balancer_distributes_across_user_1_and_user_2() -> Result<()> {
    if !enabled() {
        eprintln!("skipped: set JEKKO_LIVE_BALANCER=1 to run");
        return Ok(());
    }
    if std::env::var("CI").as_deref() == Ok("true") {
        bail!("live balancer smoke must not run in CI");
    }

    let bin = jekko_bin();
    if !bin.is_file() {
        bail!(
            "jekko binary missing at {} — build + install first (scripts/live-balancer-smoke.sh)",
            bin.display()
        );
    }

    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .context("HOME must be set")?;
    let user_1 = home
        .join(".jekko")
        .join("users")
        .join("user_1")
        .join("llm.env");
    let user_2 = home
        .join(".jekko")
        .join("users")
        .join("user_2")
        .join("llm.env");
    for path in [&user_1, &user_2] {
        if !path.is_file() {
            bail!(
                "{} missing — populate both user_1 and user_2 llm.env before running",
                path.display()
            );
        }
    }

    let provider =
        std::env::var("JEKKO_LIVE_BALANCER_PROVIDER").unwrap_or_else(|_| DEFAULT_PROVIDER.into());
    let model = std::env::var("JEKKO_LIVE_BALANCER_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.into());
    let count = loop_count();

    let mut saw_user_1 = 0usize;
    let mut saw_user_2 = 0usize;
    let mut failures = 0usize;
    for i in 0..count {
        let out = Command::new(&bin)
            .arg("run")
            .arg("--provider")
            .arg(&provider)
            .arg("--model")
            .arg(&model)
            .arg("--ephemeral")
            .arg(format!("Say only the word ping #{i}"))
            .env("RUST_LOG", "jekko_runtime::agent::provider=debug")
            .env("JEKKO_PURE", "1")
            .output()
            .with_context(|| format!("spawn {}", bin.display()))?;

        let stderr = String::from_utf8_lossy(&out.stderr);
        if !out.status.success() {
            failures += 1;
            eprintln!(
                "[live-balancer] run #{i} failed (status {:?}):\n{stderr}",
                out.status.code()
            );
            continue;
        }
        if stderr.contains("user=user_1") {
            saw_user_1 += 1;
        }
        if stderr.contains("user=user_2") {
            saw_user_2 += 1;
        }
        eprintln!("[live-balancer] run #{i}: user_1_hits={saw_user_1} user_2_hits={saw_user_2}",);
        std::thread::sleep(Duration::from_millis(250));
    }

    if failures > count / 2 {
        bail!("too many upstream failures ({failures}/{count}); check provider+model+keys");
    }
    if saw_user_1 == 0 {
        bail!("balancer never picked user_1 across {count} runs — selection is not balanced");
    }
    if saw_user_2 == 0 {
        bail!("balancer never picked user_2 across {count} runs — selection is not balanced");
    }
    eprintln!(
        "[live-balancer] OK — user_1 picked {saw_user_1}x, user_2 picked {saw_user_2}x over {count} runs"
    );
    Ok(())
}
