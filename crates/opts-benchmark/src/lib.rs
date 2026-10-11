//! OPTS benchmarks and corpus verification.
//!
//! The first tool is the scenery object load verification of M-0054
//! (`docs/src/architecture/scenery-benchmark.md`): it places every scenery object of an OMSI
//! installation on a generated one-tile map, renders it off screen and checks it against the
//! object's own files. It needs the default feature `scenery` (Bevy's renderer and the game's
//! loader). The Criterion suite of M-0028 will live in `benches/` and builds with
//! `--no-default-features`.

#![warn(missing_docs)]

#[cfg(feature = "scenery")]
pub mod logging;
#[cfg(feature = "scenery")]
pub mod scenery;
