//! # OPTS OMSI keyword-block scanner (`opts-omsi-scan`)
//!
//! Shared text decoding and block parsing utilities for OMSI configuration formats.
//!
//! OMSI text configuration files (`.bus`, `paths.cfg`, `Money/*.cfg`, `.sco`, etc.)
//! use a common keyword-driven structure where keywords appear in square brackets
//! (`[keyword]`) followed by parameter lines.
//!
//! Split out of `opts-omsi-text` (ISS-087, stage 0). `opts-omsi-text` re-exports this crate as
//! `formats::text`, so the old paths keep working. Pure in-memory parsers: no file I/O, no engine,
//! no OMSI binaries (ADR-0002).

#![warn(missing_docs)]
#![deny(unsafe_code)]

mod blocks;
mod decode;
mod values;

pub use blocks::{parse_keyword, scan_blocks, scan_blocks_with, OmsiBlock};
pub use decode::decode_bytes;
pub use values::{parse_f64, parse_float, parse_vec3};

#[cfg(test)]
mod tests;
