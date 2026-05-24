use std::fs;

use anyhow::Result;
use jekko_provider::key_pool::discover_user_dirs;
use jekko_provider::setup::parse_env_lines;

use super::args::KeysUsersArgs;

pub(super) fn users(args: &KeysUsersArgs) -> Result<()> {
    let unlocked = jekko_jnoccio_boot::unlock::is_unlocked();
    let dirs = discover_user_dirs(unlocked);
    if args.json {
        let rows: Vec<_> = dirs
            .iter()
            .map(|d| {
                let count = fs::read_to_string(&d.llm_env_path)
                    .map(|t| parse_env_lines(&t).len())
                    .unwrap_or(0);
                serde_json::json!({
                    "user": d.user_id,
                    "path": d.llm_env_path.display().to_string(),
                    "exists": d.llm_env_path.is_file(),
                    "keys": count,
                })
            })
            .collect();
        let blob = serde_json::json!({
            "unlocked": unlocked,
            "users": rows,
        });
        println!("{}", serde_json::to_string_pretty(&blob)?);
        return Ok(());
    }
    println!("unlocked: {unlocked}");
    for d in &dirs {
        let count = fs::read_to_string(&d.llm_env_path)
            .map(|t| parse_env_lines(&t).len())
            .unwrap_or(0);
        let exists = if d.llm_env_path.is_file() {
            "ok"
        } else {
            "missing"
        };
        println!(
            "  {user:<10} {count:>3} keys  [{exists}]  {path}",
            user = d.user_id,
            path = d.llm_env_path.display()
        );
    }
    if !unlocked {
        println!("(set JNOCCIO_DEVELOPER_KEY to enable multi-user balancing)");
    }
    Ok(())
}
