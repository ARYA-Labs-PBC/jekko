use anyhow::Result;
use clap::Args as ClapArgs;

use crate::exit;
use crate::index;

#[derive(Debug, ClapArgs)]
pub struct Args {
    #[arg(long)]
    pub active: bool,
}

pub fn run(args: &Args, json: bool) -> Result<i32> {
    let sandbox_root = crate::cli::status_root();
    let entries = index::all(&sandbox_root)?;
    let filtered: Vec<_> = entries
        .into_iter()
        .filter(|e| !args.active || e.status == "ready")
        .collect();
    if json {
        println!("{}", serde_json::to_string_pretty(&filtered)?);
    } else if filtered.is_empty() {
        println!("(no sandboxes)");
    } else {
        for e in filtered {
            println!(
                "{}  lane={}  backend={}  status={}  root={}",
                e.run_id,
                e.lane_name,
                e.backend,
                e.status,
                e.root.display()
            );
        }
    }
    Ok(exit::OK)
}
