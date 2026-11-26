//! Text-format parsers of OMSI 2 (see the crate docs).

pub mod constfile;
pub mod cti;
pub mod engine_config;
pub mod envir;
pub mod humans;
pub mod money;
pub mod oft;
pub mod org;
pub mod sound;
pub use opts_omsi_scan as text;
pub mod ticketpack;
pub use opts_omsi_timetable as timetable;
pub mod varlist;
pub use opts_omsi_weather as weather;
