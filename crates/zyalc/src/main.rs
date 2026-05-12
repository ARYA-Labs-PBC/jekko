use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use zyalc::compile;

#[derive(Parser, Debug)]
#[command(
    name = "zyalc",
    version,
    about = "Compile .zyal source files to TOML or GitHub Actions YAML"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Compile a single .zyal file or all known sources.
    Compile {
        /// Path to a .zyal file. Omit (with --all) to compile every registered source.
        path: Option<PathBuf>,
        /// Override the output path (single-file mode only).
        #[arg(long)]
        out: Option<PathBuf>,
        /// Compile every registered .zyal source under the repo (uses agent/generated-zones.toml).
        #[arg(long)]
        all: bool,
        /// Verify the existing target matches a freshly-compiled output; exit 1 if drifted.
        #[arg(long)]
        check: bool,
        /// Repo root (default: current working directory).
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
    /// Print the detected profile + emitted target for a `.zyal` file.
    Inspect {
        path: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();
    let code = match dispatch(&cli) {
        Ok(c) => c,
        Err(err) => {
            eprintln!("zyalc: {err:#}");
            1
        }
    };
    std::process::exit(code);
}

fn dispatch(cli: &Cli) -> Result<i32> {
    match &cli.cmd {
        Cmd::Compile {
            path,
            out,
            all,
            check,
            root,
        } => {
            if *all {
                let report = compile::compile_all(root, *check)
                    .context("compile --all from generated-zones.toml")?;
                if report.drifted.is_empty() {
                    println!(
                        "zyalc: {} compiled, {} unchanged",
                        report.compiled.len(),
                        report.unchanged.len()
                    );
                    Ok(0)
                } else {
                    eprintln!("zyalc: drift detected in {} target(s):", report.drifted.len());
                    for path in &report.drifted {
                        eprintln!("  - {}", path.display());
                    }
                    Ok(1)
                }
            } else {
                let path = path
                    .as_ref()
                    .context("provide a .zyal path or pass --all")?;
                let outcome = compile::compile_one(path, out.as_deref(), *check)?;
                match outcome {
                    compile::Outcome::Wrote(p) => {
                        println!("zyalc: wrote {}", p.display());
                        Ok(0)
                    }
                    compile::Outcome::Unchanged(p) => {
                        println!("zyalc: unchanged {}", p.display());
                        Ok(0)
                    }
                    compile::Outcome::Drift(p) => {
                        eprintln!("zyalc: drift detected in {}", p.display());
                        Ok(1)
                    }
                }
            }
        }
        Cmd::Inspect { path } => {
            let info = compile::inspect(path)?;
            println!("profile: {}", info.profile);
            if let Some(target) = info.target {
                println!("target:  {}", target.display());
            }
            if let Some(schema) = info.schema {
                println!("schema:  {schema}");
            }
            Ok(0)
        }
    }
}
