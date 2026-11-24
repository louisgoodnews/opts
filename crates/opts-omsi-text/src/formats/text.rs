//! Shared text decoding and block parsing utilities for OMSI configuration formats.
//!
//! OMSI text configuration files (`.bus`, `paths.cfg`, `Money/*.cfg`, `.sco`, etc.)
//! use a common keyword-driven structure where keywords appear in square brackets
//! (`[keyword]`) followed by parameter lines.

mod blocks;
mod decode;
mod values;

pub use blocks::{parse_keyword, scan_blocks, scan_blocks_with, OmsiBlock};
pub use decode::decode_bytes;
pub use values::{parse_f64, parse_float, parse_vec3};

#[cfg(test)]
mod tests;
