//! The course of a timetable trip on the map (M-0105): the element index that places every
//! timetable ID on a tile.
//!
//! Design: `docs/src/architecture/minimap-path-recognition.md`. Pure: no Bevy, no I/O; the host
//! supplies parsed tiles.

mod index;

pub use index::{ElementCache, ElementIndex, ElementKind, ElementRef, Lookup, TileKey};
