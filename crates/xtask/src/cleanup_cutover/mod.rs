//! Packet O cutover helper.
//!
//! Powers `xtask cleanup-cutover [--execute]`. The Phase 15 deletion pass
//! that removes the live JS/TypeScript runtime surface from the repo once
//! the Rust port has reached parity.
//!
//! This module re-exports the public API of the cutover so existing callers
//! (`crate::cleanup_cutover::compute_plan`, `crate::cleanup_cutover::CutoverPlan`)
//! keep working unchanged. Implementation is split:
//!
//! - [`inventory`] — relative-path const arrays the plan iterates over.
//! - [`plan`] — `CutoverPlan` struct and `compute_plan(repo_root)`.
//! - [`tests`] — plan-level integration assertions (gated by `cfg(test)`).
//!
//! The plan returned here mirrors
//! `docs/archive/historical/open-tui-bun-deletion-plan.md`. The
//! documentation is the source of truth: if you change the doc, change the
//! lists below to match.

mod inventory;
mod plan;

#[cfg(test)]
mod tests;

pub use plan::compute_plan;
// `CutoverPlan` is part of the public surface for downstream tooling even
// though no in-tree caller currently names it.
#[allow(unused_imports)]
pub use plan::CutoverPlan;
