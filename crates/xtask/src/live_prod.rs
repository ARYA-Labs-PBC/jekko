use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::path::Path;

use crate::shared::{
    candidate_home_env_files, existing_env_keys, home_dir, host_binary_path,
    live_prod_allowed_keys, parse_env_line, read_env_file, run_cargo_test,
};

const DEFAULT_ENV_FILE_TEXT: &str = "";
const DEFAULT_TUI_FLAG: &str = "1";

pub(crate) fn live_prod_init() -> Result<()> {
    let home = home_dir()?;
    let destination = live_prod_env_path()?;
    let mut discovered = HashMap::<String, String>::new();
    for file in candidate_home_env_files(&home)? {
        let text = match std::fs::read_to_string(&file) {
            Ok(value) => value,
            Err(_) => DEFAULT_ENV_FILE_TEXT.to_string(),
        };
        for line in text.lines() {
            if let Some((key, value)) = parse_env_line(line, &live_prod_allowed_keys()) {
                let assignment = format!("{key}={}", value.trim());
                discovered.entry(key).or_insert(assignment);
            }
        }
    }

    if discovered.is_empty() {
        println!("No approved Jekko/Jnoccio keys found in ~/*.env, ~/.env, or ~/.*.env. No changes made.");
        return Ok(());
    }

    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create env directory {parent:?}"))?;
    }
    let current = match std::fs::read_to_string(&destination) {
        Ok(value) => value,
        Err(_) => DEFAULT_ENV_FILE_TEXT.to_string(),
    };
    let current_keys = existing_env_keys(&current);
    let additions: Vec<_> = discovered
        .into_iter()
        .filter(|(key, _)| !current_keys.contains(key))
        .collect();

    if additions.is_empty() {
        println!(
            "Live prod env already has approved keys: {}",
            destination.display()
        );
        let mut keys: Vec<_> = current_keys.into_iter().collect();
        keys.sort();
        for key in keys {
            println!("{key}=<redacted>");
        }
        return Ok(());
    }

    let mut output = String::new();
    if current.trim().is_empty() {
        output.push_str("# Local live production TUI test keys. Do not commit this file.\n");
    } else {
        output.push_str(&current);
        if !current.ends_with('\n') {
            output.push('\n');
        }
    }
    for (_, assignment) in &additions {
        output.push_str(assignment);
        output.push('\n');
    }
    std::fs::write(&destination, output)
        .with_context(|| format!("write env file {destination:?}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&destination, std::fs::Permissions::from_mode(0o600));
    }

    println!("Updated local live prod env: {}", destination.display());
    for (key, _) in additions {
        println!("{key}=<redacted>");
    }
    Ok(())
}

fn live_prod_env_path() -> Result<std::path::PathBuf> {
    if let Some(path) = std::env::var_os("JEKKO_LIVE_PROD_ENV") {
        return Ok(std::path::PathBuf::from(path));
    }
    Ok(home_dir()?
        .join(".config")
        .join("jekko")
        .join("live-prod.env"))
}

pub(crate) fn live_prod() -> Result<()> {
    if std::env::var("CI").as_deref() == Ok("true") {
        bail!("Refusing to run live production TUI tests in CI");
    }

    let env_path = live_prod_env_path()?;
    if !env_path.exists() {
        bail!(
            "Local live prod env file is missing: {}\nRun: just tui-live-prod-init",
            env_path.display()
        );
    }

    let file_env = read_env_file(&env_path)?;
    let binary = match std::env::var("JEKKO_BIN").ok() {
        Some(value) => value,
        None => match file_env.get("JEKKO_BIN").cloned() {
            Some(value) => value,
            None => host_binary_path()?,
        },
    };
    if !Path::new(&binary).exists() {
        bail!("Jekko binary is missing at {binary}. Run: just jekko-build-host-fast");
    }
    if std::env::var("JEKKO_API_KEY")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .is_none()
        && file_env
            .get("JEKKO_API_KEY")
            .map(|value| value.trim().is_empty())
            .unwrap_or(true)
    {
        bail!(
            "JEKKO_API_KEY is required in {} or the process environment",
            env_path.display()
        );
    }

    let default_unlock_path = home_dir()?.join("jnoccio-fusion.unlock");
    let unlock_path = match file_env.get("JNOCCIO_UNLOCK_SECRET_PATH").cloned() {
        Some(value) => value,
        None => match std::env::var("JNOCCIO_UNLOCK_SECRET_PATH").ok() {
            Some(value) => value,
            None => default_unlock_path.display().to_string(),
        },
    };
    let has_unlock_secret = Path::new(&unlock_path).exists();

    let mut env = HashMap::new();
    for (key, value) in std::env::vars() {
        env.insert(key, value);
    }
    for (key, value) in file_env {
        env.insert(key, value);
    }
    env.insert("JEKKO_BIN".to_string(), binary.clone());
    env.insert("JEKKO_TUI_LIVE_PROD".to_string(), "1".to_string());
    let jnoccio_tui_test = match env.get("JNOCCIO_TUI_TEST").cloned() {
        Some(value) => value,
        None => DEFAULT_TUI_FLAG.to_string(),
    };
    env.insert("JNOCCIO_TUI_TEST".to_string(), jnoccio_tui_test);
    if has_unlock_secret {
        env.insert(
            "JNOCCIO_UNLOCK_SECRET_PATH".to_string(),
            unlock_path.clone(),
        );
        let tuiwright_flag = match env.get("JNOCCIO_TUIWRIGHT_E2E").cloned() {
            Some(value) => value,
            None => DEFAULT_TUI_FLAG.to_string(),
        };
        env.insert("JNOCCIO_TUIWRIGHT_E2E".to_string(), tuiwright_flag);
    }

    println!("Using local live prod env: {}", env_path.display());
    println!("Using Jekko binary: {binary}");
    println!("JEKKO_API_KEY=<redacted>");
    if has_unlock_secret {
        println!("JNOCCIO_UNLOCK_SECRET_PATH={unlock_path}");
    } else {
        println!(
            "Jnoccio unlock secret not found at {unlock_path}; unlock-specific TUI test will be skipped"
        );
    }

    run_cargo_test(
        "live Jekko TUI prompt",
        &[
            "live_jekko_prompt_round_trips_through_tui",
            "--",
            "--ignored",
            "--nocapture",
        ],
        &env,
    )?;
    run_cargo_test(
        "Jnoccio dashboard TUI checks",
        &["jnoccio_", "--", "--ignored", "--nocapture"],
        &env,
    )?;
    if has_unlock_secret {
        run_cargo_test(
            "Jnoccio unlock PTY check",
            &[
                "jekko_tui_paste_unlocks_jnoccio_fusion",
                "--",
                "--nocapture",
            ],
            &env,
        )?;
    }
    Ok(())
}
