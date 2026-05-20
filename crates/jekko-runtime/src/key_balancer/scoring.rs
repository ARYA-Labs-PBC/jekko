fn score(usage: &KeyUsage, now: i64) -> f64 {
    if usage
        .cooldown_until
        .map(|until| until > now)
        .unwrap_or(false)
    {
        return 0.0;
    }
    let health = usage.status.weight();
    if health == 0.0 {
        return 0.0;
    }
    let load = 1.0 / (1.0 + usage.attempts as f64 / 20.0);
    let penalty = if usage.failures == 0 {
        1.0
    } else {
        (1.0 / (1.0 + usage.failures as f64 * 6.0)).clamp(0.01, 1.0)
    };
    (health * load * penalty).max(0.0001)
}

fn pick_weighted_index(weights: &[f64]) -> Option<usize> {
    let total: f64 = weights.iter().sum();
    if total <= 0.0 {
        return None;
    }
    let mut draw = rand::thread_rng().gen_range(0.0..total);
    for (i, w) in weights.iter().enumerate() {
        if draw < *w {
            return Some(i);
        }
        draw -= *w;
    }
    weights.iter().position(|w| *w > 0.0)
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
