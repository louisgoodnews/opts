//! # OPTS OMSI text readers (`opts-omsi-text`)
//!
//! The parsers of OMSI 2's text formats that need nothing else: `constfile`, `varlist`, `envir`,
//! `engine_config`, `cti`, `oft`, `org`, `money`, `humans`, `ticketpack` and `sound`.
//! The keyword-block scanner (`text`, crate `opts-omsi-scan`), the timetable readers (`timetable`,
//! crate `opts-omsi-timetable`) and the weather readers (`weather`, crate `opts-omsi-weather`) are
//! split out and re-exported here (ISS-087). Split
//! out of `opts-omsi` (ISS-034 budget room; plan in `docs/dev/crate-split-omsi.md`). The paths are
//! the same as before: `opts-omsi` re-exports every module, so `opts_omsi::formats::text` and the
//! other paths keep working.
//!
//! Pure in-memory parsers: no file I/O, no engine, no OMSI binaries (ADR-0002).

#![warn(missing_docs)]
#![deny(unsafe_code)]

pub mod formats;
