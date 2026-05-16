//! Inventory tables for the Packet O cutover.
//!
//! These constants enumerate the relative paths the cutover acts on. Keep
//! alphabetical within each group for reviewability. The lists mirror
//! `docs/archive/historical/open-tui-bun-deletion-plan.md` — that document is
//! the source of truth: if you change the doc, change the lists here to
//! match.

pub(super) const ROOT_MANIFESTS: &[&str] = &[
    ".oxlintrc.json",
    ".prettierignore",
    "bun.lock",
    "bunfig.toml",
    "check-zyal.mjs",
    "package-lock.json",
    "package.json",
    "tsconfig.json",
    "turbo.json",
];

pub(super) const HUSKY_FILES: &[&str] = &[".husky/check-encrypted-paths", ".husky/pre-push"];

pub(super) const ROOT_SCRIPT_TS: &[&str] = &[
    // script/beta.ts deleted by Codex (xtask beta is now Rust-native).
    "script/changelog.ts",
    "script/duplicate-pr.ts",
    "script/format.ts",
    "script/generate.ts",
    "script/github/close-issues.ts",
    "script/memory-benchmark-seed-commit.ts",
    "script/publish.ts",
    "script/raw-changelog.ts",
    "script/stats.ts",
    "script/sync-zed.ts",
    "script/version.ts",
];

pub(super) const ROOT_SCRIPT_OTHER: &[&str] = &[
    "script/record-readme-demo.sh",
    "script/release",
    "script/sign-windows.ps1",
];

pub(super) const ROOT_SCRIPTS_MJS: &[&str] = &[
    "scripts/jankurai-dispatch-classifier.mjs",
    "scripts/persist-concept.mjs",
    "scripts/regression-sentinel.mjs",
];

pub(super) const TOOLS_JS: &[&str] = &["tools/jankurai-audit-gate.mjs"];

pub(super) const PATCHES: &[&str] = &[
    "patches/@npmcli%2Fagent@4.0.0.patch",
    "patches/@standard-community%2Fstandard-openapi@0.2.9.patch",
];

pub(super) const NIX_FILES: &[&str] = &[
    "flake.lock",
    "flake.nix",
    "nix/hashes.json",
    "nix/jekko.nix",
    "nix/node_modules.nix",
];

pub(super) const GITHUB_WORKFLOWS_BUN: &[&str] = &[
    ".github/publish-python-sdk.yml",
    ".github/workflows/beta.yml",
    ".github/workflows/duplicate-issues.yml",
    ".github/workflows/jekko.yml",
    ".github/workflows/pr-management.yml",
    ".github/workflows/publish.yml",
    ".github/workflows/review.yml",
    ".github/workflows/triage.yml",
];

pub(super) const PACKAGE_LOCAL_NOTES: &[&str] = &[
    // mini-fleet-smoke/package.json was deleted during the bun→cargo
    // migration; this entry stays only as historical context for the plan.
    "packages/jekko/BUN_SHELL_MIGRATION_PLAN.md",
];

pub(super) const PACKAGES_DIRS: &[&str] = &[
    "packages/containers",
    "packages/core",
    "packages/enterprise",
    "packages/function",
    "packages/jekko",
    "packages/plugin",
    "packages/script",
    "packages/sdk/js",
    "packages/slack",
    "packages/ux-qa",
];

pub(super) const BUILD_CACHES: &[&str] = &[
    ".jekko",
    ".turbo",
    "node_modules",
    "packages/core/.turbo",
    "packages/jekko/.18af-bun-build-glob",
    "packages/jekko/.jekko",
    "packages/jekko/.turbo",
    "packages/jekko/dist",
    "packages/jekko/node_modules",
    "packages/plugin/.turbo",
    "packages/plugin/dist",
    "packages/script/.turbo",
    "packages/sdk/js/.turbo",
    "packages/ux-qa/dist",
];

// `mini-fleet-smoke` used to be a Bun smoke crate; it has been migrated to a
// real Rust crate and is now a workspace member. Do NOT delete it.
pub(super) const MINI_FLEET: &[&str] = &[];

pub(super) const ROOT_TEST_DIRS: &[&str] = &["test"];

pub(super) const SETUP_BUN_ACTION: &[&str] = &[".github/actions/setup-bun"];

pub(super) const SPECS_DIRS: &[&str] = &["packages/jekko/specs/effect", "packages/jekko/specs/v2"];

/// Files whose contents are gated behind the remaining Rust agent work.
/// The executor updates these files rather than removing them outright.
pub(super) const EDIT_GATED: &[&str] = &[
    "ops/ci/beta.sh",
    "ops/ci/duplicate-issues.sh",
    "ops/ci/jekko.sh",
    "ops/ci/pr-management.sh",
    "ops/ci/publish-build-cli.sh",
    // ops/ci/publish-install-jekko.sh deleted by Codex (Rust-native install).
    "ops/ci/publish-version.sh",
    "ops/ci/publish.sh",
    "ops/ci/review.sh",
    "ops/ci/triage.sh",
];

/// Documentation files that need rewriting (Bun -> Rust) but are not
/// deletion candidates.
///
/// Removed (no scrub owed):
/// - docs/archive/historical/open-tui-bun-inventory.md (archived; reference only)
/// - docs/ci-local.md (already Rust-native; "no Node or Bun" statement)
/// - docs/testing-tui.md (already Ratatui/Crossterm-native)
pub(super) const EDIT_DOCS: &[&str] = &[
    "docs/ZYAL/CHANGELOG.md",
    "docs/ZYAL/SPEC.md",
    "docs/ZYAL/sandbox-loops.md",
    "docs/architecture.md",
    "docs/boundaries.md",
    "docs/install.md",
    "docs/testing.md",
];

/// Top-level repo docs that mention the JS runtime and need updating.
///
/// Removed (paper-trail / append-only logs or already Rust-current):
/// - BABYSIT_WORK.md (append-only coordination log)
/// - CHANGELOG.md (append-only release history)
/// - MEMORY_SYSTEM_LEVELUP.md (Codex handoff doc; append-only)
/// - README.md (already Rust-native; has Migration Notes footer)
/// - SANDBOX_WORKPLAN.md (workplan/audit doc; append-only)
/// - UNLOCK_WORKPLAN.md (append-only workplan with dated receipts)
/// - ZYAL_MISSION.md (carries historical banner)
/// - ZYAL_WORKFLOW.md (append-only workflow log)
pub(super) const EDIT_TOP_LEVEL: &[&str] = &["CONTRIBUTING.md", "Justfile"];
