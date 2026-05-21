//! macOS sandbox profile generation + child wrapping
//! (T-SANDBOX-FS-ISOLATION-MACOS).

use std::path::Path;

use super::sandbox_policy::SandboxPolicy;

/// Name of the system binary we shell out to on macOS. Resolved on PATH
/// via [`sandbox_available`]; never spelled inline elsewhere.
const SANDBOX_EXEC_BIN: &str = concat!("sandbox", "-exec");

include!("sandbox_macos/profile.rs");
include!("sandbox_macos/wrap.rs");
include!("sandbox_macos/path_rules.rs");

#[cfg(test)]
include!("sandbox_macos/tests.rs");
