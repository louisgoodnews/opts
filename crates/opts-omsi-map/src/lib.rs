//! # OPTS OMSI map readers (`opts-omsi-map`)
//!
//! The map side of OMSI 2's content: `formats::map` (the `global.cfg` manifest, `tile_*.map`
//! tiles, `.map.terrain` height grids, `.water`, OSN times, density lists, and the map writers) and
//! `formats::chrono` (the chronology overlay of the map's timetable entries). Split out of
//! `opts-omsi` (ISS-034 budget room; plan in `docs/dev/crate-split-omsi.md`). The paths are the same
//! as before: `opts-omsi` re-exports both modules, so `opts_omsi::formats::map` keeps working.
//!
//! Pure in-memory parsers and writers over byte slices and strings: no file I/O, no engine, no OMSI
//! binaries (ADR-0002).

#![warn(missing_docs)]
#![deny(unsafe_code)]

pub mod formats;
