//! zyalc — `.zyal` source compiler.
//!
//! Three profiles, disambiguated by top-of-file pragma:
//! - Profile A — runbook (sentinel-wrapped strict YAML, written through as-is).
//! - Profile B — declarative (`# zyal: declarative target=toml ...`), emits TOML.
//! - Profile C — workflow (`# zyal: declarative target=github-workflow ...`), emits YAML.

pub mod compile;
pub mod profile;
