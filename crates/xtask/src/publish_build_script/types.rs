use serde::Deserialize;

/// Local deserialize-only mirror of [`crate::publish_build_plan::PublishBuildPlan`].
///
/// `publish_build_plan.rs` only derives `Serialize` (its existing public
/// shape), so we deserialize the JSON output through our own struct rather
/// than widen the public type. The field layout must match the JSON contract
/// emitted by `xtask publish-build-plan`.
#[derive(Debug, Clone, Deserialize)]
pub(super) struct PublishBuildPlanJson {
    pub targets: Vec<PublishBuildTarget>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct PublishBuildTarget {
    pub os: String,
    pub arch: String,
    pub name: String,
    pub bun_target: String,
    #[serde(default)]
    pub abi: Option<String>,
    #[serde(default)]
    pub avx2: Option<bool>,
}

/// Resolved per-target build descriptor used by the orchestrator.
#[derive(Debug, Clone)]
pub(super) struct ResolvedTarget {
    pub plan: PublishBuildTarget,
    pub rust_triple: &'static str,
    pub baseline: bool,
    pub is_host: bool,
}

/// Outcome of attempting to build one target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum BuildOutcome {
    Built,
    Skipped(String),
}

/// Parsed CLI / `BUILD_ARGS` flags.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct Flags {
    pub single: bool,
    pub baseline: bool,
    #[allow(dead_code)] // accepted no-op flag from the publish wrapper
    pub skip_install: bool,
    #[allow(dead_code)]
    // accepted no-op flag; release profile already emits debug info
    pub sourcemaps: bool,
}
