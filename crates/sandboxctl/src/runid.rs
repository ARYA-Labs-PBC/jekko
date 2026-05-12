//! Run-ID generation: `YYYYMMDDTHHMMSSZ-<ulid-suffix>`.
//!
//! Sortable + collision-resistant. Embeds an 8-char ulid suffix so parallel
//! `create` calls from the same wall-clock second don't collide.

use std::time::{SystemTime, UNIX_EPOCH};

use ulid::Ulid;

pub fn generate() -> String {
    // System clock running before UNIX_EPOCH would represent host clock
    // corruption, not a recoverable state; we clamp to epoch and continue so
    // the run-id stays sortable.
    let now = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(d) => d,
        Err(_) => std::time::Duration::ZERO,
    };
    let secs = now.as_secs() as i64;
    let ulid = Ulid::new().to_string();
    // Last 8 chars of ulid is plenty of entropy for the run-id suffix.
    let suffix = &ulid[ulid.len().saturating_sub(8)..];
    let datetime = format_iso(secs);
    format!("{datetime}-{suffix}")
}

fn format_iso(secs: i64) -> String {
    // Minimal ISO-8601 formatter without pulling in chrono. Good for UTC only.
    const SECS_PER_DAY: i64 = 86_400;
    let days = secs.div_euclid(SECS_PER_DAY);
    let secs_of_day = secs.rem_euclid(SECS_PER_DAY);
    let (year, month, day) = civil_from_days(days);
    let hour = (secs_of_day / 3600) as u32;
    let minute = ((secs_of_day % 3600) / 60) as u32;
    let second = (secs_of_day % 60) as u32;
    format!("{year:04}{month:02}{day:02}T{hour:02}{minute:02}{second:02}Z")
}

// Howard Hinnant's civil_from_days, adapted: days since 1970-01-01.
fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn shape_is_stable() {
        let id = generate();
        assert_eq!(id.len(), 16 + 1 + 8, "expected YYYYMMDDTHHMMSSZ-XXXXXXXX, got {id}");
        assert!(id.contains('T'));
        assert!(id.contains('-'));
    }

    #[test]
    fn parallel_generation_is_unique() {
        let mut seen = HashSet::new();
        for _ in 0..1000 {
            seen.insert(generate());
        }
        assert_eq!(seen.len(), 1000, "all 1000 run-ids must be unique");
    }

    #[test]
    fn civil_epoch() {
        let (y, m, d) = civil_from_days(0);
        assert_eq!((y, m, d), (1970, 1, 1));
    }
}
