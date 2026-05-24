//! Priority-based single-line status packer.

mod model;
mod packer;

#[cfg(test)]
mod tests;

pub use model::{PackOptions, Segment};
pub use packer::pack;
