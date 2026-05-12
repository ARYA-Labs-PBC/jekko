use std::path::PathBuf;

use anyhow::Result;
use clap::Args as ClapArgs;

use crate::exit;

/// Thin alias for `zyalc compile` — kept for ergonomics so users who only
/// remember `sandboxctl` can still discover the compile flow.
#[derive(Debug, ClapArgs)]
pub struct Args {
    pub path: PathBuf,
    #[arg(long)]
    pub out: Option<PathBuf>,
    #[arg(long)]
    pub check: bool,
}

pub fn run(args: &Args, json: bool) -> Result<i32> {
    let mut cmd = std::process::Command::new("zyalc");
    cmd.arg("compile").arg(&args.path);
    if let Some(out) = &args.out {
        cmd.arg("--out").arg(out);
    }
    if args.check {
        cmd.arg("--check");
    }
    let status = cmd.status();
    match status {
        Ok(s) => {
            if json {
                println!(
                    "{}",
                    serde_json::json!({"forwarded": "zyalc compile", "code": s.code()})
                );
            }
            Ok(s.code().unwrap_or(exit::FS_ERROR))
        }
        Err(err) => {
            eprintln!("sandboxctl: zyalc not on PATH ({err}); run `cargo build -p zyalc` and retry");
            Ok(exit::FS_ERROR)
        }
    }
}
