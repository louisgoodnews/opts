//! # OPTS OMSI scenery readers (`opts-omsi-scenery`)
//!
//! The scenery side of OMSI 2's content: `formats::scenery` (`.sco` scenery objects, `.sli`
//! spline profiles, crossings and their base meshes) and `formats::trains` (train consists, which
//! use the vehicle bus manifests). Split out of `opts-omsi` (ISS-034 budget room; plan in
//! `docs/dev/crate-split-omsi.md`). The paths are the same as before: `opts-omsi` re-exports both
//! modules, so `opts_omsi::formats::scenery` and `opts_omsi::formats::trains` keep working.
//!
//! Pure in-memory parsers over text and byte slices: no file I/O, no engine, no OMSI binaries
//! (ADR-0002).

#![warn(missing_docs)]
#![deny(unsafe_code)]

pub mod formats;
