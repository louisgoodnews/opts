//! # OPTS OMSI vehicle readers (`opts-omsi-vehicles`)
//!
//! The vehicle side of OMSI 2's content: `formats::vehicles` (bus manifests, `model.cfg` with its
//! animations and lights, `paths.cfg`, depot `.hof`, script textures), the `formats::scenery::light`
//! reader (shared with the model), and the `setvars` catalogue of each vehicle's equipment options.
//! Split out of `opts-omsi` (ISS-034 budget room; plan in `docs/dev/crate-split-omsi.md`). The paths
//! are the same as before: `opts-omsi` re-exports every module, so `opts_omsi::formats::vehicles`
//! and `opts_omsi::setvars` keep working.
//!
//! Pure in-memory parsers and pure data: no file I/O, no engine, no OMSI binaries (ADR-0002). The
//! setvar catalogue's curated data is `data/setvar_families.json`, read at compile time.

#![warn(missing_docs)]
#![deny(unsafe_code)]

pub mod formats;
pub mod setvars;
