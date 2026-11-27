//! # `opts-holidays`: the holiday calendar of a map
//!
//! The map folder's `Holidays.txt` (`docs/src/formats/map/lists.md`) lists public holidays
//! (`[holiday]`) and school holiday periods (`[holidays]`). [`Holidays::parse`] reads the decoded
//! text; [`Holidays::day_kind`] tells which day kind a date selects for the `[set_day_of_week]`
//! schedules. Dates are `yyyymmdd` numbers.
//!
//! The file is UTF-16LE in some maps: decode it first (`opts_omsi_text::decode_bytes`).

#![deny(unsafe_code)]
#![warn(missing_docs)]

mod calendar;
mod parse;

pub use calendar::{weekday, DayKind, Holidays, PublicHoliday, SchoolHoliday};
pub use parse::parse_date;
