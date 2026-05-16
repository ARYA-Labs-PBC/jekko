use anyhow::{Context, Result};
use serde::Serialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MigrationRecord {
    pub sql: String,
    pub timestamp: i64,
    pub name: String,
}

pub fn collect(root: &Path) -> Result<Vec<MigrationRecord>> {
    let mut migrations = Vec::new();
    for entry in fs::read_dir(root).with_context(|| format!("read {}", root.display()))? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !file_type.is_dir() || !looks_like_timestamped_migration(&name) {
            continue;
        }

        let sql_path = entry.path().join("migration.sql");
        let sql = fs::read_to_string(&sql_path)
            .with_context(|| format!("read {}", sql_path.display()))?;
        migrations.push(MigrationRecord {
            sql,
            timestamp: parse_migration_timestamp(&name),
            name: name.into_owned(),
        });
    }
    migrations.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(migrations)
}

pub fn parse_migration_timestamp(name: &str) -> i64 {
    if name.len() < 14 || !name.as_bytes()[..14].iter().all(u8::is_ascii_digit) {
        return 0;
    }
    let year = name[0..4].parse::<i32>().ok();
    let month = name[4..6].parse::<u32>().ok();
    let day = name[6..8].parse::<u32>().ok();
    let hour = name[8..10].parse::<u32>().ok();
    let minute = name[10..12].parse::<u32>().ok();
    let second = name[12..14].parse::<u32>().ok();

    let (Some(year), Some(month), Some(day), Some(hour), Some(minute), Some(second)) =
        (year, month, day, hour, minute, second)
    else {
        return 0;
    };

    chrono::NaiveDate::from_ymd_opt(year, month, day)
        .and_then(|date| date.and_hms_opt(hour, minute, second))
        .map(|dt| dt.and_utc().timestamp_millis())
        .unwrap_or(0)
}

fn looks_like_timestamped_migration(name: &str) -> bool {
    name.as_bytes()
        .get(..14)
        .is_some_and(|prefix| prefix.iter().all(u8::is_ascii_digit))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn timestamp_parser_returns_epoch_for_invalid_names() {
        assert_eq!(parse_migration_timestamp("abc"), 0);
        assert_eq!(parse_migration_timestamp("202401"), 0);
    }

    #[test]
    fn timestamp_parser_reads_valid_prefix() {
        assert_eq!(
            parse_migration_timestamp("20260512200000_example"),
            1_778_616_000_000
        );
    }

    #[test]
    fn collect_finds_timestamped_migrations() {
        let dir = tempdir().unwrap();
        let mig = dir.path().join("20260512200000_demo");
        fs::create_dir_all(&mig).unwrap();
        fs::write(mig.join("migration.sql"), "select 1;").unwrap();
        let records = collect(dir.path()).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].name, "20260512200000_demo");
        assert_eq!(records[0].sql, "select 1;");
        assert_eq!(records[0].timestamp, 1_778_616_000_000);
    }
}
