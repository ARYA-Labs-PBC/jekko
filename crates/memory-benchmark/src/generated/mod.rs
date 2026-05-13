// This file is hand-maintained: the original generator regenerates the
// listed modules but new modules (compounding, hardening) live alongside
// without conflict.

pub mod compounding;
pub mod hardening;
pub mod math_world;
pub mod metamorphic;
pub mod names;
pub mod physics_world;
pub mod privacy_world;
pub mod procedural_world;
pub mod seed;
pub mod suite;
pub mod theorem_world;

pub use compounding::{generate_compounding_suite, CompoundingConfig};
pub use hardening::{generate_hardening_suite, HardeningConfig};
pub use suite::{generate_suite, GeneratedSuiteConfig};
