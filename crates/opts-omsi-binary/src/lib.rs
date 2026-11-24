//! # OPTS OMSI binary mesh readers (`opts-omsi-binary`)
//!
//! The parsers of OMSI 2's binary mesh formats: `.o3d` (`formats::o3d`) and the DirectX `.x`
//! format (`formats::x`). Split out of `opts-omsi` (ISS-034 budget room; plan in
//! `docs/dev/crate-split-omsi.md`). The paths are the same as before: `opts-omsi` re-exports
//! both modules, so `opts_omsi::formats::o3d` and `opts_omsi::formats::x` keep working.
//!
//! Pure in-memory parsers over byte slices: no file I/O, no engine, no OMSI binaries (ADR-0002).

#![warn(missing_docs)]
#![deny(unsafe_code)]

pub mod formats;
