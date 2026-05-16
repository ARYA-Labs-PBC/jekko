//! Schema primitives shared across v2 modules.
//!
//! Ported from `packages/jekko/src/v2/schema.ts`.
use serde::{Deserialize, Serialize};

/// UTC timestamp expressed as milliseconds since the Unix epoch.
///
/// The TypeScript codebase uses `DateTimeUtcFromMillis` to read epoch-millis
/// numbers into Effect's `DateTime.Utc`. The Rust port keeps the raw integer;
/// downstream code can convert to a richer type as needed (e.g. `time::OffsetDateTime`)
/// without dragging chrono into the core crate.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct UtcMillis(pub i64);

impl UtcMillis {
    /// Construct from a raw millisecond count.
    pub const fn new(value: i64) -> Self {
        Self(value)
    }

    /// Underlying millisecond count.
    pub const fn get(self) -> i64 {
        self.0
    }
}

impl From<i64> for UtcMillis {
    fn from(value: i64) -> Self {
        Self(value)
    }
}

impl From<UtcMillis> for i64 {
    fn from(value: UtcMillis) -> Self {
        value.0
    }
}
