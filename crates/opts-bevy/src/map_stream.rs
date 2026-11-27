//! Tile streaming off the frame thread (ISS-070, M-0024 phase B): reading and parsing a tile file
//! as an async chunk job, and the coordinator that queues, runs and collects those jobs.
//!
//! The tile build itself (entities, meshes, textures) stays on the frame thread, because it needs
//! the Bevy world. Only the file read and the parse move to `AsyncComputeTaskPool`. The design and
//! its limits are in `docs/src/architecture/map-streaming.md`.

mod coordinator;
mod parse;

pub use coordinator::{TileKey, TileParseCoordinator};
pub use parse::{parse_tile_bytes, parse_tile_file, TileParseError, TileParseResult};
