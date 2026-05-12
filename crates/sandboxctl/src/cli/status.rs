use anyhow::Result;
use clap::Args as ClapArgs;

use crate::exit;
use crate::index;
use crate::wrapper;

#[derive(Debug, ClapArgs)]
pub struct Args {
    pub run_id: String,
    #[arg(long, default_value_t = 10)]
    pub last: usize,
}

pub fn run(args: &Args, json: bool) -> Result<i32> {
    let sandbox_root = crate::cli::status_root();
    let entry = match index::find(&sandbox_root, &args.run_id)? {
        Some(e) => e,
        None => {
            eprintln!("sandboxctl: run_id '{}' not found", args.run_id);
            return Ok(exit::LANE_NOT_FOUND);
        }
    };

    let runs = wrapper::list_runs(&entry.root, args.last)?;
    if json {
        println!(
            "{}",
            serde_json::json!({
                "run_id": entry.run_id,
                "lane": entry.lane_name,
                "backend": entry.backend,
                "status": entry.status,
                "root": entry.root.display().to_string(),
                "runs": runs,
            })
        );
    } else {
        println!(
            "sandboxctl: run_id={} lane={} backend={} status={}",
            entry.run_id, entry.lane_name, entry.backend, entry.status
        );
        for r in runs {
            println!(
                "  {} exit={} duration_ms={} changed={} argv={}",
                r.cmd_id, r.exit_code, r.duration_ms, r.changed_files, r.argv.join(" ")
            );
        }
    }
    Ok(exit::OK)
}
