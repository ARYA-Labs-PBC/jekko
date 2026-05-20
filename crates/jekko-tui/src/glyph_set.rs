//! ASCII fallback for Unicode glyphs (COWBOY.md T3-A3, tip2.txt §6.2).

use std::sync::OnceLock;

include!("glyph_set/table.rs");
include!("glyph_set/mode.rs");

#[cfg(test)]
include!("glyph_set/tests.rs");
