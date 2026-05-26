//! jnoccio-fusion auto-boot helpers.
//!
//! Wraps the health probe + auto-spawn dance into a single
//! [`ensure_jnoccio_ready`] entry point used by the runtime before issuing
//! turns against the local jnoccio provider.

use std::path::Path;
use std::sync::OnceLock;
use std::time::Duration;

use tokio::time::sleep;

use crate::error::{RuntimeError, RuntimeResult};

static JNOCCIO_READY_GUARD: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();

fn jnoccio_ready_guard() -> &'static tokio::sync::Mutex<()> {
    JNOCCIO_READY_GUARD.get_or_init(|| tokio::sync::Mutex::new(()))
}

fn jnoccio_extra_port() -> Option<u16> {
    std::env::var("JNOCCIO_EXTRA_PORT")
        .ok()
        .and_then(|value| value.trim().parse::<u16>().ok())
}

pub(in crate::agent) async fn ensure_jnoccio_ready(start: &Path) -> RuntimeResult<()> {
    let _guard = jnoccio_ready_guard().lock().await;
    let extra_port = jnoccio_extra_port();
    let initial = tokio::task::spawn_blocking(move || {
        jekko_jnoccio_boot::health::probe_health_combined(extra_port)
    })
    .await
    .map_err(|err| RuntimeError::other(format!("jnoccio health probe failed: {err}")))?;
    if initial.reachable {
        return Ok(());
    }

    if !jekko_jnoccio_boot::unlock::is_unlocked() {
        return Err(RuntimeError::other(
            "jnoccio-fusion is not unlocked on this machine",
        ));
    }

    let Some(fusion_root) = jekko_jnoccio_boot::unlock::find_jnoccio_fusion_root_from(start) else {
        return Err(RuntimeError::other(
            "jnoccio-fusion checkout not found for runtime auto-boot",
        ));
    };

    tokio::task::spawn_blocking(move || jekko_jnoccio_boot::spawn::ensure_and_spawn(&fusion_root))
        .await
        .map_err(|err| RuntimeError::other(format!("jnoccio spawn failed: {err}")))?
        .map_err(|err| RuntimeError::other(format!("jnoccio spawn failed: {err}")))?;

    for _ in 0..6 {
        sleep(Duration::from_millis(1350)).await;
        let extra_port = jnoccio_extra_port();
        let result = tokio::task::spawn_blocking(move || {
            jekko_jnoccio_boot::health::probe_health_combined(extra_port)
        })
        .await
        .map_err(|err| RuntimeError::other(format!("jnoccio health probe failed: {err}")))?;
        if result.reachable {
            return Ok(());
        }
    }

    Err(RuntimeError::other(
        "jnoccio-fusion did not become reachable after restart",
    ))
}
