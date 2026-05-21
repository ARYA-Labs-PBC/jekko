fn wait_for_gateway_ready(
    host: &str,
    port: u16,
    cancel: &CancellationToken,
) -> std::io::Result<()> {
    wait_for_gateway_ready_with_budget(
        host,
        port,
        cancel,
        GATEWAY_READY_TIMEOUT,
        GATEWAY_READY_RETRY_DELAY,
    )
}

fn wait_for_gateway_ready_with_budget(
    host: &str,
    port: u16,
    cancel: &CancellationToken,
    max_wait: Duration,
    retry_delay: Duration,
) -> std::io::Result<()> {
    tracing::debug!(
        host,
        port,
        max_wait_ms = max_wait.as_millis(),
        retry_delay_ms = retry_delay.as_millis(),
        "waiting for gateway readiness"
    );

    let started = Instant::now();
    let mut attempts: u32 = 0;

    loop {
        if cancel.is_cancelled() {
            tracing::debug!(host, port, attempts, "gateway readiness wait cancelled");
            return Err(std::io::Error::new(ErrorKind::Interrupted, "cancelled"));
        }

        match TcpStream::connect((host, port)) {
            Ok(_) => {
                tracing::debug!(
                    host,
                    port,
                    attempts,
                    elapsed_ms = started.elapsed().as_millis(),
                    "gateway readiness wait complete"
                );
                return Ok(());
            }
            Err(err) => {
                attempts += 1;

                let elapsed = started.elapsed();
                if elapsed >= max_wait {
                    tracing::debug!(
                        host,
                        port,
                        attempts,
                        elapsed_ms = elapsed.as_millis(),
                        error = %err,
                        "gateway readiness wait timed out"
                    );
                    return Err(err);
                }

                tracing::debug!(
                    host,
                    port,
                    attempts,
                    error = %err,
                    "gateway not ready yet"
                );

                let sleep_for = min(retry_delay, max_wait - elapsed);
                sleep_cancelable(cancel, sleep_for)?;
            }
        }
    }
}

fn sleep_cancelable(cancel: &CancellationToken, duration: Duration) -> std::io::Result<()> {
    let mut slept = Duration::ZERO;
    while slept < duration {
        if cancel.is_cancelled() {
            return Err(std::io::Error::new(ErrorKind::Interrupted, "cancelled"));
        }
        let step = min(GATEWAY_READY_CANCEL_POLL, duration - slept);
        std::thread::sleep(step);
        slept += step;
    }
    Ok(())
}
