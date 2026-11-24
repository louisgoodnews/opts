//! # OPTS OMSI text readers (`opts-omsi-text`)
//!
//! The keyword-block scanner of OMSI 2's text formats and the parsers that need nothing else:
//! `constfile`, `varlist`, `envir`, `engine_config`, `cti`, `oft`, `org`, `money`, `humans`,
//! `ticketpack`, `weather`, `timetable` and `sound`. Split out of `opts-omsi` (ISS-034 budget
//! room; plan in `docs/dev/crate-split-omsi.md`). The paths are the same as before: `opts-omsi`
//! re-exports every module, so `opts_omsi::formats::text` and the other paths keep working.
//!
//! Pure in-memory parsers: no file I/O, no engine, no OMSI binaries (ADR-0002).

#![warn(missing_docs)]
#![deny(unsafe_code)]

pub mod formats;
