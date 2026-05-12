use std::path::PathBuf;

use anyhow::Result;
use clap::Args as ClapArgs;

use crate::exit;
use crate::index;
use crate::wrapper;

#[derive(Debug, ClapArgs)]
pub struct Args {
    pub run_id: String,
    /// Override the patch output path (default: <root>/patch.diff).
    #[arg(long)]
    pub out: Option<PathBuf>,
    /// Include untracked-non-ignored files as a separate tar.
    #[arg(long)]
    pub include_untracked: bool,
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
    let patch_path = match args.out.clone() {
        Some(p) => p,
        None => entry.root.join("patch.diff"),
    };
    let report = wrapper::export_patch(&entry.root, &patch_path, args.include_untracked)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!(
            "sandboxctl: wrote {} ({} hunks, {} changed file(s))",
            report.patch_path.display(),
            report.hunks,
            report.changed_files
        );
        if let Some(extra) = &report.untracked_tar {
            println!("sandboxctl: untracked → {}", extra.display());
        }
    }
    Ok(exit::OK)
}
