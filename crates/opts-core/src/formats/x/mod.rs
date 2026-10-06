//! Microsoft DirectX `.x` legacy mesh format parser.
//!
//! Cites specifications in:
//! - Format Specification: `docs/src/formats/binary/x.md`
//! - Coordinates & Math: `docs/src/architecture/coordinates.md`
//! - Mutation test observations: Addendum in `x.md` (tests T10–T12)
//!
//! This module provides an in-memory, safe Rust parser for DirectX `.x` text mesh files,
//! normalizing vertex buffers, triangulating quads with CCW front-facing winding,
//! resolving named material references, and preserving hierarchical frame transformations.

pub mod ast;
pub mod error;
pub mod header;
pub mod lexer;
pub mod parse;
pub mod triangulate;

pub use ast::{XFrame, XMaterial, XMesh, XSubMesh, XVertex};
pub use error::XParseError;
pub use header::{parse_header, XHeader};
pub use parse::parse_x;

#[cfg(test)]
mod tests;
