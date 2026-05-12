use anyhow::Result;
use clap::Args as ClapArgs;

use crate::backend;
use crate::exit;
use crate::index;

#[derive(Debug, ClapArgs)]
pub struct Args {
    pub run_id: String,
    #[arg(long)]
    pub keep_logs: bool,
    #[arg(long)]
    pub force: bool,
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

    let workspace = backend::Workspace {
        run_id: entry.run_id.clone(),
        root: entry.root.clone(),
        repo_subdir: std::path::PathBuf::from("workspace"),
    };
    let resolved = backend::Resolver::for_backend(&entry.backend)?;
    resolved.destroy(&workspace, args.keep_logs || !args.force)?;
    index::remove(&sandbox_root, &args.run_id)?;
    if json {
        println!("{}", serde_json::json!({"run_id": entry.run_id, "destroyed": true}));
    } else {
        println!("sandboxctl: destroyed {} (logs kept: {})", entry.run_id, args.keep_logs);
    }
    Ok(exit::OK)
}
