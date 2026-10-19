//! `opts-types`: the small set of types the legacy OMSI readers (`opts-omsi`) and the native model
//! (`opts-model`) both need, so neither depends on the other
//! ([ADR-0014](../../docs/src/adr/0014-data-model-separation.md)).
//!
//! Nothing here knows a file format: only geometry and coordinates. Today that is the tile grid
//! of a map ([`grid::TileGrid`]); ids and error kinds join it when the native model needs them.

#![warn(missing_docs)]
#![deny(unsafe_code)]

pub mod grid;

pub use grid::{mercator_row_width, BorderSpline, GridFit, TileGrid, DEFAULT_TILE_SIZE_M};
